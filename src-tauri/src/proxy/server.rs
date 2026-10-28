//! Local bridge proxy: a loopback HTTP/WebSocket endpoint the webview can use
//! as if it were the chat server's own origin.
//!
//! It exists because the webview origin (`tauri://localhost`) cannot talk to
//! the chat server directly: the server rejects foreign WebSocket origins,
//! emits no CORS headers, and binds `SameSite=Lax` cookies the browser would
//! not send cross-site. The proxy holds the session, spoofs `Origin`/`Host`,
//! adds CORS for the webview, and tunnels `/socket.io/` unchanged.
//!
//! Bind address is loopback only and the port is chosen by the kernel; the
//! actual port is injected into the webview through
//! [`init_script`] as `window.__AQVN_PROXY__`.

use crate::config::ConfigWatcher;
use crate::proxy::upstream::{ForwardResponse, Upstream};
use crate::proxy::ws;
use axum::body::Body;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{FromRequestParts, State};
use axum::http::{
    header::{self, HeaderMap, HeaderValue},
    Method, Request, StatusCode, Uri,
};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;

/// Loopback only: this listener is never reachable from the network.
const BIND_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
const MAX_BODY_BYTES: usize = 32 * 1024 * 1024;
const LOGIN_PATH: &str = "/login";

/// Published to the webview at window creation time.
#[derive(Clone, Debug)]
pub struct ProxyInfo {
    pub port: u16,
    pub url: String,
}

impl ProxyInfo {
    fn from_addr(addr: SocketAddr) -> Self {
        Self {
            port: addr.port(),
            url: format!("http://{addr}"),
        }
    }
}

/// Shared, swappable upstream (retargeted when `server_url` changes).
#[derive(Clone)]
pub struct ProxyState {
    upstream: Arc<RwLock<Arc<Upstream>>>,
}

impl ProxyState {
    pub fn new(upstream: Upstream) -> Self {
        Self {
            upstream: Arc::new(RwLock::new(Arc::new(upstream))),
        }
    }

    pub async fn upstream(&self) -> Arc<Upstream> {
        self.upstream.read().await.clone()
    }

    pub async fn replace(&self, upstream: Upstream) {
        *self.upstream.write().await = Arc::new(upstream);
    }
}

/// Binds `127.0.0.1:0` and prints the resolved port.
pub async fn bind(server_url: &str) -> Result<(TcpListener, ProxyState, ProxyInfo), String> {
    let upstream = Upstream::new(server_url)?;
    let address = SocketAddr::new(BIND_ADDRESS, 0);
    let listener = TcpListener::bind(address)
        .await
        .map_err(|err| format!("bridge proxy failed to bind {address}: {err}"))?;
    let local = listener
        .local_addr()
        .map_err(|err| format!("bridge proxy has no local address: {err}"))?;
    println!(
        "Bridge proxy listening on http://{local} (target {})",
        upstream.target().origin()
    );
    Ok((
        listener,
        ProxyState::new(upstream),
        ProxyInfo::from_addr(local),
    ))
}

/// Serves until the process exits.
pub async fn serve(listener: TcpListener, state: ProxyState) -> std::io::Result<()> {
    axum::serve(listener, router(state)).await
}

/// Script injected into the webview before any page script runs.
pub fn init_script(port: u16) -> String {
    format!(
        "window.__AQVN_PROXY__ = Object.freeze({{ url: \"http://127.0.0.1:{port}\", port: {port} }}); window.__AQVN_WINDOW_SHOWN__ = performance.now();"
    )
}

/// Keeps the proxy pointed at the configured server; a change resets the jar.
pub async fn watch_target(watcher: ConfigWatcher, state: ProxyState) {
    let mut receiver = watcher.subscribe();
    let mut current = watcher.current().server_url;
    loop {
        if receiver.changed().await.is_err() {
            return;
        }
        let config = receiver.borrow_and_update().clone();
        if config.server_url == current {
            continue;
        }
        match Upstream::new(&config.server_url) {
            Ok(upstream) => {
                println!(
                    "Bridge proxy: retargeted to {} (cookie jar reset)",
                    upstream.target().origin()
                );
                state.replace(upstream).await;
                current = config.server_url;
            }
            Err(err) => eprintln!("Bridge proxy: ignoring invalid server_url: {err}"),
        }
    }
}

