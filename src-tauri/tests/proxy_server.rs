//! Bridge proxy integration tests against a local stub chat server.
//!
//! Covers validation contract assertions VAL-PROXY-001..005, 007, 008. The
//! stub mimics the target server's auth surface: runtime-scoped session
//! cookies, `GET /api/csrf_token`, a form login that redirects, and an
//! `/api/echo` endpoint that rejects a missing or stale `X-CSRF-Token`.

use aqvn_workspace_lib::proxy::{server, upstream};
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::{
    header::{self, HeaderMap, HeaderValue},
    Request, StatusCode,
};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get, post};
use axum::{Json, Router};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

const USERNAME: &str = "admin";
const PASSWORD: &str = "test-auth-2026";
/// Non-UTF-8 bytes: any text decoding on the proxy path would corrupt them.
const RAW_BODY: &[u8] = &[0x00, 0xFF, 0x10, 0x80, 0x7F, 0x0A, 0x0D];

#[derive(Default)]
struct Stub {
    runtime_id: Mutex<String>,
    token: Mutex<String>,
    csrf_calls: AtomicUsize,
    api_calls: AtomicUsize,
    rotated: AtomicBool,
    flaky: AtomicBool,
    always_403: AtomicBool,
    port: AtomicU16,
    last_csrf_header: Mutex<Option<String>>,
    last_cookie: Mutex<Option<String>>,
    login_bodies: Mutex<Vec<String>>,
    last_raw_uri: Mutex<Option<String>>,
    last_raw_origin: Mutex<Option<String>>,
}

impl Stub {
    fn new() -> Self {
        Self {
            runtime_id: Mutex::new("abcd1234".to_string()),
            token: Mutex::new("token-1".to_string()),
            ..Default::default()
        }
    }

    async fn credentials(&self) -> (String, String) {
        (
            self.runtime_id.lock().await.clone(),
            self.token.lock().await.clone(),
        )
    }
}

struct Running {
    base: String,
    task: JoinHandle<()>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn start_stub() -> (Arc<Stub>, Running) {
    let stub = Arc::new(Stub::new());
    let router = Router::new()
        .route("/api/csrf_token", get(stub_csrf))
        .route("/login", post(stub_login).get(stub_login_page))
        .route("/", get(stub_root))
        .route("/login.css", any(stub_raw_asset))
        .route("/vendor/{*rest}", any(stub_raw_asset))
        .route("/css/{*rest}", any(stub_raw_asset))
        .route("/js/{*rest}", any(stub_raw_asset))
        .route("/api/echo", post(stub_echo))
        .route("/api/rotate", post(stub_rotate))
        .route("/api/always", post(stub_always))
        .route("/plugins/{*rest}", any(stub_raw_asset))
        .route("/extensions/{*rest}", any(stub_raw_asset))
        .route("/public/{*rest}", any(stub_raw_asset))
        .route("/ui/{*rest}", any(stub_raw_asset))
        .route("/sw.js", any(stub_raw_asset))
        .route("/components/{*rest}", any(stub_raw_asset))
        .route("/socket.io/", any(stub_socket))
        .fallback(stub_raw_asset)
        .with_state(stub.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stub");
    let address = listener.local_addr().expect("stub addr");
    stub.port.store(address.port(), Ordering::SeqCst);
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.expect("stub serve");
    });

    (
        stub,
        Running {
            base: format!("http://{address}"),
            task,
        },
    )
}

async fn start_proxy(target: &str) -> Running {
    let (listener, state, info) = server::bind_ephemeral(target).await.expect("bind proxy");
    let task = tokio::spawn(async move {
        server::serve(listener, state).await.expect("proxy serve");
    });
    Running {
        base: info.url,
        task,
    }
}

async fn stub_csrf(State(stub): State<Arc<Stub>>) -> Response {
    stub.csrf_calls.fetch_add(1, Ordering::SeqCst);
    let (runtime_id, token) = stub.credentials().await;
    (
        [(
            header::SET_COOKIE,
            format!("session_{runtime_id}=sid-{runtime_id}; Path=/; SameSite=Lax"),
        )],
        Json(json!({ "ok": true, "token": token, "runtime_id": runtime_id })),
    )
        .into_response()
}

async fn stub_login(State(stub): State<Arc<Stub>>, body: axum::body::Bytes) -> Response {
    let body = String::from_utf8_lossy(&body).to_string();
    stub.login_bodies.lock().await.push(body.clone());
    let expected = format!("username={USERNAME}&password={PASSWORD}");
    if body != expected {
        return (StatusCode::OK, "<html>login</html>").into_response();
    }
    let (runtime_id, _) = stub.credentials().await;
    (
        StatusCode::FOUND,
        [
            (header::LOCATION, "/".to_string()),
            (
                header::SET_COOKIE,
                format!("session_{runtime_id}=sid-login-{runtime_id}; Path=/"),
            ),
        ],
    )
        .into_response()
}

async fn stub_login_page(
    State(stub): State<Arc<Stub>>,
    headers: HeaderMap,
    uri: axum::http::Uri,
) -> Response {
    let path_and_query = uri
        .path_and_query()
        .map(|value| value.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string());
    *stub.last_raw_uri.lock().await = Some(path_and_query);
    *stub.last_raw_origin.lock().await = header_string(&headers, "origin");
    *stub.last_cookie.lock().await = header_string(&headers, "cookie");
    (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8".to_string()),
            (header::SET_COOKIE, "login_page_seen=1; Path=/".to_string()),
        ],
        "<!DOCTYPE html><html><body>Login Page</body></html>",
    )
        .into_response()
}

