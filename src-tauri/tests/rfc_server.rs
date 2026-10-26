//! RFC server wire tests.
//!
//! Spec: missionDir `research/rfc-protocol.md` §§2–4 (request/auth/endpoint
//! semantics) and §7 (worked example, password `secret`).
//! Covers validation contract assertions VAL-RFC-001..008 and VAL-RFC-025.

use aqvn_workspace_lib::config::AppConfig;
use aqvn_workspace_lib::rfc::{auth, server};
use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

/// §7 worked example: exact bytes the client signs (Python `json.dumps` defaults).
const WORKED_INPUT: &str = r#"{"module": "helpers.rfc_files", "function_name": "_read_file_binary_impl", "args": ["/a0/usr/workdir/test.txt"], "kwargs": {}}"#;
/// §7 precomputed HMAC-SHA256 of `WORKED_INPUT` with password `secret`.
const WORKED_HASH: &str = "b3baa5df8d06d6fe2418d020ff8000b3060db4026ebd2fae45eee854db379694";
const RFC_PATH: &str = "/api/rfc";
const TEST_PASSWORD: &str = "secret";

struct TempSandbox {
    dir: PathBuf,
}

impl TempSandbox {
    fn new(files: &[(&str, &[u8])]) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("aqvn-rfc-test-{}-{unique}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create sandbox root");
        for (relative, contents) in files {
            let path = dir.join(relative);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create sandbox parent");
            }
            std::fs::write(&path, contents).expect("write sandbox fixture");
        }
        Self { dir }
    }

    fn path(&self) -> &Path {
        &self.dir
    }
}

impl Drop for TempSandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn config_for(sandbox: &Path) -> AppConfig {
    let mut config = AppConfig {
        rfc_enabled: true,
        allowed_paths: vec![sandbox.to_string_lossy().into_owned()],
        rfc_port: 0,
        ..Default::default()
    };
    config.rfc_password.push_str(TEST_PASSWORD);
    config
}

fn app_for(sandbox: &Path) -> Router {
    server::router(server::build_state(&config_for(sandbox)))
}

struct HttpResponse {
    status: StatusCode,
    content_type: String,
    body: String,
}

impl HttpResponse {
    fn json_body(&self) -> Value {
        serde_json::from_str(&self.body).expect("response body is JSON")
    }
}

async fn send(app: &Router, request: Request<Body>) -> HttpResponse {
    let response = app.clone().oneshot(request).await.expect("router call");
    let status = response.status();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|value| value.to_str().expect("utf8 content type").to_string())
        .unwrap_or_default();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    HttpResponse {
        status,
        content_type,
        body: String::from_utf8_lossy(&bytes).to_string(),
    }
}

fn post_envelope(rfc_input: &str, hash: &str) -> Request<Body> {
    post_json(&json!({ "rfc_input": rfc_input, "hash": hash }))
}

fn post_json(body: &Value) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(RFC_PATH)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request")
}

fn signed(input: &str) -> Request<Body> {
    post_envelope(input, &auth::compute_hash(TEST_PASSWORD, input))
}

// ---------------------------------------------------------------- auth (VAL-RFC-002)

#[test]
fn hmac_hash_matches_spec_worked_example() {
    assert_eq!(auth::compute_hash(TEST_PASSWORD, WORKED_INPUT), WORKED_HASH);
    assert!(auth::verify(TEST_PASSWORD, WORKED_INPUT, WORKED_HASH));
}

#[test]
fn auth_rejects_wrong_password_and_malformed_hashes() {
    assert!(!auth::verify("wrong", WORKED_INPUT, WORKED_HASH));
    // Uppercase hex is not the wire format (lowercase only).
    assert!(!auth::verify(
        TEST_PASSWORD,
        WORKED_INPUT,
        &WORKED_HASH.to_ascii_uppercase()
    ));
    // Wrong length / non-hex must be rejected without panicking.
    assert!(!auth::verify(TEST_PASSWORD, WORKED_INPUT, "abc"));
    assert!(!auth::verify(
        TEST_PASSWORD,
        WORKED_INPUT,
        &"z".repeat(WORKED_HASH.len())
    ));
    assert!(!auth::verify(TEST_PASSWORD, WORKED_INPUT, ""));
}

// ------------------------------------------------------ success path (VAL-RFC-001)

#[tokio::test]
async fn worked_example_returns_bare_base64_result() {
    let sandbox = TempSandbox::new(&[("usr/workdir/test.txt", b"hello rfc\n")]);
    let app = app_for(sandbox.path());

    let response = send(&app, post_envelope(WORKED_INPUT, WORKED_HASH)).await;

    assert_eq!(response.status, StatusCode::OK, "body: {}", response.body);
    assert!(
        response.content_type.starts_with("application/json"),
        "content type: {}",
        response.content_type
    );
    // Bare JSON string, no `{"result": ...}` envelope.
    assert_eq!(response.json_body(), json!("aGVsbG8gcmZjCg=="));
    assert!(response.body.starts_with('"'), "body: {}", response.body);
}