pub fn router(state: ProxyState) -> Router {
    Router::new()
        .route("/health", any(handle_health))
        .route("/proxy/csrf", any(handle_proxy_csrf))
        .route("/benchmark/fcp", any(handle_benchmark_fcp))
        .route(LOGIN_PATH, any(handle_login))
        .route("/api/{*rest}", any(handle_api))
        .route("/plugins/{*rest}", any(handle_forward_raw))
        .route("/extensions/{*rest}", any(handle_forward_raw))
        .route("/public/{*rest}", any(handle_forward_raw))
        .route("/socket.io/", any(handle_socket_io))
        .route("/socket.io", any(handle_socket_io))
        .fallback(handle_not_found)
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct LoginBody {
    username: String,
    password: String,
}

async fn handle_health(State(state): State<ProxyState>, headers: HeaderMap) -> Response {
    let origin = request_origin(&headers);
    if let Some(response) = reject_bad_origin(origin.as_deref()) {
        return with_cors(response, origin.as_deref());
    }
    let upstream = state.upstream().await;
    with_cors(
        Json(json!({ "ok": true, "target": upstream.target().origin() })).into_response(),
        origin.as_deref(),
    )
}

#[derive(Debug, Deserialize)]
struct FcpPayload {
    fcp_ms: f64,
    method: String,
}

async fn handle_benchmark_fcp(
    headers: HeaderMap,
    body: Result<Json<FcpPayload>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let origin = request_origin(&headers);
    if let Some(response) = reject_bad_origin(origin.as_deref()) {
        return with_cors(response, origin.as_deref());
    }
    if let Ok(Json(payload)) = body {
        let doc = json!({
            "fcp_ms": payload.fcp_ms,
            "method": payload.method,
        });
        let _ = std::fs::write("/tmp/aqvn-benchmark-fcp.json", doc.to_string());
    }
    with_cors(
        Json(json!({ "ok": true })).into_response(),
        origin.as_deref(),
    )
}

/// Returns the current CSRF token to the webview.
///
/// Socket.IO's namespace CONNECT carries `auth.csrf_token`; the server checks
/// it against the session cookie the proxy holds. The token must therefore be
/// readable by page JS (it is not a cookie and never leaves the proxy⇄page
/// boundary).
async fn handle_proxy_csrf(State(state): State<ProxyState>, headers: HeaderMap) -> Response {
    let origin = request_origin(&headers);
    if let Some(response) = reject_bad_origin(origin.as_deref()) {
        return with_cors(response, origin.as_deref());
    }
    let upstream = state.upstream().await;
    let response = match upstream.csrf().await {
        Ok((runtime_id, token)) => {
            Json(json!({ "ok": true, "runtime_id": runtime_id, "token": token })).into_response()
        }
        Err(err) => text_response(StatusCode::BAD_GATEWAY, err),
    };
    with_cors(response, origin.as_deref())
}

async fn handle_login(
    State(state): State<ProxyState>,
    method: Method,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let origin = request_origin(&headers);
    if let Some(response) = reject_bad_origin(origin.as_deref()) {
        return with_cors(response, origin.as_deref());
    }
    // The webview sends JSON, so the browser preflights /login. A 4xx
    // preflight is a hard CORS failure, so answer it before parsing a body.
    if method == Method::OPTIONS {
        return with_cors(StatusCode::NO_CONTENT.into_response(), origin.as_deref());
    }

    let login: LoginBody = match serde_json::from_slice(&body) {
        Ok(login) => login,
        Err(err) => {
            return with_cors(
                text_response(
                    StatusCode::BAD_REQUEST,
                    format!("login body must be JSON {{username,password}}: {err}"),
                ),
                origin.as_deref(),
            )
        }
    };
    if login.username.trim().is_empty() || login.password.is_empty() {
        return with_cors(
            text_response(
                StatusCode::BAD_REQUEST,
                "username and password are required".to_string(),
            ),
            origin.as_deref(),
        );
    }

    let upstream = state.upstream().await;
    let response = match upstream.login(&login.username, &login.password).await {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(err) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "ok": false, "error": err })),
        )
            .into_response(),
    };
    with_cors(response, origin.as_deref())
}