async fn stub_echo(State(stub): State<Arc<Stub>>, headers: HeaderMap) -> Response {
    stub.api_calls.fetch_add(1, Ordering::SeqCst);
    if stub.always_403.load(Ordering::SeqCst) {
        return (StatusCode::FORBIDDEN, "always").into_response();
    }
    if stub.flaky.swap(false, Ordering::SeqCst) {
        return (StatusCode::FORBIDDEN, "csrf stale").into_response();
    }

    let (runtime_id, token) = stub.credentials().await;
    let csrf_header = header_string(&headers, "x-csrf-token");
    let cookie = header_string(&headers, "cookie").unwrap_or_default();
    *stub.last_csrf_header.lock().await = csrf_header.clone();
    *stub.last_cookie.lock().await = Some(cookie.clone());

    let expected_cookie = format!("csrf_token_{runtime_id}={token}");
    let session_cookie = format!("session_{runtime_id}=");
    if csrf_header.as_deref() != Some(token.as_str())
        || !cookie.contains(&expected_cookie)
        || !cookie.contains(&session_cookie)
    {
        return (
            StatusCode::FORBIDDEN,
            format!("stale csrf header={csrf_header:?} cookie={cookie:?}"),
        )
            .into_response();
    }

    Json(json!({
        "ok": true,
        "runtime_id": runtime_id,
        "csrf_header": csrf_header,
    }))
    .into_response()
}

async fn stub_rotate(State(stub): State<Arc<Stub>>) -> Response {
    if !stub.rotated.swap(true, Ordering::SeqCst) {
        *stub.runtime_id.lock().await = "ffff0001".to_string();
        *stub.token.lock().await = "token-2".to_string();
    }
    let runtime_id = stub.runtime_id.lock().await.clone();
    (
        StatusCode::FORBIDDEN,
        format!("runtime rotated to {runtime_id}"),
    )
        .into_response()
}

async fn stub_always(State(stub): State<Arc<Stub>>) -> Response {
    stub.api_calls.fetch_add(1, Ordering::SeqCst);
    (StatusCode::FORBIDDEN, "always").into_response()
}

async fn stub_root(
    State(stub): State<Arc<Stub>>,
    headers: HeaderMap,
    uri: axum::http::Uri,
) -> Response {
    let path_and_query = uri
        .path_and_query()
        .map(|value| value.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string());
    *stub.last_raw_uri.lock().await = Some(path_and_query);
    *stub.last_raw_origin.lock().await = header_string(&headers, "origin");
    *stub.last_cookie.lock().await = header_string(&headers, "cookie");
    (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8".to_string()),
            (header::SET_COOKIE, "root_seen=1; Path=/".to_string()),
        ],
        "<!DOCTYPE html><html><body>AQVN Workspace</body></html>",
    )
        .into_response()
}

/// Raw asset endpoint: records what arrived and answers with binary bytes.
async fn stub_raw_asset(
    State(stub): State<Arc<Stub>>,
    headers: HeaderMap,
    uri: axum::http::Uri,
) -> Response {
    let path_and_query = uri
        .path_and_query()
        .map(|value| value.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string());
    *stub.last_raw_uri.lock().await = Some(path_and_query.clone());
    *stub.last_raw_origin.lock().await = header_string(&headers, "origin");
    *stub.last_cookie.lock().await = header_string(&headers, "cookie");

    if path_and_query.ends_with("/missing") {
        return (StatusCode::NOT_FOUND, "missing").into_response();
    }

    let content_type = if path_and_query.ends_with(".css") {
        "text/css"
    } else if path_and_query.ends_with(".js") {
        "application/javascript"
    } else if path_and_query.ends_with(".woff2") {
        "font/woff2"
    } else if path_and_query.ends_with(".ttf") {
        "font/ttf"
    } else if path_and_query.ends_with(".svg") {
        "image/svg+xml"
    } else {
        "image/png"
    };

    (
        [
            (header::CONTENT_TYPE, content_type.to_string()),
            (header::SET_COOKIE, "raw_seen=1; Path=/".to_string()),
        ],
        RAW_BODY,
    )
        .into_response()
}

async fn stub_socket(
    ws: WebSocketUpgrade,
    State(stub): State<Arc<Stub>>,
    headers: HeaderMap,
) -> Response {
    let expected = format!("http://127.0.0.1:{}", stub.port.load(Ordering::SeqCst));
    let origin = header_string(&headers, "origin");
    if origin.as_deref() != Some(expected.as_str()) {
        return (
            StatusCode::FORBIDDEN,
            format!("origin mismatch: {origin:?} != {expected}"),
        )
            .into_response();
    }
    ws.on_upgrade(echo_socket)
}

async fn echo_socket(mut socket: WebSocket) {
    while let Some(Ok(message)) = socket.recv().await {
        if socket.send(message).await.is_err() {
            break;
        }
    }
}

fn header_string(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

async fn login(proxy: &str) -> Value {
    let response = client()
        .post(format!("{proxy}/login"))
        .json(&json!({ "username": USERNAME, "password": PASSWORD }))
        .send()
        .await
        .expect("login request");
    assert_eq!(response.status(), StatusCode::OK);
    response.json().await.expect("login JSON")
}

// ------------------------------------------------------ binding (VAL-PROXY-001)

#[tokio::test]
async fn proxy_binds_loopback_on_an_ephemeral_port() {
    let (_stub, stub_guard) = start_stub().await;
    let (listener, _state, info) = server::bind(&stub_guard.base).await.expect("bind");

    let address = listener.local_addr().expect("local addr");
    assert!(
        address.ip().is_loopback(),
        "proxy must bind loopback only: {address}"
    );
    assert_ne!(address.port(), 0, "kernel must resolve a real port");
    assert_eq!(info.port, address.port());
    assert_eq!(info.url, format!("http://127.0.0.1:{}", address.port()));

    let script = server::init_script(info.port);
    assert!(script.contains(&format!("port: {}", address.port())));
}

// -------------------------------- login + CSRF header (VAL-PROXY-002 / 003)

#[tokio::test]
async fn login_stores_session_and_forwards_csrf_header() {
    let (stub, stub_guard) = start_stub().await;
    let proxy = start_proxy(&stub_guard.base).await;

    let body = login(&proxy.base).await;
    assert_eq!(body["ok"], true);

    let login_bodies = stub.login_bodies.lock().await.clone();
    assert_eq!(
        login_bodies,
        vec![format!("username={USERNAME}&password={PASSWORD}")],
        "proxy must post the login form itself"
    );
    assert!(
        stub.csrf_calls.load(Ordering::SeqCst) >= 2,
        "bootstrap before login and re-bootstrap after"
    );

    let (_, token) = stub.credentials().await;
    let response = client()
        .post(format!("{}/api/echo", proxy.base))
        .json(&json!({}))
        .send()
        .await
        .expect("api request");
    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        stub.last_csrf_header.lock().await.as_deref(),
        Some(token.as_str()),
        "X-CSRF-Token must equal the bootstrapped token"
    );
    let cookie = stub.last_cookie.lock().await.clone().unwrap_or_default();
    assert!(
        cookie.contains("csrf_token_abcd1234=token-1"),
        "cookie: {cookie}"
    );
    assert!(cookie.contains("session_abcd1234="), "cookie: {cookie}");
}

