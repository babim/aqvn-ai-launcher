//! Upstream HTTP/WebSocket client for the bridge proxy.
//!
//! The proxy owns every credential the browser normally would: it bootstraps
//! CSRF (`GET /api/csrf_token`), performs the login form post, replays the
//! cookie jar, spoofs `Origin`/`Host` to the target server, and retries a
//! request exactly once after a CSRF-related 403.

use crate::proxy::jar::CookieJar;
use axum::http::{header, Method};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::Mutex;

const MAX_REDIRECTS: usize = 8;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
const CSRF_PATH: &str = "/api/csrf_token";
const LOGIN_PATH: &str = "/login";

/// Resolves the default cookie directory on the host.
pub fn cookie_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("AQVN_CONFIG_DIR") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.trim().is_empty() {
            return PathBuf::from(xdg)
                .join("net.aqvn.workspace")
                .join("cookies");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.trim().is_empty() {
            return PathBuf::from(home)
                .join(".config")
                .join("net.aqvn.workspace")
                .join("cookies");
        }
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        if !profile.trim().is_empty() {
            return PathBuf::from(profile)
                .join(".config")
                .join("net.aqvn.workspace")
                .join("cookies");
        }
    }
    std::env::temp_dir()
        .join("net.aqvn.workspace")
        .join("cookies")
}

/// Resolves the cookie storage file path for a target origin.
pub fn cookie_file_path(target_origin: &str) -> PathBuf {
    let origin = if let Ok(parsed) = url::Url::parse(target_origin) {
        if let Some(host) = parsed.host_str() {
            let authority = match parsed.port() {
                Some(port) => format!("{host}:{port}"),
                None => host.to_string(),
            };
            format!("{}://{authority}", parsed.scheme())
        } else {
            target_origin.to_string()
        }
    } else {
        target_origin.to_string()
    };
    let mut hasher = Sha256::new();
    hasher.update(origin.as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    let filename = format!("cookies_{}.json", &hash[..16]);
    cookie_dir().join(filename)
}

/// Parsed target server: scheme, authority, and optional base path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    base: String,
    scheme: String,
    authority: String,
    origin: String,
}

impl Target {
    pub fn parse(server_url: &str) -> Result<Self, String> {
        let url = url::Url::parse(server_url)
            .map_err(|err| format!("invalid server_url {server_url:?}: {err}"))?;
        let scheme = url.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(format!(
                "unsupported server_url scheme {scheme:?}: expected http or https"
            ));
        }
        let host = url
            .host_str()
            .ok_or_else(|| format!("server_url {server_url:?} has no host"))?;
        let authority = match url.port() {
            Some(port) => format!("{host}:{port}"),
            None => host.to_string(),
        };
        let base_path = url.path().trim_end_matches('/').to_string();
        let origin = format!("{scheme}://{authority}");
        Ok(Self {
            base: format!("{origin}{base_path}"),
            scheme: scheme.to_string(),
            authority,
            origin,
        })
    }

    /// Fully-qualified HTTP URL for a path-and-query string from the browser.
    pub fn url(&self, path_and_query: &str) -> String {
        let suffix = if path_and_query.starts_with('/') {
            path_and_query.to_string()
        } else {
            format!("/{path_and_query}")
        };
        format!("{}{suffix}", self.base)
    }

    /// Fully-qualified WebSocket URL for the same path-and-query string.
    pub fn ws_url(&self, path_and_query: &str) -> String {
        let scheme = if self.scheme == "https" { "wss" } else { "ws" };
        let suffix = if path_and_query.starts_with('/') {
            path_and_query.to_string()
        } else {
            format!("/{path_and_query}")
        };
        let base_path = self.base.strip_prefix(&self.origin).unwrap_or_default();
        format!("{scheme}://{}{base_path}{suffix}", self.authority)
    }

    pub fn origin(&self) -> &str {
        &self.origin
    }

    pub fn authority(&self) -> &str {
        &self.authority
    }

    pub fn same_origin(&self, url: &url::Url) -> bool {
        let authority = match url.port() {
            Some(port) => format!("{}:{port}", url.host_str().unwrap_or_default()),
            None => url.host_str().unwrap_or_default().to_string(),
        };
        url.scheme() == self.scheme && authority == self.authority
    }
}

/// Response forwarded back to the webview.
#[derive(Debug)]
pub struct ForwardResponse {
    pub status: u16,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
}