async fn handle_api(
    State(state): State<ProxyState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let origin = request_origin(&headers);
    if let Some(response) = reject_bad_origin(origin.as_deref()) {
        return with_cors(response, origin.as_deref());
    }
    if method == Method::OPTIONS {
        return with_cors(StatusCode::NO_CONTENT.into_response(), origin.as_deref());
    }
    if !matches!(
        method,
        Method::GET | Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    ) {
        return with_cors(
            text_response(
                StatusCode::METHOD_NOT_ALLOWED,
                format!("method {method} is not proxied"),
            ),
            origin.as_deref(),
        );
    }

    let Ok(upstream_method) = reqwest::Method::from_bytes(method.as_str().as_bytes()) else {
        return with_cors(
            text_response(StatusCode::METHOD_NOT_ALLOWED, "unsupported method"),
            origin.as_deref(),
        );
    };
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let upstream = state.upstream().await;
    let result = upstream
        .forward(
            upstream_method,
            &path_and_query(&uri),
            content_type.as_deref(),
            body.to_vec(),
        )
        .await;
    upstream_response(result, origin.as_deref())
}

/// Raw passthrough for static assets (`/plugins/*`, `/extensions/*`, `/public/*`).
///
/// No CSRF token is required: the jar replays its session cookies upstream and
/// the upstream `Set-Cookie` headers update the jar, so plugin assets load
/// without the webview ever holding a credential.
async fn handle_forward_raw(
    State(state): State<ProxyState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let origin = request_origin(&headers);
    if let Some(response) = reject_bad_origin(origin.as_deref()) {
        return with_cors(response, origin.as_deref());
    }
    if method == Method::OPTIONS {
        return with_cors(StatusCode::NO_CONTENT.into_response(), origin.as_deref());
    }
    let Ok(upstream_method) = reqwest::Method::from_bytes(method.as_str().as_bytes()) else {
        return with_cors(
            text_response(StatusCode::METHOD_NOT_ALLOWED, "unsupported method"),
            origin.as_deref(),
        );
    };
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let upstream = state.upstream().await;
    let result = upstream
        .forward_raw(
            upstream_method,
            &path_and_query(&uri),
            content_type.as_deref(),
            body.to_vec(),
        )
        .await;
    upstream_response(result, origin.as_deref())
}

async fn handle_socket_io(State(state): State<ProxyState>, request: Request<Body>) -> Response {
    let origin = request_origin(request.headers());
    if let Some(response) = reject_bad_origin(origin.as_deref()) {
        return with_cors(response, origin.as_deref());
    }
    if request.method() == Method::OPTIONS {
        return with_cors(StatusCode::NO_CONTENT.into_response(), origin.as_deref());
    }

    if is_websocket_upgrade(&request) {
        let (mut parts, _body) = request.into_parts();
        let upgrade = match WebSocketUpgrade::from_request_parts(&mut parts, &state).await {
            Ok(upgrade) => upgrade,
            Err(rejection) => return with_cors(rejection.into_response(), origin.as_deref()),
        };
        let path_and_query = path_and_query(&parts.uri);
        let upstream = state.upstream().await;
        return upgrade.on_upgrade(move |socket| ws::bridge(socket, upstream, path_and_query));
    }

    // Engine.IO polling fallback: plain HTTP passthrough on the same path.
    let (parts, body) = request.into_parts();
    let Ok(bytes) = axum::body::to_bytes(body, MAX_BODY_BYTES).await else {
        return with_cors(
            text_response(StatusCode::PAYLOAD_TOO_LARGE, "request body too large"),
            origin.as_deref(),
        );
    };
    let Ok(method) = reqwest::Method::from_bytes(parts.method.as_str().as_bytes()) else {
        return with_cors(
            text_response(StatusCode::METHOD_NOT_ALLOWED, "unsupported method"),
            origin.as_deref(),
        );
    };
    let content_type = parts
        .headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let upstream = state.upstream().await;
    let result = upstream
        .forward_raw(
            method,
            &path_and_query(&parts.uri),
            content_type.as_deref(),
            bytes.to_vec(),
        )
        .await;
    upstream_response(result, origin.as_deref())
}