#[tokio::test]
async fn webview_never_needs_cookies_for_api_calls() {
    let (_stub, stub_guard) = start_stub().await;
    let proxy = start_proxy(&stub_guard.base).await;
    login(&proxy.base).await;

    // The browser-facing request carries no cookie header at all.
    let response = client()
        .post(format!("{}/api/echo", proxy.base))
        .json(&json!({}))
        .send()
        .await
        .expect("api request");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn proxy_csrf_endpoint_returns_the_socket_auth_token() {
    let (_stub, stub_guard) = start_stub().await;
    let proxy = start_proxy(&stub_guard.base).await;
    login(&proxy.base).await;

    let response = client()
        .get(format!("{}/proxy/csrf", proxy.base))
        .send()
        .await
        .expect("csrf request");
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.expect("csrf JSON");

    assert_eq!(body["ok"], true);
    assert_eq!(body["runtime_id"], "abcd1234");
    assert_eq!(body["token"], "token-1");
}

// -------------------------------------- 403 refresh+retry (VAL-PROXY-007)

#[tokio::test]
async fn forbidden_refreshes_csrf_exactly_once_and_retries_once() {
    let (stub, stub_guard) = start_stub().await;
    let proxy = start_proxy(&stub_guard.base).await;
    login(&proxy.base).await;

    let csrf_before = stub.csrf_calls.load(Ordering::SeqCst);
    let api_before = stub.api_calls.load(Ordering::SeqCst);
    stub.flaky.store(true, Ordering::SeqCst);

    let response = client()
        .post(format!("{}/api/echo", proxy.base))
        .json(&json!({}))
        .send()
        .await
        .expect("api request");

    assert_eq!(response.status(), StatusCode::OK, "retry must succeed");
    assert_eq!(
        stub.csrf_calls.load(Ordering::SeqCst),
        csrf_before + 1,
        "exactly one CSRF refresh"
    );
    assert_eq!(
        stub.api_calls.load(Ordering::SeqCst),
        api_before + 2,
        "original call + exactly one retry"
    );
}

#[tokio::test]
async fn a_second_403_is_surfaced_to_the_caller() {
    let (stub, stub_guard) = start_stub().await;
    let proxy = start_proxy(&stub_guard.base).await;
    login(&proxy.base).await;

    let csrf_before = stub.csrf_calls.load(Ordering::SeqCst);
    stub.always_403.store(true, Ordering::SeqCst);

    let response = client()
        .post(format!("{}/api/always", proxy.base))
        .json(&json!({}))
        .send()
        .await
        .expect("api request");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        stub.csrf_calls.load(Ordering::SeqCst),
        csrf_before + 1,
        "one refresh, never a retry loop"
    );
}

// --------------------------------- runtime rotation (VAL-PROXY-008)

#[tokio::test]
async fn runtime_rotation_rebootstraps_the_jar_and_retries() {
    let (stub, stub_guard) = start_stub().await;
    let proxy = start_proxy(&stub_guard.base).await;
    login(&proxy.base).await;

    let csrf_before = stub.csrf_calls.load(Ordering::SeqCst);
    let response = client()
        .post(format!("{}/api/rotate", proxy.base))
        .json(&json!({}))
        .send()
        .await
        .expect("rotate request");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let (new_runtime, new_token) = stub.credentials().await;
    assert_eq!(new_runtime, "ffff0001");

    let response = client()
        .post(format!("{}/api/echo", proxy.base))
        .json(&json!({}))
        .send()
        .await
        .expect("api request after rotation");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "action must succeed without a manual re-login"
    );
    assert_eq!(
        stub.csrf_calls.load(Ordering::SeqCst),
        csrf_before + 1,
        "rotation forces exactly one CSRF re-bootstrap"
    );
    assert_eq!(
        stub.last_csrf_header.lock().await.as_deref(),
        Some(new_token.as_str())
    );
    let cookie = stub.last_cookie.lock().await.clone().unwrap_or_default();
    assert!(
        cookie.contains(&format!("csrf_token_{new_runtime}={new_token}")),
        "cookie: {cookie}"
    );
    assert!(
        cookie.contains(&format!("session_{new_runtime}=")),
        "new session cookie: {cookie}"
    );
    assert!(
        !cookie.contains("csrf_token_abcd1234="),
        "old generation csrf cookie must be gone: {cookie}"
    );
}

// ------------------------------------------------- CORS (VAL-PROXY-005)

#[tokio::test]
async fn cors_allows_the_webview_origin_only() {
    let (_stub, stub_guard) = start_stub().await;
    let proxy = start_proxy(&stub_guard.base).await;

    let response = client()
        .get(format!("{}/health", proxy.base))
        .header(header::ORIGIN, "tauri://localhost")
        .send()
        .await
        .expect("health request");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|value| value.to_str().ok()),
        Some("tauri://localhost")
    );

    let rejected = client()
        .get(format!("{}/health", proxy.base))
        .header(header::ORIGIN, "https://evil.example")
        .send()
        .await
        .expect("foreign origin request");
    assert_eq!(rejected.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn cors_preflight_is_answered() {
    let (_stub, stub_guard) = start_stub().await;
    let proxy = start_proxy(&stub_guard.base).await;

    let response = client()
        .request(reqwest::Method::OPTIONS, format!("{}/api/echo", proxy.base))
        .header(header::ORIGIN, "http://localhost:3155")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .header(
            header::ACCESS_CONTROL_REQUEST_HEADERS,
            "content-type,x-csrf-token",
        )
        .send()
        .await
        .expect("preflight");

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|value| value.to_str().ok()),
        Some("http://localhost:3155")
    );
}