#[tokio::test]
async fn read_file_as_base64_matches_binary_read() {
    let sandbox = TempSandbox::new(&[("usr/workdir/test.txt", b"hello rfc\n")]);
    let app = app_for(sandbox.path());
    let input = r#"{"module": "helpers.rfc_files", "function_name": "_read_file_as_base64_impl", "args": ["/a0/usr/workdir/test.txt"], "kwargs": {}}"#;

    let response = send(&app, signed(input)).await;

    assert_eq!(response.status, StatusCode::OK, "body: {}", response.body);
    assert_eq!(response.json_body(), json!("aGVsbG8gcmZjCg=="));
}

#[tokio::test]
async fn auth_accepts_exact_received_bytes() {
    let sandbox = TempSandbox::new(&[]);
    let app = app_for(sandbox.path());
    // Compact separators + non-canonical kwargs order — hash covers these bytes.
    let input = r#"{"module":"helpers.rfc_files","function_name":"_path_exists_impl","args":["/a0"],"kwargs":{"z":1,"a":2}}"#;

    let response = send(&app, signed(input)).await;

    assert_eq!(response.status, StatusCode::OK, "body: {}", response.body);
    assert_eq!(response.json_body(), json!(true));
}

#[tokio::test]
async fn path_exists_reports_true_inside_sandbox() {
    let sandbox = TempSandbox::new(&[("note.txt", b"x")]);
    let app = app_for(sandbox.path());
    let input = r#"{"module": "helpers.rfc_files", "function_name": "_path_exists_impl", "args": ["/a0/note.txt"], "kwargs": {}}"#;

    let response = send(&app, signed(input)).await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.json_body(), json!(true));
}

// -------------------------------------------------- auth failures (VAL-RFC-003/004)

#[tokio::test]
async fn wrong_hash_403_text_plain() {
    let sandbox = TempSandbox::new(&[]);
    let app = app_for(sandbox.path());
    let input = r#"{"module": "helpers.rfc_files", "function_name": "_path_exists_impl", "args": ["/a0"], "kwargs": {}}"#;
    let wrong = "0".repeat(64);

    let response = send(&app, post_envelope(input, &wrong)).await;

    assert_eq!(response.status, StatusCode::FORBIDDEN);
    assert!(
        response.content_type.starts_with("text/plain"),
        "content type: {}",
        response.content_type
    );
    assert!(
        !response.body.trim().is_empty(),
        "informative body required"
    );
}

#[tokio::test]
async fn missing_hash_403() {
    let sandbox = TempSandbox::new(&[]);
    let app = app_for(sandbox.path());
    let body = json!({ "rfc_input": WORKED_INPUT });

    let response = send(&app, post_json(&body)).await;

    assert_eq!(response.status, StatusCode::FORBIDDEN);
    assert!(response.content_type.starts_with("text/plain"));
}

#[tokio::test]
async fn empty_hash_403() {
    let sandbox = TempSandbox::new(&[]);
    let app = app_for(sandbox.path());

    let response = send(&app, post_envelope(WORKED_INPUT, "")).await;

    assert_eq!(response.status, StatusCode::FORBIDDEN);
    assert!(response.content_type.starts_with("text/plain"));
}

#[tokio::test]
async fn uppercase_hash_403() {
    let sandbox = TempSandbox::new(&[("usr/workdir/test.txt", b"hello rfc\n")]);
    let app = app_for(sandbox.path());

    let response = send(
        &app,
        post_envelope(WORKED_INPUT, &WORKED_HASH.to_ascii_uppercase()),
    )
    .await;

    assert_eq!(response.status, StatusCode::FORBIDDEN);
}

// ------------------------------------------------ malformed input (VAL-RFC-005)

#[tokio::test]
async fn unparseable_rfc_input_500_then_server_still_serves() {
    let sandbox = TempSandbox::new(&[("usr/workdir/test.txt", b"hello rfc\n")]);
    let app = app_for(sandbox.path());
    let broken = "not-json{";

    let response = send(&app, signed(broken)).await;

    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        response.content_type.starts_with("text/plain"),
        "content type: {}",
        response.content_type
    );
    assert!(!response.body.trim().is_empty());

    let follow_up = send(&app, post_envelope(WORKED_INPUT, WORKED_HASH)).await;
    assert_eq!(follow_up.status, StatusCode::OK);
}

#[tokio::test]
async fn rfc_input_missing_module_500() {
    let sandbox = TempSandbox::new(&[]);
    let app = app_for(sandbox.path());
    let input = r#"{"function_name": "_path_exists_impl", "args": ["/a0"], "kwargs": {}}"#;

    let response = send(&app, signed(input)).await;

    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(response.content_type.starts_with("text/plain"));
}