/// A proxied WebSocket stream (plain TCP or TLS depending on the target).
pub type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

#[derive(Debug)]
enum WsConnectError {
    Http(u16),
    Other(String),
}

impl std::fmt::Display for WsConnectError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(status) => write!(
                formatter,
                "upstream websocket handshake returned HTTP {status}"
            ),
            Self::Other(message) => {
                write!(formatter, "upstream websocket connect failed: {message}")
            }
        }
    }
}

struct HttpOutcome {
    status: u16,
    content_type: Option<String>,
    body: Vec<u8>,
    final_path: String,
}

/// Upstream client plus the single-user cookie jar.
pub struct Upstream {
    client: reqwest::Client,
    target: Target,
    jar: Mutex<CookieJar>,
    cookie_path: Option<PathBuf>,
}

impl Upstream {
    pub fn new(server_url: &str) -> Result<Self, String> {
        let target = Target::parse(server_url)?;
        // Redirects are followed manually so every hop's `Set-Cookie` lands in
        // the jar (the login response sets the session on a 302).
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|err| format!("failed to build upstream HTTP client: {err}"))?;
        let cookie_path = cookie_file_path(target.origin());
        let jar = if cookie_path.exists() {
            CookieJar::load_from_file(&cookie_path).unwrap_or_default()
        } else {
            CookieJar::new()
        };
        Ok(Self {
            client,
            target,
            jar: Mutex::new(jar),
            cookie_path: Some(cookie_path),
        })
    }

    pub fn cookie_path(&self) -> Option<&Path> {
        self.cookie_path.as_deref()
    }

    fn persist_jar(&self, jar: &CookieJar) {
        if let Some(path) = &self.cookie_path {
            if let Err(err) = jar.save_to_file(path) {
                eprintln!(
                    "Bridge proxy: failed to persist cookies to {}: {err}",
                    path.display()
                );
            }
        }
    }

    pub fn target(&self) -> &Target {
        &self.target
    }

    /// Current CSRF token and runtime id (bootstrapping on first use).
    pub async fn csrf(&self) -> Result<(String, String), String> {
        let mut jar = self.jar.lock().await;
        if jar.is_bootstrapped() {
            let runtime_id = jar.runtime_id().unwrap_or_default().to_string();
            let token = jar.csrf_token().unwrap_or_default().to_string();
            return Ok((runtime_id, token));
        }
        self.bootstrap_locked(&mut jar, true)
            .await?
            .ok_or_else(|| "CSRF bootstrap did not return a token".to_string())
    }

    /// Bootstraps CSRF unless the jar already holds a token for the runtime.
    async fn ensure_bootstrapped(&self) -> Result<(), String> {
        let mut jar = self.jar.lock().await;
        if jar.is_bootstrapped() {
            return Ok(());
        }
        // Before login the target redirects `/api/csrf_token` to `/login`
        // (the endpoint requires a session): expected, not an error here.
        self.bootstrap_locked(&mut jar, false).await.map(|_| ())
    }

    /// `GET /api/csrf_token`; stores the session cookie and the client-side
    /// CSRF cookie the server expects on the next handshake.
    ///
    /// With `required = false` an auth redirect (3xx) or 401/403 is treated as
    /// "not logged in yet" and returns `Ok(None)`; authenticated callers pass
    /// `required = true` so a failed refresh is surfaced.
    async fn bootstrap_locked(
        &self,
        jar: &mut CookieJar,
        required: bool,
    ) -> Result<Option<(String, String)>, String> {
        let url = self.target.url(CSRF_PATH);
        let mut request = self
            .client
            .get(&url)
            .header(header::ORIGIN, self.target.origin())
            .header(header::ACCEPT, "application/json");
        if let Some(cookie) = jar.cookie_header() {
            request = request.header(header::COOKIE, cookie);
        }
        let response = request
            .send()
            .await
            .map_err(|err| format!("CSRF bootstrap request failed: {err}"))?;
        let status = response.status();
        let set_cookies = set_cookie_values(response.headers());
        let names = jar.store_all(set_cookies.iter().map(String::as_str));
        let rotation = names.iter().any(|name| jar.observes_rotation(name));
        let body = response
            .text()
            .await
            .map_err(|err| format!("CSRF bootstrap response unreadable: {err}"))?;
        if !status.is_success() {
            if !required
                && (status.is_redirection()
                    || status == axum::http::StatusCode::UNAUTHORIZED
                    || status == axum::http::StatusCode::FORBIDDEN)
            {
                if !names.is_empty() {
                    self.persist_jar(jar);
                }
                println!(
                    "Bridge proxy: CSRF bootstrap deferred (HTTP {}); login required",
                    status.as_u16()
                );
                return Ok(None);
            }
            return Err(format!(
                "CSRF bootstrap returned HTTP {}: {}",
                status.as_u16(),
                truncate(&body)
            ));
        }
        let value: serde_json::Value = serde_json::from_str(&body)
            .map_err(|err| format!("CSRF bootstrap response is not JSON: {err}"))?;
        let token = value
            .get("token")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "CSRF bootstrap response missing `token`".to_string())?;
        let runtime_id = value
            .get("runtime_id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "CSRF bootstrap response missing `runtime_id`".to_string())?;

        let previous = jar.runtime_id().map(str::to_string);
        jar.apply_bootstrap(runtime_id, token);
        self.persist_jar(jar);
        match previous {
            Some(old) if old != runtime_id => println!(
                "Bridge proxy: runtime_id rotated {old} -> {runtime_id}; cookie jar re-bootstrapped"
            ),
            Some(_) => {}
            None if rotation => {
                println!("Bridge proxy: new runtime_id {runtime_id}; cookie jar re-bootstrapped")
            }
            None => println!("Bridge proxy: CSRF bootstrapped (runtime_id {runtime_id})"),
        }
        Ok(Some((runtime_id.to_string(), token.to_string())))
    }

    /// Performs the UI's login on its behalf: form-post, follow the redirect,
    /// then re-bootstrap CSRF so the token is bound to the new session.
    pub async fn login(&self, username: &str, password: &str) -> Result<(), String> {
        self.ensure_bootstrapped().await?;
        let form = form_encode(&[("username", username), ("password", password)]);
        let outcome = {
            let mut jar = self.jar.lock().await;
            self.send_locked(
                &mut jar,
                Method::POST,
                LOGIN_PATH,
                Some("application/x-www-form-urlencoded"),
                Some(form.into_bytes()),
                None,
            )
            .await?
        };

        if !(200..300).contains(&outcome.status) || outcome.final_path == LOGIN_PATH {
            return Err(format!(
                "login rejected (HTTP {}, final path {})",
                outcome.status,
                if outcome.final_path.is_empty() {
                    "<unknown>"
                } else {
                    &outcome.final_path
                }
            ));
        }

        let mut jar = self.jar.lock().await;
        self.bootstrap_locked(&mut jar, true).await?;
        self.persist_jar(&jar);
        Ok(())
    }

    /// Forwards an `/api/*` request with the CSRF header.
    ///
    /// A 403 triggers exactly one CSRF refresh and one retry; a second 403 is
    /// returned to the caller unchanged so the UI can surface it.
    pub async fn forward(
        &self,
        method: Method,
        path_and_query: &str,
        content_type: Option<&str>,
        body: Vec<u8>,
    ) -> Result<ForwardResponse, String> {
        self.ensure_bootstrapped().await?;
        let first = {
            let mut jar = self.jar.lock().await;
            let token = jar.csrf_token().map(str::to_string).unwrap_or_default();
            self.send_locked(
                &mut jar,
                method.clone(),
                path_and_query,
                content_type,
                Some(body.clone()),
                Some(&token),
            )
            .await?
        };

        if first.status == 403 {
            println!("Bridge proxy: upstream 403, refreshing CSRF token once");
            {
                let mut jar = self.jar.lock().await;
                self.bootstrap_locked(&mut jar, true).await?;
            }
            let mut jar = self.jar.lock().await;
            let token = jar.csrf_token().map(str::to_string).unwrap_or_default();
            let retried = self
                .send_locked(
                    &mut jar,
                    method,
                    path_and_query,
                    content_type,
                    Some(body),
                    Some(&token),
                )
                .await?;
            return Ok(forward_response(retried));
        }

        Ok(forward_response(first))
    }

    /// Forwarding for non-REST paths (Engine.IO polling fallback).
    pub async fn forward_raw(
        &self,
        method: Method,
        path_and_query: &str,
        content_type: Option<&str>,
        body: Vec<u8>,
    ) -> Result<ForwardResponse, String> {
        let mut jar = self.jar.lock().await;
        let outcome = self
            .send_locked(
                &mut jar,
                method,
                path_and_query,
                content_type,
                Some(body),
                None,
            )
            .await?;
        Ok(forward_response(outcome))
    }

    /// Opens the second leg of the WebSocket proxy.
    ///
    /// A 403 handshake (stale CSRF after a server restart) refreshes the token
    /// once and retries, mirroring the HTTP path.
    pub async fn connect_ws(&self, path_and_query: &str) -> Result<WsStream, String> {
        self.ensure_bootstrapped().await?;
        match self.ws_try(path_and_query).await {
            Ok(stream) => Ok(stream),
            Err(WsConnectError::Http(403)) => {
                println!(
                    "Bridge proxy: upstream websocket rejected (403), refreshing CSRF token once"
                );
                {
                    let mut jar = self.jar.lock().await;
                    self.bootstrap_locked(&mut jar, true).await?;
                }
                self.ws_try(path_and_query)
                    .await
                    .map_err(|err| err.to_string())
            }
            Err(err) => Err(err.to_string()),
        }
    }

    async fn ws_try(&self, path_and_query: &str) -> Result<WsStream, WsConnectError> {
        use axum::http::HeaderValue;
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;

        let url = self.target.ws_url(path_and_query);
        // Build the handshake through tungstenite (it generates the required
        // Sec-WebSocket-Key/Version headers), then spoof origin/host/cookie.
        let mut request = url
            .as_str()
            .into_client_request()
            .map_err(|err| WsConnectError::Other(err.to_string()))?;
        {
            let jar = self.jar.lock().await;
            let headers = request.headers_mut();
            headers.insert(
                header::HOST,
                HeaderValue::from_str(self.target.authority())
                    .map_err(|err| WsConnectError::Other(err.to_string()))?,
            );
            headers.insert(
                header::ORIGIN,
                HeaderValue::from_str(self.target.origin())
                    .map_err(|err| WsConnectError::Other(err.to_string()))?,
            );
            headers.insert(
                header::USER_AGENT,
                HeaderValue::from_static("AQVN-Workspace"),
            );
            if let Some(cookie) = jar.cookie_header() {
                headers.insert(
                    header::COOKIE,
                    HeaderValue::from_str(&cookie)
                        .map_err(|err| WsConnectError::Other(err.to_string()))?,
                );
            }
        }

        match tokio_tungstenite::connect_async(request).await {
            Ok((stream, _response)) => Ok(stream),
            Err(tokio_tungstenite::tungstenite::Error::Http(response)) => {
                Err(WsConnectError::Http(response.status().as_u16()))
            }
            Err(err) => Err(WsConnectError::Other(err.to_string())),
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn send_locked(
        &self,
        jar: &mut CookieJar,
        method: Method,
        path_and_query: &str,
        content_type: Option<&str>,
        body: Option<Vec<u8>>,
        csrf_token: Option<&str>,
    ) -> Result<HttpOutcome, String> {
        let mut url = self.target.url(path_and_query);
        let mut method = method;
        let mut body = body;
        let mut content_type = content_type.map(str::to_string);

        for _ in 0..=MAX_REDIRECTS {
            let mut request = self
                .client
                .request(method.clone(), &url)
                .header(header::ORIGIN, self.target.origin());
            if let Some(cookie) = jar.cookie_header() {
                request = request.header(header::COOKIE, cookie);
            }
            if let Some(token) = csrf_token {
                request = request.header("X-CSRF-Token", token);
            }
            if let Some(kind) = content_type.as_deref() {
                request = request.header(header::CONTENT_TYPE, kind);
            }
            if let Some(bytes) = body.as_ref() {
                request = request.body(bytes.clone());
            }

            let response = request
                .send()
                .await
                .map_err(|err| format!("upstream request to {url} failed: {err}"))?;
            let status = response.status().as_u16();
            let headers = response.headers().clone();
            let set_cookies = set_cookie_values(&headers);
            let names = jar.store_all(set_cookies.iter().map(String::as_str));
            if !names.is_empty() {
                self.persist_jar(jar);
            }

            if matches!(status, 301 | 302 | 303 | 307 | 308) {
                if let Some(location) = headers
                    .get(header::LOCATION)
                    .and_then(|value| value.to_str().ok())
                {
                    let Ok(base) = url::Url::parse(&url) else {
                        return Err(format!("upstream URL unparsable: {url}"));
                    };
                    let Ok(next) = base.join(location) else {
                        return Err(format!("redirect location unparsable: {location}"));
                    };
                    if !self.target.same_origin(&next) {
                        // Never follow off the configured server: that would
                        // leak the jar's cookies to a third party.
                        break;
                    }
                    if !matches!(status, 307 | 308)
                        && method != Method::GET
                        && method != Method::HEAD
                    {
                        method = Method::GET;
                        body = None;
                        content_type = None;
                    }
                    url = next.to_string();
                    continue;
                }
            }

            let final_path = url::Url::parse(&url)
                .map(|parsed| parsed.path().to_string())
                .unwrap_or_default();
            let content_type = headers
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string);
            let bytes = response
                .bytes()
                .await
                .map_err(|err| format!("upstream response body unreadable: {err}"))?;
            return Ok(HttpOutcome {
                status,
                content_type,
                body: bytes.to_vec(),
                final_path,
            });
        }

        // Only reached when a redirect pointed off-origin: surface it as-is.
        Err("upstream redirected off the configured server; refused to follow".to_string())
    }
}