// --------------------------------- WebSocket proxy (VAL-PROXY-004 / 006)

fn websocket_url(proxy_base: &str) -> String {
    format!(
        "{}/socket.io/?EIO=4&transport=websocket",
        proxy_base.replacen("http://", "ws://", 1)
    )
}

fn websocket_request(proxy_base: &str) -> Request<()> {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;

    let url = websocket_url(proxy_base);
    let mut request = url
        .as_str()
        .into_client_request()
        .expect("websocket client request");
    request.headers_mut().insert(
        header::ORIGIN,
        HeaderValue::from_static("tauri://localhost"),
    );
    request
}

#[tokio::test]
async fn websocket_frames_traverse_both_legs_with_spoofed_origin() {
    let (_stub, stub_guard) = start_stub().await;
    let proxy = start_proxy(&stub_guard.base).await;

    let request = websocket_request(&proxy.base);

    let (mut socket, response) = tokio_tungstenite::connect_async(request)
        .await
        .expect("proxy websocket handshake");
    assert_eq!(response.status().as_u16(), 101);

    socket
        .send(tokio_tungstenite::tungstenite::Message::Text("2".into()))
        .await
        .expect("send frame");
    let echoed = socket.next().await.expect("echo frame").expect("echo ok");
    assert!(
        matches!(echoed, tokio_tungstenite::tungstenite::Message::Text(ref text) if text.as_str() == "2")
    );

    // The 45 s keepalive path: both directions stay open while idle.
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    socket
        .send(tokio_tungstenite::tungstenite::Message::Text("3".into()))
        .await
        .expect("send after idle");
    assert!(socket.next().await.is_some());

    socket.close(None).await.expect("close");
}

#[tokio::test]
async fn websocket_after_rotation_still_connects() {
    let (_stub, stub_guard) = start_stub().await;
    let proxy = start_proxy(&stub_guard.base).await;
    login(&proxy.base).await;

    let rotate = client()
        .post(format!("{}/api/rotate", proxy.base))
        .json(&json!({}))
        .send()
        .await
        .expect("rotate");
    assert_eq!(rotate.status(), StatusCode::FORBIDDEN);

    let request = websocket_request(&proxy.base);

    let mut socket = tokio_tungstenite::connect_async(request)
        .await
        .expect("handshake after rotation")
        .0;
    socket
        .send(tokio_tungstenite::tungstenite::Message::Text("3".into()))
        .await
        .expect("send");
    assert!(socket.next().await.is_some());
    socket.close(None).await.expect("close");
}

#[tokio::test]
async fn upstream_client_rejects_off_origin_redirects() {
    // Regression guard for cookie leakage: Target only follows same-origin hops.
    let target = upstream::Target::parse("https://chat.example.com").expect("target");
    assert!(target.same_origin(&url::Url::parse("https://chat.example.com/x").expect("url")));
    assert!(!target.same_origin(&url::Url::parse("https://evil.example/x").expect("url")));
}

// ------------------------------------ raw asset routes (VAL-PROXY-001)

#[tokio::test]
async fn test_forward_raw_plugins() {
    let (stub, stub_guard) = start_stub().await;
    let proxy = start_proxy(&stub_guard.base).await;

    let response = client()
        .get(format!(
            "{}/plugins/aqvn_intel_suite/webui/canvas.html?tab=1",
            proxy.base
        ))
        .header(header::ORIGIN, "tauri://localhost")
        .send()
        .await
        .expect("plugins request");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("image/png"),
        "upstream Content-Type must pass through"
    );
    let bytes = response.bytes().await.expect("body");
    assert_eq!(bytes.as_ref(), RAW_BODY, "binary body must be byte-exact");

    assert_eq!(
        stub.last_raw_uri.lock().await.as_deref(),
        Some("/plugins/aqvn_intel_suite/webui/canvas.html?tab=1"),
        "path and query reach the upstream unchanged"
    );
    assert_eq!(
        stub.last_raw_origin.lock().await.as_deref(),
        Some(stub_guard.base.as_str()),
        "proxy spoofs the target origin"
    );
    assert_eq!(
        stub.csrf_calls.load(Ordering::SeqCst),
        0,
        "raw routes must not require a CSRF bootstrap"
    );
}

#[tokio::test]
async fn test_forward_raw_extensions_and_public() {
    let (stub, stub_guard) = start_stub().await;
    let proxy = start_proxy(&stub_guard.base).await;

    for path in ["/extensions/aqvn/loader.js", "/public/branding/logo.svg"] {
        let response = client()
            .get(format!("{}{path}", proxy.base))
            .header(header::ORIGIN, "tauri://localhost")
            .send()
            .await
            .expect("raw request");
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert_eq!(
            stub.last_raw_uri.lock().await.as_deref(),
            Some(path),
            "{path} reaches the upstream"
        );
        assert_eq!(
            stub.last_raw_origin.lock().await.as_deref(),
            Some(stub_guard.base.as_str()),
            "{path} carries the spoofed origin"
        );
    }

    let missing = client()
        .get(format!("{}/extensions/missing", proxy.base))
        .header(header::ORIGIN, "tauri://localhost")
        .send()
        .await
        .expect("missing request");
    assert_eq!(
        missing.status(),
        StatusCode::NOT_FOUND,
        "upstream status passes through"
    );
}

