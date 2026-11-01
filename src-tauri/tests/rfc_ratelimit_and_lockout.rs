//! Validation test for inbound API rate limiting and SSH authentication lockout.
//!
//! Fulfills assertion VAL-SEC-005:
//! - `src-tauri/src/rfc/server.rs` implements request rate limiting returning
//!   HTTP 429 when burst requests exceed threshold (>60 req/min).
//! - `src-tauri/src/ssh/server.rs` enforces consecutive failed password attempt lockout (5 attempts).

use aqvn_workspace_lib::config::AppConfig;
use aqvn_workspace_lib::rfc::{auth, server as rfc_server};
use aqvn_workspace_lib::ssh::server as ssh_server;
use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use russh::client;
use serde_json::json;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

const RFC_PASSWORD: &str = "test-rfc-pass";
const SSH_PASSWORD: &str = "test-ssh-pass";
const WORKED_INPUT: &str = r#"{"module": "helpers.rfc_files", "function_name": "_read_file_binary_impl", "args": ["/test.txt"], "kwargs": {}}"#;

struct TempDir {
    dir: PathBuf,
}

impl TempDir {
    fn new(prefix: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("aqvn-{prefix}-{unique}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        Self { dir }
    }

    fn path(&self) -> &Path {
        &self.dir
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn rfc_app(allowed_path: &Path) -> Router {
    let mut config = AppConfig {
        rfc_enabled: true,
        allowed_paths: vec![allowed_path.to_string_lossy().into_owned()],
        rfc_port: 0,
        ..Default::default()
    };
    config.rfc_password.push_str(RFC_PASSWORD);
    rfc_server::router(rfc_server::build_state(&config))
}

fn make_signed_request(rfc_input: &str, client_ip: Option<&str>) -> Request<Body> {
    let hash = auth::compute_hash(RFC_PASSWORD, rfc_input);
    let payload = json!({
        "rfc_input": rfc_input,
        "hash": hash,
    });
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri("/api/rfc")
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(ip) = client_ip {
        builder = builder.header("x-forwarded-for", ip);
    }
    builder
        .body(Body::from(payload.to_string()))
        .expect("request")
}

// ---------------------------------------------------------------- RFC Rate Limit Tests

#[tokio::test]
async fn test_rfc_burst_rate_limiting_returns_429() {
    let temp = TempDir::new("rfc-ratelimit");
    let app = rfc_app(temp.path());

    // Send 60 requests in quick succession within 1 minute window.
    for i in 0..60 {
        let req = make_signed_request(WORKED_INPUT, None);
        let resp = app.clone().oneshot(req).await.expect("call");
        assert_ne!(
            resp.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "Request #{i} within burst limit of 60 should not be throttled (got {})",
            resp.status()
        );
    }

    // 61st request (>60 req/min) MUST be throttled with HTTP 429 Too Many Requests.
    let req61 = make_signed_request(WORKED_INPUT, None);
    let resp61 = app.clone().oneshot(req61).await.expect("call");
    assert_eq!(
        resp61.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "61st request should be throttled returning HTTP 429"
    );

    let bytes = resp61
        .into_body()
        .collect()
        .await
        .expect("collect")
        .to_bytes();
    let body_str = String::from_utf8_lossy(&bytes);
    assert!(
        body_str.to_lowercase().contains("too many requests") || body_str.contains("429"),
        "Response body should explain rate limit exceeded: {body_str}"
    );
}

#[tokio::test]
async fn test_rfc_rate_limiting_per_ip_isolation() {
    let temp = TempDir::new("rfc-ip-isolation");
    let app = rfc_app(temp.path());

    // Exhaust rate limit for IP 192.168.1.10
    for _ in 0..60 {
        let req = make_signed_request(WORKED_INPUT, Some("192.168.1.10"));
        let resp = app.clone().oneshot(req).await.expect("call");
        assert_ne!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    // IP 192.168.1.10 is now throttled
    let req_throttled = make_signed_request(WORKED_INPUT, Some("192.168.1.10"));
    let resp_throttled = app.clone().oneshot(req_throttled).await.expect("call");
    assert_eq!(resp_throttled.status(), StatusCode::TOO_MANY_REQUESTS);

    // IP 192.168.1.20 has fresh quota and is NOT throttled
    let req_fresh = make_signed_request(WORKED_INPUT, Some("192.168.1.20"));
    let resp_fresh = app.clone().oneshot(req_fresh).await.expect("call");
    assert_ne!(resp_fresh.status(), StatusCode::TOO_MANY_REQUESTS);
}

// ---------------------------------------------------------------- SSH Lockout Tests

#[derive(Clone)]
struct MockClientHandler;

#[async_trait::async_trait]
impl client::Handler for MockClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

async fn wait_until_listening(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(10);
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    while Instant::now() < deadline {
        if tokio::net::TcpStream::connect(addr).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("SSH server did not start on port {port}");
}

#[allow(clippy::field_reassign_with_default)]
fn ssh_config(temp_path: &Path) -> AppConfig {
    let mut config = AppConfig {
        rfc_enabled: true,
        ssh_port: 0,
        allowed_paths: vec![temp_path.to_string_lossy().into_owned()],
        ..Default::default()
    };
    config.ssh_password.push_str(SSH_PASSWORD);
    config
}

#[tokio::test]
async fn test_ssh_lockout_after_5_failed_password_attempts() {
    let temp = TempDir::new("ssh-lockout");
    let config = ssh_config(temp.path());

    let listener = ssh_server::bind_if_enabled(&config)
        .await
        .expect("bind")
        .expect("listener");
    let port = listener.local_addr().expect("port").port();

    let server_task = tokio::spawn(async move {
        let _ = ssh_server::serve(listener, config).await;
    });
    wait_until_listening(port).await;

    // 5 consecutive failed attempts
    for attempt in 1..=5 {
        let mut client = client::connect(
            Arc::new(client::Config::default()),
            (IpAddr::V4(Ipv4Addr::LOCALHOST), port),
            MockClientHandler,
        )
        .await
        .expect("connect");

        let accepted = client
            .authenticate_password("root", format!("wrong-password-{attempt}"))
            .await
            .expect("auth");
        assert!(!accepted, "Failed attempt #{attempt} must be rejected");
    }

    // 6th attempt with CORRECT password: MUST be rejected due to lockout!
    let mut client6 = client::connect(
        Arc::new(client::Config::default()),
        (IpAddr::V4(Ipv4Addr::LOCALHOST), port),
        MockClientHandler,
    )
    .await
    .expect("connect");

    let accepted = client6
        .authenticate_password("root", SSH_PASSWORD)
        .await
        .expect("auth");
    assert!(
        !accepted,
        "Authentication with valid password MUST be rejected after 5 consecutive failed attempts (lockout active)"
    );

    server_task.abort();
}

#[tokio::test]
async fn test_ssh_consecutive_counter_resets_on_success() {
    let temp = TempDir::new("ssh-reset");
    let config = ssh_config(temp.path());

    let listener = ssh_server::bind_if_enabled(&config)
        .await
        .expect("bind")
        .expect("listener");
    let port = listener.local_addr().expect("port").port();

    let server_task = tokio::spawn(async move {
        let _ = ssh_server::serve(listener, config).await;
    });
    wait_until_listening(port).await;

    // 3 failed attempts (less than 5)
    for attempt in 1..=3 {
        let mut client = client::connect(
            Arc::new(client::Config::default()),
            (IpAddr::V4(Ipv4Addr::LOCALHOST), port),
            MockClientHandler,
        )
        .await
        .expect("connect");

        let accepted = client
            .authenticate_password("root", format!("wrong-pass-{attempt}"))
            .await
            .expect("auth");
        assert!(!accepted);
    }

    // Successful attempt resets the consecutive counter
    let mut good_client = client::connect(
        Arc::new(client::Config::default()),
        (IpAddr::V4(Ipv4Addr::LOCALHOST), port),
        MockClientHandler,
    )
    .await
    .expect("connect");

    let accepted = good_client
        .authenticate_password("root", SSH_PASSWORD)
        .await
        .expect("auth");
    assert!(accepted, "Valid password should authenticate");

    // 3 more failed attempts (total 6 failures, but consecutive is only 3)
    for attempt in 1..=3 {
        let mut client = client::connect(
            Arc::new(client::Config::default()),
            (IpAddr::V4(Ipv4Addr::LOCALHOST), port),
            MockClientHandler,
        )
        .await
        .expect("connect");

        let accepted = client
            .authenticate_password("root", format!("wrong-pass-second-{attempt}"))
            .await
            .expect("auth");
        assert!(!accepted);
    }

    // Next login with correct password should STILL succeed because consecutive count was reset!
    let mut good_client2 = client::connect(
        Arc::new(client::Config::default()),
        (IpAddr::V4(Ipv4Addr::LOCALHOST), port),
        MockClientHandler,
    )
    .await
    .expect("connect");

    let accepted2 = good_client2
        .authenticate_password("root", SSH_PASSWORD)
        .await
        .expect("auth");
    assert!(
        accepted2,
        "Authentication should succeed because consecutive counter was reset by earlier success"
    );

    server_task.abort();
}