fn forward_response(outcome: HttpOutcome) -> ForwardResponse {
    ForwardResponse {
        status: outcome.status,
        content_type: outcome.content_type,
        body: outcome.body,
    }
}

fn set_cookie_values(headers: &axum::http::HeaderMap) -> Vec<String> {
    headers
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok().map(str::to_string))
        .collect()
}

fn form_encode(pairs: &[(&str, &str)]) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in pairs {
        serializer.append_pair(key, value);
    }
    serializer.finish()
}

fn truncate(body: &str) -> String {
    const LIMIT: usize = 200;
    let trimmed = body.trim();
    if trimmed.chars().count() <= LIMIT {
        trimmed.to_string()
    } else {
        let shortened: String = trimmed.chars().take(LIMIT).collect();
        format!("{shortened}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_https_target() {
        let target = Target::parse("https://chat.example.com").expect("parse");

        assert_eq!(target.origin(), "https://chat.example.com");
        assert_eq!(target.authority(), "chat.example.com");
        assert_eq!(
            target.url("/api/csrf_token"),
            "https://chat.example.com/api/csrf_token"
        );
        assert_eq!(
            target.ws_url("/socket.io/?EIO=4&transport=websocket"),
            "wss://chat.example.com/socket.io/?EIO=4&transport=websocket"
        );
    }

    #[test]
    fn keeps_explicit_port_and_http_scheme() {
        let target = Target::parse("http://127.0.0.1:5000/").expect("parse");

        assert_eq!(target.origin(), "http://127.0.0.1:5000");
        assert_eq!(target.authority(), "127.0.0.1:5000");
        assert_eq!(
            target.ws_url("/socket.io/"),
            "ws://127.0.0.1:5000/socket.io/"
        );
    }

    #[test]
    fn preserves_base_path() {
        let target = Target::parse("https://example.com/a0/").expect("parse");
        assert_eq!(target.url("/api/poll"), "https://example.com/a0/api/poll");
        assert_eq!(
            target.ws_url("/socket.io/"),
            "wss://example.com/a0/socket.io/"
        );
    }

    #[test]
    fn rejects_unsupported_schemes() {
        assert!(Target::parse("ftp://example.com").is_err());
        assert!(Target::parse("not a url").is_err());
    }

    #[test]
    fn compares_origins() {
        let target = Target::parse("https://example.com").expect("parse");
        let same = url::Url::parse("https://example.com/next").expect("url");
        let other = url::Url::parse("https://elsewhere.example/next").expect("url");
        assert!(target.same_origin(&same));
        assert!(!target.same_origin(&other));
    }

    #[test]
    fn encodes_login_form() {
        assert_eq!(
            form_encode(&[("username", "admin"), ("password", "p@ss word&x")]),
            "username=admin&password=p%40ss+word%26x"
        );
    }

    #[test]
    fn resolves_cookie_file_path() {
        let p1 = cookie_file_path("http://127.0.0.1:8080");
        let p2 = cookie_file_path("http://127.0.0.1:8080/some/path");
        let p3 = cookie_file_path("http://127.0.0.1:9090");
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
        assert!(p1.to_string_lossy().ends_with(".json"));
    }
}
