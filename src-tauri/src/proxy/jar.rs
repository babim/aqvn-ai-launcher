//! Rust-owned cookie jar for the bridge proxy.
//!
//! The webview never stores credentials or cookies: the proxy keeps every
//! `Set-Cookie` value in this jar and replays them upstream. Two families are
//! runtime-scoped by the chat server:
//!
//! * `session_<runtime_id>` — authentication session
//! * `csrf_token_<runtime_id>` — CSRF token the browser normally sets itself
//!
//! `runtime_id` changes when the server restarts. Cookies from the previous
//! generation are dead, so a new bootstrap drops them before storing the
//! replacement pair.

use std::collections::BTreeMap;

const SESSION_PREFIX: &str = "session_";
const CSRF_PREFIX: &str = "csrf_token_";

/// Cookie jar scoped to one upstream chat server.
#[derive(Debug, Default, Clone)]
pub struct CookieJar {
    cookies: BTreeMap<String, String>,
    runtime_id: Option<String>,
    csrf_value: Option<String>,
}

impl CookieJar {
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores a single `Set-Cookie` header value.
    ///
    /// Returns the cookie name. Attributes (`Path`, `SameSite`, `Max-Age`, …)
    /// are ignored: this jar only lives as long as the process. An empty value
    /// is treated as a deletion, matching browser behaviour.
    pub fn store(&mut self, set_cookie: &str) -> Option<String> {
        let pair = set_cookie.split(';').next()?.trim();
        let (name, value) = pair.split_once('=')?;
        let name = name.trim();
        let value = value.trim();
        if name.is_empty() {
            return None;
        }
        if value.is_empty() {
            self.cookies.remove(name);
        } else {
            self.cookies.insert(name.to_string(), value.to_string());
        }
        Some(name.to_string())
    }

    /// Stores every header value and returns the cookie names seen.
    pub fn store_all<'a>(&mut self, values: impl IntoIterator<Item = &'a str>) -> Vec<String> {
        values
            .into_iter()
            .filter_map(|value| self.store(value))
            .collect()
    }

    /// Builds the upstream `Cookie` header, or `None` when the jar is empty.
    pub fn cookie_header(&self) -> Option<String> {
        if self.cookies.is_empty() {
            return None;
        }
        Some(
            self.cookies
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect::<Vec<_>>()
                .join("; "),
        )
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.cookies.get(name).map(String::as_str)
    }

    pub fn csrf_token(&self) -> Option<&str> {
        self.csrf_value.as_deref()
    }

    pub fn runtime_id(&self) -> Option<&str> {
        self.runtime_id.as_deref()
    }

    pub fn session_cookie_name(&self) -> Option<&str> {
        self.cookies
            .keys()
            .find(|name| name.starts_with(SESSION_PREFIX))
            .map(String::as_str)
    }

    /// Records a CSRF bootstrap response.
    ///
    /// Cookies bound to a previous runtime generation are dropped; the
    /// `session_<new_runtime>` cookie set by this same response is kept.
    pub fn apply_bootstrap(&mut self, runtime_id: &str, token: &str) {
        self.cookies
            .retain(|name, _| match runtime_scoped_id(name) {
                Some(id) => id == runtime_id,
                None => true,
            });
        self.cookies
            .insert(format!("{CSRF_PREFIX}{runtime_id}"), token.to_string());
        self.runtime_id = Some(runtime_id.to_string());
        self.csrf_value = Some(token.to_string());
    }

    /// True when the stored CSRF cookie matches the recorded value.
    pub fn is_bootstrapped(&self) -> bool {
        match (self.runtime_id.as_deref(), self.csrf_value.as_deref()) {
            (Some(runtime_id), Some(value)) => {
                self.get(&format!("{CSRF_PREFIX}{runtime_id}")) == Some(value)
            }
            _ => false,
        }
    }

    /// True when `name` belongs to a runtime generation newer than the recorded
    /// one — a rotation signal observed in a `Set-Cookie` header.
    pub fn observes_rotation(&self, name: &str) -> bool {
        let Some(runtime_id) = runtime_scoped_id(name) else {
            return false;
        };
        self.runtime_id
            .as_deref()
            .is_some_and(|current| current != runtime_id)
    }

    pub fn invalidate(&mut self) {
        self.cookies.clear();
        self.runtime_id = None;
        self.csrf_value = None;
    }
}

fn runtime_scoped_id(name: &str) -> Option<&str> {
    name.strip_prefix(SESSION_PREFIX)
        .or_else(|| name.strip_prefix(CSRF_PREFIX))
        .filter(|id| !id.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_set_cookie_and_ignores_attributes() {
        let mut jar = CookieJar::new();
        let name = jar.store("session_abcd1234=opaque; Path=/; SameSite=Lax; HttpOnly");

        assert_eq!(name.as_deref(), Some("session_abcd1234"));
        assert_eq!(jar.get("session_abcd1234"), Some("opaque"));
        assert_eq!(
            jar.cookie_header().as_deref(),
            Some("session_abcd1234=opaque")
        );
    }

    #[test]
    fn empty_value_deletes_cookie() {
        let mut jar = CookieJar::new();
        jar.store("session_abcd1234=opaque");
        jar.store("session_abcd1234=; Max-Age=0");

        assert_eq!(jar.cookie_header(), None);
    }

    #[test]
    fn bootstrap_stores_runtime_scoped_csrf_cookie() {
        let mut jar = CookieJar::new();
        jar.apply_bootstrap("abcd1234", "token-1");

        assert_eq!(jar.runtime_id(), Some("abcd1234"));
        assert_eq!(jar.csrf_token(), Some("token-1"));
        assert_eq!(jar.get("csrf_token_abcd1234"), Some("token-1"));
        assert!(jar.is_bootstrapped());
    }

    #[test]
    fn rotation_drops_previous_generation_cookies() {
        let mut jar = CookieJar::new();
        jar.store("session_old00000=stale");
        jar.apply_bootstrap("old00000", "old-token");
        jar.store("session_old00000=stale");
        jar.store("session_new11111=fresh");

        jar.apply_bootstrap("new11111", "new-token");

        assert_eq!(jar.runtime_id(), Some("new11111"));
        assert_eq!(jar.csrf_token(), Some("new-token"));
        assert_eq!(
            jar.session_cookie_name(),
            Some("session_new11111"),
            "old generation session dropped, new one kept"
        );
        assert_eq!(jar.get("csrf_token_old00000"), None);
        assert_eq!(jar.get("csrf_token_new11111"), Some("new-token"));
    }

    #[test]
    fn observes_rotation_only_for_new_generations() {
        let mut jar = CookieJar::new();
        jar.apply_bootstrap("abcd1234", "token-1");

        assert!(!jar.observes_rotation("session_abcd1234"));
        assert!(!jar.observes_rotation("csrf_token_abcd1234"));
        assert!(jar.observes_rotation("session_ffff9999"));
        assert!(jar.observes_rotation("csrf_token_ffff9999"));
        assert!(!jar.observes_rotation("other_cookie"));
        assert!(!jar.observes_rotation("session_"));
    }

    #[test]
    fn invalidate_clears_everything() {
        let mut jar = CookieJar::new();
        jar.store("session_abcd1234=opaque");
        jar.apply_bootstrap("abcd1234", "token-1");

        jar.invalidate();

        assert_eq!(jar.cookie_header(), None);
        assert_eq!(jar.runtime_id(), None);
        assert_eq!(jar.csrf_token(), None);
        assert!(!jar.is_bootstrapped());
    }
}