// ------------------------------------------------- method/path (VAL-RFC-006)

#[tokio::test]
async fn non_post_verbs_405_exact_bodies() {
    let sandbox = TempSandbox::new(&[]);
    let app = app_for(sandbox.path());

    for verb in [Method::GET, Method::PUT, Method::DELETE] {
        let request = Request::builder()
            .method(verb.clone())
            .uri(RFC_PATH)
            .body(Body::empty())
            .expect("build request");
        let response = send(&app, request).await;

        assert_eq!(
            response.status,
            StatusCode::METHOD_NOT_ALLOWED,
            "verb {verb}"
        );
        assert!(
            response.content_type.starts_with("text/html"),
            "verb {verb} content type: {}",
            response.content_type
        );
        assert_eq!(response.body, format!("Method {verb} not allowed for: rfc"));
    }
}

#[tokio::test]
async fn unknown_paths_404_exact_body() {
    let sandbox = TempSandbox::new(&[]);
    let app = app_for(sandbox.path());

    for verb in [Method::GET, Method::POST] {
        let request = Request::builder()
            .method(verb.clone())
            .uri("/api/nonexistent")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from("{}"))
            .expect("build request");
        let response = send(&app, request).await;

        assert_eq!(response.status, StatusCode::NOT_FOUND, "verb {verb}");
        assert!(
            response.content_type.starts_with("text/html"),
            "verb {verb} content type: {}",
            response.content_type
        );
        assert_eq!(response.body, "API endpoint not found: nonexistent");
    }
}

// ------------------------------------------------- whitelist (VAL-RFC-007/008)

#[tokio::test]
async fn unknown_module_403() {
    let sandbox = TempSandbox::new(&[]);
    let app = app_for(sandbox.path());
    let input = r#"{"module": "os", "function_name": "system", "args": ["id"], "kwargs": {}}"#;

    let response = send(&app, signed(input)).await;

    assert_eq!(response.status, StatusCode::FORBIDDEN);
    assert!(response.content_type.starts_with("text/plain"));
    assert!(!response.body.trim().is_empty());
}

#[tokio::test]
async fn unknown_function_in_whitelisted_module_403() {
    let sandbox = TempSandbox::new(&[]);
    let app = app_for(sandbox.path());
    let input = r#"{"module": "helpers.rfc_files", "function_name": "_not_a_real_fn_impl", "args": [], "kwargs": {}}"#;

    let response = send(&app, signed(input)).await;

    assert_eq!(response.status, StatusCode::FORBIDDEN);
    assert!(response.content_type.starts_with("text/plain"));
}

// ------------------------------------------------------ listener (VAL-RFC-025)

#[tokio::test]
async fn master_toggle_off_binds_no_socket() {
    let config = AppConfig::default();
    assert!(!config.rfc_enabled, "default must be OFF");

    let listener = server::bind_if_enabled(&config)
        .await
        .expect("disabled server must not error");

    assert!(
        listener.is_none(),
        "disabled master toggle must bind nothing"
    );
}

#[tokio::test]
async fn enabled_server_binds_loopback_ephemeral_port() {
    let config = config_for(std::env::temp_dir().as_path());

    let listener = server::bind_if_enabled(&config)
        .await
        .expect("bind must succeed")
        .expect("enabled server must bind");

    let addr = listener.local_addr().expect("local addr");
    assert!(addr.ip().is_loopback(), "must bind loopback only: {addr}");
    assert_ne!(addr.port(), 0, "ephemeral bind must resolve a real port");
}

#[tokio::test]
async fn serves_http_over_tcp() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let sandbox = TempSandbox::new(&[("usr/workdir/test.txt", b"hello rfc\n")]);
    let config = config_for(sandbox.path());
    let listener = server::bind_if_enabled(&config)
        .await
        .expect("bind")
        .expect("enabled");
    let addr = listener.local_addr().expect("addr");
    let state = server::build_state(&config);
    let task = tokio::spawn(async move { server::serve(listener, state).await });

    let body = json!({ "rfc_input": WORKED_INPUT, "hash": WORKED_HASH }).to_string();
    let request = format!(
        "POST {RFC_PATH} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );

    let mut stream = tokio::net::TcpStream::connect(addr).await.expect("connect");
    stream
        .write_all(request.as_bytes())
        .await
        .expect("write request");
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.expect("read response");
    let raw = String::from_utf8_lossy(&raw).to_string();

    task.abort();

    assert!(
        raw.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {raw}"
    );
    assert!(
        raw.contains("\"aGVsbG8gcmZjCg==\""),
        "unexpected body: {raw}"
    );
}