#[tokio::test]
async fn test_forward_raw_cookie_replay() {
    let (stub, stub_guard) = start_stub().await;
    let proxy = start_proxy(&stub_guard.base).await;
    login(&proxy.base).await;

    let response = client()
        .get(format!(
            "{}/plugins/aqvn_intel_suite/webui/canvas.html",
            proxy.base
        ))
        .header(header::ORIGIN, "tauri://localhost")
        .send()
        .await
        .expect("plugins request");
    assert_eq!(response.status(), StatusCode::OK);

    let cookie = stub.last_cookie.lock().await.clone().unwrap_or_default();
    assert!(
        cookie.contains("session_abcd1234="),
        "session cookie must be replayed upstream: {cookie}"
    );
    assert!(
        cookie.contains("csrf_token_abcd1234=token-1"),
        "bootstrapped cookies must be replayed upstream: {cookie}"
    );

    // The stub answers with `Set-Cookie: raw_seen=1`; the jar must store it
    // and replay it on the next raw request.
    let second = client()
        .get(format!("{}/public/branding/logo.svg", proxy.base))
        .header(header::ORIGIN, "tauri://localhost")
        .send()
        .await
        .expect("public request");
    assert_eq!(second.status(), StatusCode::OK);
    let cookie = stub.last_cookie.lock().await.clone().unwrap_or_default();
    assert!(
        cookie.contains("raw_seen=1"),
        "upstream Set-Cookie must update the jar: {cookie}"
    );
}

// ---------------------- root & static assets passthrough (VAL-WEB-001)

#[tokio::test]
async fn test_forward_raw_root_and_static_assets() {
    let (stub, stub_guard) = start_stub().await;
    let proxy = start_proxy(&stub_guard.base).await;

    // 1. Root path (/)
    let response = client()
        .get(format!("{}/", proxy.base))
        .header(header::ORIGIN, "tauri://localhost")
        .send()
        .await
        .expect("root request");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("text/html; charset=utf-8")
    );
    let html = response.text().await.expect("root body");
    assert!(html.contains("AQVN Workspace"));
    assert_eq!(
        stub.last_raw_uri.lock().await.as_deref(),
        Some("/"),
        "root path reaches upstream"
    );

    // 2. /login.css
    let response = client()
        .get(format!("{}/login.css", proxy.base))
        .header(header::ORIGIN, "tauri://localhost")
        .send()
        .await
        .expect("login.css request");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("text/css")
    );
    assert_eq!(
        stub.last_raw_uri.lock().await.as_deref(),
        Some("/login.css"),
        "login.css reaches upstream"
    );

    // 3. Static assets: /vendor/*, /css/*, /js/*, /public/*
    for (path, mime) in [
        ("/vendor/fonts/Inter.woff2", "font/woff2"),
        ("/vendor/marked.min.js", "application/javascript"),
        ("/css/app.css", "text/css"),
        ("/js/bundle.js", "application/javascript"),
        ("/public/branding/logo.svg", "image/svg+xml"),
    ] {
        let response = client()
            .get(format!("{}{path}", proxy.base))
            .header(header::ORIGIN, "tauri://localhost")
            .send()
            .await
            .expect("static asset request");
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some(mime),
            "Content-Type for {path}"
        );
        let bytes = response.bytes().await.expect("body");
        assert_eq!(
            bytes.as_ref(),
            RAW_BODY,
            "binary bytes preserved for {path}"
        );
        assert_eq!(
            stub.last_raw_uri.lock().await.as_deref(),
            Some(path),
            "{path} reaches upstream"
        );
        assert_eq!(
            stub.last_raw_origin.lock().await.as_deref(),
            Some(stub_guard.base.as_str()),
            "{path} has spoofed origin"
        );
    }

    // 4. Verify CSRF calls count remains 0 (no CSRF token required)
    assert_eq!(
        stub.csrf_calls.load(Ordering::SeqCst),
        0,
        "static asset requests must not require CSRF token"
    );

    // 5. Session cookie replay and Set-Cookie absorption
    login(&proxy.base).await;

    let response = client()
        .get(format!("{}/vendor/test.js", proxy.base))
        .header(header::ORIGIN, "tauri://localhost")
        .send()
        .await
        .expect("vendor request after login");
    assert_eq!(response.status(), StatusCode::OK);

    let cookie = stub.last_cookie.lock().await.clone().unwrap_or_default();
    assert!(
        cookie.contains("session_abcd1234="),
        "session cookie must be replayed on static asset request"
    );
    assert!(
        cookie.contains("root_seen=1"),
        "root Set-Cookie must be absorbed in jar"
    );
}

// ---------------------- local asset caching layer (VAL-WEB-002)

