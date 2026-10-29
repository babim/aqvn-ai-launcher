//! Rust-owned cookie jar for the bridge proxy.
//!
//! The webview never stores credentials or cookies: the proxy keeps every
//! `Set-Cookie` value in this jar and replays them upstream.
//!
//! * `session_<hash>` — authentication session
//! * `csrf_token_<runtime_id>` — CSRF token the browser normally sets itself
//!
//! `runtime_id` changes when the server restarts. CSRF tokens from the previous
//! generation are dead, so a new bootstrap drops them before storing the
//! replacement pair.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

const SESSION_PREFIX: &str = "session_";
const CSRF_PREFIX: &str = "csrf_token_";

/// Cookie jar scoped to one upstream chat server.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CookieJar {
    #[serde(default)]
    cookies: BTreeMap<String, String>,
    #[serde(default)]
    runtime_id: Option<String>,
    #[serde(default)]
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

    /// Loads a persisted cookie jar from disk JSON.
    pub fn load_from_file(path: &Path) -> Option<Self> {
        let bytes = std::fs::read(path).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    /// Saves the cookie jar to disk JSON. Parent directories are created if missing.
    pub fn save_to_file(&self, path: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let temp_path = path.with_extension(format!("tmp.{}", std::process::id()));
        if std::fs::write(&temp_path, &json)
            .and_then(|_| std::fs::rename(&temp_path, path))
            .is_err()
        {
            std::fs::write(path, &json)?;
            let _ = std::fs::remove_file(&temp_path);
        }
        Ok(())
    }

    /// Removes the persisted cookie jar file from disk if it exists.
    pub fn remove_file(path: &Path) {
        let _ = std::fs::remove_file(path);
    }
}

fn runtime_scoped_id(name: &str) -> Option<&str> {
    name.strip_prefix(CSRF_PREFIX).filter(|id| !id.is_empty())
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
        jar.store("session_db7dddd06e57fbec1602e03fe7473796=active-session");
        jar.apply_bootstrap("old00000", "old-token");
        assert_eq!(
            jar.get("session_db7dddd06e57fbec1602e03fe7473796"),
            Some("active-session")
        );

        jar.apply_bootstrap("763a48897ec809f5", "new-token");

        assert_eq!(jar.runtime_id(), Some("763a48897ec809f5"));
        assert_eq!(jar.csrf_token(), Some("new-token"));
        assert_eq!(
            jar.session_cookie_name(),
            Some("session_db7dddd06e57fbec1602e03fe7473796"),
            "session cookie preserved across bootstrap"
        );
        assert_eq!(
            jar.get("session_db7dddd06e57fbec1602e03fe7473796"),
            Some("active-session")
        );
        assert_eq!(jar.get("csrf_token_old00000"), None);
        assert_eq!(jar.get("csrf_token_763a48897ec809f5"), Some("new-token"));
    }

    #[test]
    fn observes_rotation_only_for_new_generations() {
        let mut jar = CookieJar::new();
        jar.apply_bootstrap("abcd1234", "token-1");

        assert!(!jar.observes_rotation("session_abcd1234"));
        assert!(!jar.observes_rotation("session_db7dddd06e57fbec1602e03fe7473796"));
        assert!(!jar.observes_rotation("session_ffff9999"));
        assert!(!jar.observes_rotation("csrf_token_abcd1234"));
        assert!(jar.observes_rotation("csrf_token_ffff9999"));
        assert!(!jar.observes_rotation("other_cookie"));
        assert!(!jar.observes_rotation("session_"));
        assert!(!jar.observes_rotation("csrf_token_"));
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

    #[test]
    fn save_and_load_from_file() {
        let mut jar = CookieJar::new();
        jar.store("session_abcd1234=opaque; Path=/");
        jar.apply_bootstrap("abcd1234", "token-1");

        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!(
            "aqvn-test-jar-{}-{}",
            std::process::id(),
            unique_id
        ));
        let file_path = temp_dir.join("cookies.json");

        assert!(jar.save_to_file(&file_path).is_ok());
        assert!(file_path.exists());

        let loaded = CookieJar::load_from_file(&file_path).expect("load saved jar");
        assert_eq!(loaded.get("session_abcd1234"), Some("opaque"));
        assert_eq!(loaded.runtime_id(), Some("abcd1234"));
        assert_eq!(loaded.csrf_token(), Some("token-1"));
        assert_eq!(loaded.cookie_header(), jar.cookie_header());
        assert!(loaded.is_bootstrapped());

        CookieJar::remove_file(&file_path);
        assert!(!file_path.exists());
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn load_from_nonexistent_or_invalid_file() {
        let nonexistent = Path::new("/tmp/nonexistent_aqvn_cookies_path_12345.json");
        assert!(CookieJar::load_from_file(nonexistent).is_none());

        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!(
            "aqvn-test-jar-invalid-{}-{}",
            std::process::id(),
            unique_id
        ));
        let file_path = temp_dir.join("invalid.json");
        let _ = std::fs::create_dir_all(&temp_dir);
        let _ = std::fs::write(&file_path, "not-valid-json-content");
        assert!(CookieJar::load_from_file(&file_path).is_none());

        CookieJar::remove_file(&file_path);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