async fn handle_not_found() -> Response {
    text_response(
        StatusCode::NOT_FOUND,
        "bridge proxy: only /login, /proxy/csrf, /api/*, /plugins/*, /extensions/*, /public/*, and /socket.io/ are proxied",
    )
}

fn upstream_response(result: Result<ForwardResponse, String>, origin: Option<&str>) -> Response {
    match result {
        Ok(outcome) => {
            let mut response = Response::new(Body::from(outcome.body));
            *response.status_mut() =
                StatusCode::from_u16(outcome.status).unwrap_or(StatusCode::BAD_GATEWAY);
            if let Some(content_type) = outcome.content_type {
                if let Ok(value) = HeaderValue::from_str(&content_type) {
                    response.headers_mut().insert(header::CONTENT_TYPE, value);
                }
            }
            with_cors(response, origin)
        }
        Err(err) => with_cors(text_response(StatusCode::BAD_GATEWAY, err), origin),
    }
}

fn text_response(status: StatusCode, message: impl Into<String>) -> Response {
    (
        status,
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        )],
        message.into(),
    )
        .into_response()
}

fn request_origin(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

fn reject_bad_origin(origin: Option<&str>) -> Option<Response> {
    match origin {
        Some(origin) if !allowed_origin(origin) => Some(text_response(
            StatusCode::FORBIDDEN,
            format!("bridge proxy: origin {origin:?} is not allowed"),
        )),
        _ => None,
    }
}

/// The webview origins and local validation servers may talk to the proxy.
/// Everything else is rejected so arbitrary web pages cannot drive it.
pub fn allowed_origin(origin: &str) -> bool {
    if origin == "tauri://localhost" {
        return true;
    }
    let Ok(url) = url::Url::parse(origin) else {
        return false;
    };
    if url.scheme() != "http" && url.scheme() != "https" {
        return false;
    }
    matches!(
        url.host_str(),
        Some("localhost") | Some("127.0.0.1") | Some("tauri.localhost")
    )
}

fn with_cors(mut response: Response, origin: Option<&str>) -> Response {
    let Some(origin) = origin else {
        return response;
    };
    if !allowed_origin(origin) {
        return response;
    }
    let Ok(value) = HeaderValue::from_str(origin) else {
        return response;
    };
    let headers = response.headers_mut();
    headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, value);
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST, OPTIONS"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("Content-Type, X-CSRF-Token, Accept"),
    );
    headers.append(header::VARY, HeaderValue::from_static("Origin"));
    response
}

fn is_websocket_upgrade(request: &Request<Body>) -> bool {
    let connection_upgrade = request
        .headers()
        .get(header::CONNECTION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(',')
                .any(|part| part.trim().eq_ignore_ascii_case("upgrade"))
        });
    let websocket = request
        .headers()
        .get(header::UPGRADE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case("websocket"));
    connection_upgrade && websocket
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|value| value.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_script_carries_the_runtime_port() {
        let script = init_script(54321);
        assert!(script.contains("port: 54321"));
        assert!(script.contains("http://127.0.0.1:54321"));
    }

    #[test]
    fn accepts_webview_and_local_origins() {
        for origin in [
            "tauri://localhost",
            "http://tauri.localhost",
            "https://tauri.localhost",
            "http://localhost:3155",
            "http://127.0.0.1:8080",
        ] {
            assert!(allowed_origin(origin), "{origin} must be allowed");
        }
    }

    #[test]
    fn rejects_foreign_origins() {
        for origin in [
            "https://evil.example",
            "http://localhost.evil.example",
            "file://",
            "null",
            "not a url",
        ] {
            assert!(!allowed_origin(origin), "{origin} must be rejected");
        }
    }

    #[test]
    fn binds_loopback_only_on_an_ephemeral_port() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let (_listener, _state, info) = runtime
            .block_on(bind("http://127.0.0.1:5000"))
            .expect("bind");

        assert_ne!(info.port, 0, "kernel must resolve a real port");
        assert_eq!(info.url, format!("http://127.0.0.1:{}", info.port));
    }
}