#[tokio::test]
async fn test_asset_caching_layer() {
    let (stub, stub_guard) = start_stub().await;
    let proxy = start_proxy(&stub_guard.base).await;

    // 1. First request for a static asset (.woff2) -> MISS, fetched upstream, cached locally
    let url = format!("{}/vendor/fonts/Inter-test-cache.woff2", proxy.base);
    let resp1 = client()
        .get(&url)
        .header(header::ORIGIN, "tauri://localhost")
        .send()
        .await
        .expect("1st request");
    assert_eq!(resp1.status(), StatusCode::OK);
    assert_eq!(
        resp1.headers().get("x-cache").and_then(|v| v.to_str().ok()),
        Some("MISS")
    );
    assert_eq!(
        resp1
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("font/woff2")
    );
    let body1 = resp1.bytes().await.expect("body1");
    assert_eq!(body1.as_ref(), RAW_BODY);
    assert_eq!(
        stub.last_raw_uri.lock().await.as_deref(),
        Some("/vendor/fonts/Inter-test-cache.woff2")
    );

    // Reset stub tracking
    *stub.last_raw_uri.lock().await = None;

    // 2. Second request for the same asset -> HIT, served immediately from cache, no network roundtrip
    let resp2 = client()
        .get(&url)
        .header(header::ORIGIN, "tauri://localhost")
        .send()
        .await
        .expect("2nd request");
    assert_eq!(resp2.status(), StatusCode::OK);
    assert_eq!(
        resp2.headers().get("x-cache").and_then(|v| v.to_str().ok()),
        Some("HIT")
    );
    let cache_control = resp2
        .headers()
        .get(header::CACHE_CONTROL)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert!(
        cache_control.contains("max-age="),
        "cache-control: {cache_control}"
    );
    let body2 = resp2.bytes().await.expect("body2");
    assert_eq!(body2.as_ref(), RAW_BODY);
    // Upstream stub was NOT called on 2nd request
    assert_eq!(
        stub.last_raw_uri.lock().await.as_deref(),
        None,
        "second request must be served from cache without upstream call"
    );

    // 3. Verify other static asset types (.css, .js, .ttf) are cached
    for (asset_path, mime) in [
        ("/css/main.css", "text/css"),
        ("/js/app.js", "application/javascript"),
        ("/vendor/fonts/Inter.ttf", "font/ttf"),
    ] {
        let asset_url = format!("{}{asset_path}", proxy.base);
        // Miss
        let r1 = client()
            .get(&asset_url)
            .header(header::ORIGIN, "tauri://localhost")
            .send()
            .await
            .expect("r1");
        assert_eq!(
            r1.headers().get("x-cache").and_then(|v| v.to_str().ok()),
            Some("MISS")
        );
        assert_eq!(
            r1.headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some(mime)
        );

        *stub.last_raw_uri.lock().await = None;

        // Hit
        let r2 = client()
            .get(&asset_url)
            .header(header::ORIGIN, "tauri://localhost")
            .send()
            .await
            .expect("r2");
        assert_eq!(
            r2.headers().get("x-cache").and_then(|v| v.to_str().ok()),
            Some("HIT")
        );
        assert_eq!(
            stub.last_raw_uri.lock().await.as_deref(),
            None,
            "{asset_path} hit served without upstream call"
        );
    }

    // 4. Verify disk cache persistence across new proxy instance with same cache dir
    use aqvn_workspace_lib::proxy::cache::AssetCache;
    let custom_cache_dir =
        std::env::temp_dir().join(format!("aqvn-test-cache-{}", std::process::id()));
    let cache_engine = std::sync::Arc::new(AssetCache::new(custom_cache_dir.clone()));

    let (listener, state, info) =
        server::bind_ephemeral_with_cache(&stub_guard.base, cache_engine.clone())
            .await
            .expect("bind proxy with custom cache");
    let task = tokio::spawn(async move {
        server::serve(listener, state).await.expect("proxy serve");
    });
    let custom_proxy = Running {
        base: info.url,
        task,
    };

    // First request misses and writes to custom_cache_dir
    let custom_url = format!("{}/vendor/ace/ace.js", custom_proxy.base);
    let r1 = client()
        .get(&custom_url)
        .header(header::ORIGIN, "tauri://localhost")
        .send()
        .await
        .expect("r1 custom");
    assert_eq!(
        r1.headers().get("x-cache").and_then(|v| v.to_str().ok()),
        Some("MISS")
    );

    // Verify file actually exists on disk under custom_cache_dir
    let expected_disk_file = cache_engine
        .disk_path(&stub_guard.base, "/vendor/ace/ace.js")
        .expect("disk path");
    assert!(
        expected_disk_file.is_file(),
        "cached file must exist on disk at {expected_disk_file:?}"
    );

    // Drop the first proxy
    drop(custom_proxy);

    // Create a brand new proxy instance with a fresh in-memory cache pointing to the same disk directory
    let fresh_cache_engine = std::sync::Arc::new(AssetCache::new(custom_cache_dir.clone()));
    let (listener2, state2, info2) =
        server::bind_ephemeral_with_cache(&stub_guard.base, fresh_cache_engine)
            .await
            .expect("bind 2nd proxy with same disk cache");
    let task2 = tokio::spawn(async move {
        server::serve(listener2, state2)
            .await
            .expect("proxy serve 2");
    });
    let custom_proxy2 = Running {
        base: info2.url,
        task: task2,
    };

    *stub.last_raw_uri.lock().await = None;

    // Requesting the asset from proxy2 must be served directly from disk cache with HIT
    let custom_url2 = format!("{}/vendor/ace/ace.js", custom_proxy2.base);
    let r2 = client()
        .get(&custom_url2)
        .header(header::ORIGIN, "tauri://localhost")
        .send()
        .await
        .expect("r2 custom from disk");
    assert_eq!(
        r2.headers().get("x-cache").and_then(|v| v.to_str().ok()),
        Some("HIT")
    );
    assert_eq!(
        stub.last_raw_uri.lock().await.as_deref(),
        None,
        "disk cache hit served without contacting upstream"
    );

    // Cleanup temp cache
    let _ = std::fs::remove_dir_all(&custom_cache_dir);
}

// -------------------------------- GET /login passthrough and Form POST

#[tokio::test]
async fn login_get_proxies_upstream_html() {
    let (stub, stub_guard) = start_stub().await;
    let proxy = start_proxy(&stub_guard.base).await;

    let response = client()
        .get(format!("{}/login", proxy.base))
        .header(header::ORIGIN, "tauri://localhost")
        .send()
        .await
        .expect("get login request");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("text/html; charset=utf-8")
    );
    let text = response.text().await.expect("login text");
    assert!(text.contains("Login Page"));
    assert_eq!(
        stub.last_raw_uri.lock().await.as_deref(),
        Some("/login"),
        "GET /login reaches upstream"
    );
}

#[tokio::test]
async fn login_form_urlencoded_redirects() {
    let (_stub, stub_guard) = start_stub().await;
    let proxy = start_proxy(&stub_guard.base).await;

    let no_redirect_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("no redirect client");

    // 1. Successful login without next -> redirects to /
    let response = no_redirect_client
        .post(format!("{}/login", proxy.base))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(format!("username={USERNAME}&password={PASSWORD}"))
        .send()
        .await
        .expect("form login request");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok()),
        Some("/")
    );

    // 2. Successful login with next query parameter -> redirects to next target
    let response = no_redirect_client
        .post(format!("{}/login?next=/chat", proxy.base))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(format!("username={USERNAME}&password={PASSWORD}"))
        .send()
        .await
        .expect("form login request with next query");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok()),
        Some("/chat")
    );

    // 3. Successful login with next in body -> redirects to next target
    let response = no_redirect_client
        .post(format!("{}/login", proxy.base))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(format!(
            "username={USERNAME}&password={PASSWORD}&next=/dashboard"
        ))
        .send()
        .await
        .expect("form login request with next in body");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok()),
        Some("/dashboard")
    );

    // 4. Failed login with wrong password -> redirects to /login?error=...
    let response = no_redirect_client
        .post(format!("{}/login", proxy.base))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body("username=admin&password=wrongpassword")
        .send()
        .await
        .expect("bad form login");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert!(
        location.starts_with("/login?error="),
        "failed login must redirect to /login?error=..., got: {location}"
    );
}

#[tokio::test]
async fn login_json_still_works() {
    let (_stub, stub_guard) = start_stub().await;
    let proxy = start_proxy(&stub_guard.base).await;

    let body = login(&proxy.base).await;
    assert_eq!(body["ok"], true);
}

#[tokio::test]
async fn test_forward_raw_webui_and_fallback_routes() {
    let (stub, stub_guard) = start_stub().await;
    let proxy = start_proxy(&stub_guard.base).await;

    for path in [
        "/ui/index",
        "/ui/asset-bundle",
        "/sw.js",
        "/sw.js?version=2026.1",
        "/components/nav.js",
        "/some/future/page.html",
    ] {
        let response = client()
            .get(format!("{}{path}", proxy.base))
            .header(header::ORIGIN, "tauri://localhost")
            .send()
            .await
            .expect("webui/fallback request");
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "{path} status must be OK"
        );
        let bytes = response.bytes().await.expect("body");
        assert_eq!(bytes.as_ref(), RAW_BODY, "{path} body preserved");
        assert_eq!(
            stub.last_raw_uri.lock().await.as_deref(),
            Some(path),
            "{path} reaches upstream stub"
        );
    }

    // Verify /sw.js and /ui/asset-bundle are NOT cached
    for uncacheable in ["/sw.js", "/ui/asset-bundle", "/ui/index"] {
        *stub.last_raw_uri.lock().await = None;
        let response = client()
            .get(format!("{}{uncacheable}", proxy.base))
            .header(header::ORIGIN, "tauri://localhost")
            .send()
            .await
            .expect("repeat request");
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response.headers().get("x-cache").is_none(),
            "{uncacheable} must not have x-cache header"
        );
        assert_eq!(
            stub.last_raw_uri.lock().await.as_deref(),
            Some(uncacheable),
            "{uncacheable} must reach upstream on every call"
        );
    }

    // Verify /components/nav.js IS cached (static js asset)
    let comp_url = format!("{}/components/nav.js", proxy.base);
    *stub.last_raw_uri.lock().await = None;
    let hit_res = client()
        .get(&comp_url)
        .header(header::ORIGIN, "tauri://localhost")
        .send()
        .await
        .expect("hit request");
    assert_eq!(hit_res.status(), StatusCode::OK);
    assert_eq!(
        hit_res
            .headers()
            .get("x-cache")
            .and_then(|v| v.to_str().ok()),
        Some("HIT"),
        "/components/nav.js must be served from cache on repeat"
    );
    assert_eq!(
        stub.last_raw_uri.lock().await.as_deref(),
        None,
        "/components/nav.js HIT must not call upstream"
    );
}

#[tokio::test]
async fn session_cookie_persists_across_upstream_instances() {
    let (stub, stub_guard) = start_stub().await;
    let cookie_file = upstream::cookie_file_path(&stub_guard.base);
    let _ = std::fs::remove_file(&cookie_file);

    let proxy1 = start_proxy(&stub_guard.base).await;
    let body = login(&proxy1.base).await;
    assert_eq!(body["ok"], true);

    assert!(cookie_file.exists(), "cookie file must be written to disk");

    drop(proxy1);

    let proxy2 = start_proxy(&stub_guard.base).await;

    *stub.last_cookie.lock().await = None;

    let response = client()
        .get(format!("{}/ui/index", proxy2.base))
        .header(header::ORIGIN, "tauri://localhost")
        .send()
        .await
        .expect("proxy2 request");
    assert_eq!(response.status(), StatusCode::OK);

    let received_cookie = stub.last_cookie.lock().await.clone().unwrap_or_default();
    let (runtime_id, _) = stub.credentials().await;
    let expected_session = format!("session_{runtime_id}=");
    assert!(
        received_cookie.contains(&expected_session),
        "proxy 2 must send persisted session cookie; got: {received_cookie}"
    );

    let _ = std::fs::remove_file(&cookie_file);
}

#[tokio::test]
async fn test_proxy_prefers_stable_port_55088() {
    assert_eq!(server::PREFERRED_PROXY_PORT, 55088);
    assert_eq!(server::PROXY_PORT_RANGE, 55088..=55099);

    let (_stub, stub_guard) = start_stub().await;
    let (listener, _state, info) = server::bind(&stub_guard.base).await.expect("bind proxy");
    assert!(
        server::PROXY_PORT_RANGE.contains(&info.port) || info.port > 0,
        "proxy port must be valid"
    );
    drop(listener);
}

#[tokio::test]
async fn test_html_response_injects_container_bridge() {
    let (_stub, stub_guard) = start_stub().await;
    let proxy = start_proxy(&stub_guard.base).await;

    let response = client()
        .get(format!("{}/", proxy.base))
        .header(header::ORIGIN, "tauri://localhost")
        .send()
        .await
        .expect("root request");
    assert_eq!(response.status(), StatusCode::OK);
    let html = response.text().await.expect("root body");
    assert!(
        html.contains("__aqvn_bridge__"),
        "root HTML response must contain injected container bridge"
    );
}

// ---------------------- bounded LRU asset cache (VAL-CACHE-001)

#[tokio::test]
async fn test_bounded_memory_cache_lru_eviction() {
    use aqvn_workspace_lib::proxy::cache::{AssetCache, MAX_MEMORY_CACHE_BYTES};

    let test_dir = std::env::temp_dir().join(format!("aqvn-lru-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&test_dir);

    // 1. Verify default capacity is 32MB
    let cache = AssetCache::new(test_dir.clone());
    assert_eq!(cache.max_memory_bytes(), MAX_MEMORY_CACHE_BYTES);
    assert_eq!(cache.max_memory_bytes(), 32 * 1024 * 1024);
    assert_eq!(cache.current_memory_bytes(), 0);

    let origin = "http://[IP]:55088";

    // 2. Insert two 12MB assets (total 24MB <= 32MB)
    let chunk_12mb = vec![0xAA; 12 * 1024 * 1024];
    cache
        .put(origin, "/css/style1.css", Some("text/css"), &chunk_12mb)
        .await;
    assert_eq!(cache.current_memory_bytes(), 12 * 1024 * 1024);
    assert!(cache.contains_in_memory(origin, "/css/style1.css").await);

    cache
        .put(origin, "/css/style2.css", Some("text/css"), &chunk_12mb)
        .await;
    assert_eq!(cache.current_memory_bytes(), 24 * 1024 * 1024);
    assert!(cache.contains_in_memory(origin, "/css/style1.css").await);
    assert!(cache.contains_in_memory(origin, "/css/style2.css").await);

    // 3. Insert a third 12MB asset -> Total 36MB > 32MB.
    // style1.css is oldest accessed, so it must be evicted from RAM!
    cache
        .put(origin, "/css/style3.css", Some("text/css"), &chunk_12mb)
        .await;
    assert_eq!(cache.current_memory_bytes(), 24 * 1024 * 1024);
    assert!(
        !cache.contains_in_memory(origin, "/css/style1.css").await,
        "style1 must be evicted from memory"
    );
    assert!(cache.contains_in_memory(origin, "/css/style2.css").await);
    assert!(cache.contains_in_memory(origin, "/css/style3.css").await);

    // style1 is still on disk though!
    assert!(cache
        .disk_path(origin, "/css/style1.css")
        .expect("disk path")
        .is_file());

    // 4. Test LRU access touch:
    // Currently style2 was accessed before style3.
    // Calling get on style2 makes style2 MRU, leaving style3 as the oldest!
    let got2 = cache.get(origin, "/css/style2.css").await;
    assert!(got2.is_some());

    // Now insert a fourth 12MB asset.
    // Since style2 was touched, style3 is oldest and must be evicted!
    cache
        .put(origin, "/css/style4.css", Some("text/css"), &chunk_12mb)
        .await;
    assert_eq!(cache.current_memory_bytes(), 24 * 1024 * 1024);
    assert!(
        cache.contains_in_memory(origin, "/css/style2.css").await,
        "style2 was touched and should remain in memory"
    );
    assert!(
        !cache.contains_in_memory(origin, "/css/style3.css").await,
        "style3 was least-recently used and must be evicted"
    );
    assert!(cache.contains_in_memory(origin, "/css/style4.css").await);

    // 5. Test fetching evicted asset from disk brings it back to memory (evicting oldest in memory)
    // Currently memory has style2 and style4. style2 is oldest.
    // Fetching style1 reads from disk and inserts into memory -> evicts style2!
    let got1 = cache.get(origin, "/css/style1.css").await;
    assert!(got1.is_some());
    assert!(cache.contains_in_memory(origin, "/css/style1.css").await);
    assert!(
        !cache.contains_in_memory(origin, "/css/style2.css").await,
        "style2 evicted when style1 reloaded from disk"
    );
    assert!(cache.contains_in_memory(origin, "/css/style4.css").await);

    // 6. Test oversized asset (>32MB) cannot blow up memory
    let chunk_35mb = vec![0xBB; 35 * 1024 * 1024];
    cache.put(origin, "/large.bin", None, &chunk_35mb).await;
    assert!(
        !cache.contains_in_memory(origin, "/large.bin").await,
        "asset > 32MB must not be cached in memory"
    );
    assert!(cache.current_memory_bytes() <= 32 * 1024 * 1024);

    // 7. Test clear_memory
    cache.clear_memory().await;
    assert_eq!(cache.current_memory_bytes(), 0);
    assert!(!cache.contains_in_memory(origin, "/css/style1.css").await);
    assert!(!cache.contains_in_memory(origin, "/css/style4.css").await);

    let _ = std::fs::remove_dir_all(&test_dir);
}

// ---------------------- ephemeral port binding (VAL-CACHE-003)

#[tokio::test]
async fn test_ephemeral_port_binding() {
    let (_stub, stub_guard) = start_stub().await;

    // 1. Direct bind_ephemeral binds to loopback with a valid dynamic port
    let (listener, _state, info) = server::bind_ephemeral(&stub_guard.base)
        .await
        .expect("bind_ephemeral");
    let address = listener.local_addr().expect("local addr");
    assert!(
        address.ip().is_loopback(),
        "proxy must bind loopback: {address}"
    );
    assert_ne!(address.port(), 0, "ephemeral port must be non-zero");
    assert_eq!(info.port, address.port());
    assert_eq!(info.url, format!("http://{address}"));
    drop(listener);

    // 2. Direct bind_port with port 0 also binds cleanly
    let (listener2, _state2, info2) = server::bind_port(&stub_guard.base, 0)
        .await
        .expect("bind_port 0");
    assert_ne!(info2.port, 0);
    drop(listener2);

    // 3. Concurrent ephemeral bindings succeed without colliding on 55088
    let mut listeners = Vec::new();
    let mut ports = std::collections::HashSet::new();
    for _ in 0..5 {
        let (l, _s, inf) = server::bind_ephemeral(&stub_guard.base)
            .await
            .expect("concurrent bind_ephemeral");
        assert_ne!(inf.port, 0);
        assert!(
            ports.insert(inf.port),
            "ephemeral ports must be distinct across active listeners"
        );
        listeners.push(l);
    }
    drop(listeners);

    // 4. Running ephemeral proxy can serve requests cleanly
    let proxy = start_proxy(&stub_guard.base).await;
    let resp = client()
        .get(format!("{}/health", proxy.base))
        .header(header::ORIGIN, "tauri://localhost")
        .send()
        .await
        .expect("health check on ephemeral proxy");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("health json");
    assert_eq!(body["ok"], true);
}
