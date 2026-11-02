//! RFC server core and supervisor auto-recovery tests.
//!
//! Covers validation contract assertion VAL-RES-001:
//! When the RFC server experiences a transient listener or socket error,
//! the supervisor logs the failure, backs off exponentially, and restarts
//! without exiting the background task.

use aqvn_workspace_lib::config::{AppConfig, ConfigWatcher};
use aqvn_workspace_lib::rfc::{auth, server};
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{header, Method, Request, StatusCode};
use serde_json::json;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

const TEST_PASSWORD: &str = "secret-supervisor-test";
const WORKED_INPUT: &str = r#"{"module": "helpers.rfc_files", "function_name": "_path_exists_impl", "args": ["/a0/test.txt"], "kwargs": {}}"#;

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
            std::env::temp_dir().join(format!("aqvn-rfc-core-{}-{unique}", std::process::id()));
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

fn config_for(sandbox: &Path, port: u16) -> AppConfig {
    let mut config = AppConfig {
        rfc_enabled: true,
        allowed_paths: vec![sandbox.to_string_lossy().into_owned()],
        rfc_port: port,
        ..Default::default()
    };
    config.rfc_password.push_str(TEST_PASSWORD);
    config
}

async fn wait_for_http_200(port: u16, timeout: Duration) -> bool {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let deadline = Instant::now() + timeout;
    let local = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let hash = auth::compute_hash(TEST_PASSWORD, WORKED_INPUT);
    let body = json!({ "rfc_input": WORKED_INPUT, "hash": hash }).to_string();
    let request = format!(
        "POST /api/rfc HTTP/1.1\r\nHost: {local}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );

    while Instant::now() < deadline {
        if let Ok(mut stream) = tokio::net::TcpStream::connect(local).await {
            if stream.write_all(request.as_bytes()).await.is_ok() {
                let mut resp = Vec::new();
                if stream.read_to_end(&mut resp).await.is_ok() {
                    let text = String::from_utf8_lossy(&resp);
                    if text.starts_with("HTTP/1.1 200 OK") {
                        return true;
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

#[test]
fn val_res_001_exponential_backoff_progression() {
    let min = server::INITIAL_BACKOFF;
    let max = server::MAX_BACKOFF;
    let factor = server::BACKOFF_FACTOR;

    assert_eq!(min, Duration::from_secs(1));
    assert_eq!(max, Duration::from_secs(10));
    assert_eq!(factor, 2.0);

    let step1 = server::compute_backoff(min, factor, max);
    assert_eq!(step1, Duration::from_secs(2));

    let step2 = server::compute_backoff(step1, factor, max);
    assert_eq!(step2, Duration::from_secs(4));

    let step3 = server::compute_backoff(step2, factor, max);
    assert_eq!(step3, Duration::from_secs(8));

    let step4 = server::compute_backoff(step3, factor, max);
    assert_eq!(step4, Duration::from_secs(10));

    let step5 = server::compute_backoff(step4, factor, max);
    assert_eq!(step5, Duration::from_secs(10));
}

#[tokio::test]
async fn val_res_001_rfc_supervisor_auto_recovers_on_transient_listener_error() {
    let sandbox = TempSandbox::new(&[("test.txt", b"core content")]);

    // Bind an ephemeral port so it is initially occupied and unavailable to the RFC supervisor.
    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind probe");
    let busy_port = probe.local_addr().expect("probe addr").port();

    let config = config_for(sandbox.path(), busy_port);
    let watcher = ConfigWatcher::new(config);
    let sup_watcher = watcher.clone();

    // Launch supervisor with fast backoff for testing.
    let supervisor = tokio::spawn(async move {
        server::run_supervised_with_backoff(
            sup_watcher,
            Duration::from_millis(50),
            Duration::from_millis(300),
        )
        .await
    });

    // Let the supervisor attempt to bind and fail, entering backoff retry loop.
    tokio::time::sleep(Duration::from_millis(120)).await;

    // Verify supervisor did NOT terminate on bind error!
    assert!(
        !supervisor.is_finished(),
        "supervisor background task must survive transient socket bind error"
    );

    // Release the port: now the supervisor can bind on its next retry.
    drop(probe);

    // Wait for the supervisor to recover, bind, and serve requests.
    let recovered = wait_for_http_200(busy_port, Duration::from_secs(5)).await;
    assert!(
        recovered,
        "RFC server should auto-recover and serve requests after port becomes available"
    );

    // Verify supervisor is still healthy and running after recovery.
    assert!(
        !supervisor.is_finished(),
        "supervisor must remain active after recovering from socket error"
    );

    supervisor.abort();
}

#[tokio::test]
async fn val_res_001_rfc_supervisor_default_loop_does_not_exit_on_error() {
    let sandbox = TempSandbox::new(&[("test.txt", b"core content")]);

    // Bind ephemeral port
    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind probe");
    let busy_port = probe.local_addr().expect("probe addr").port();

    let config = config_for(sandbox.path(), busy_port);
    let watcher = ConfigWatcher::new(config);
    let sup_watcher = watcher.clone();

    // Run standard run_supervised (1s..10s backoff)
    let supervisor = tokio::spawn(async move { server::run_supervised(sup_watcher).await });

    // Wait 150ms while socket is busy
    tokio::time::sleep(Duration::from_millis(150)).await;

    assert!(
        !supervisor.is_finished(),
        "run_supervised must not exit on initial bind collision"
    );

    // Drop collision and allow standard 1s retry to bind
    drop(probe);

    let recovered = wait_for_http_200(busy_port, Duration::from_secs(5)).await;
    assert!(
        recovered,
        "run_supervised should recover within 1s exponential backoff window"
    );

    assert!(!supervisor.is_finished());
    supervisor.abort();
}

fn make_request_with_peer(
    rfc_input: &str,
    peer_addr: Option<SocketAddr>,
    forwarded_for: Option<&str>,
) -> Request<Body> {
    let hash = auth::compute_hash(TEST_PASSWORD, rfc_input);
    let payload = json!({
        "rfc_input": rfc_input,
        "hash": hash,
    });
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri("/api/rfc")
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(forwarded) = forwarded_for {
        builder = builder.header("x-forwarded-for", forwarded);
    }
    let mut req = builder
        .body(Body::from(payload.to_string()))
        .expect("request");
    if let Some(peer) = peer_addr {
        req.extensions_mut().insert(ConnectInfo(peer));
    }
    req
}

#[tokio::test]
async fn val_res_003_rate_limiter_trusted_reverse_proxy_x_forwarded_for() {
    let sandbox = TempSandbox::new(&[("test.txt", b"core content")]);
    let app = server::router(server::build_state(&config_for(sandbox.path(), 0)));

    // ConnectInfo is a trusted local reverse proxy ([IP])
    let proxy_peer = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);

    // 60 requests from client A behind proxy
    for _ in 0..60 {
        let req = make_request_with_peer(WORKED_INPUT, Some(proxy_peer), Some("198.51.100.1"));
        let resp = app.clone().oneshot(req).await.expect("call");
        assert_ne!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    // Client A is now throttled (61st request)
    let req_a_throttled =
        make_request_with_peer(WORKED_INPUT, Some(proxy_peer), Some("198.51.100.1"));
    let resp_a = app.clone().oneshot(req_a_throttled).await.expect("call");
    assert_eq!(resp_a.status(), StatusCode::TOO_MANY_REQUESTS);

    // Client B from same reverse proxy has distinct IP and must NOT be throttled
    let req_b = make_request_with_peer(WORKED_INPUT, Some(proxy_peer), Some("198.51.100.2"));
    let resp_b = app.clone().oneshot(req_b).await.expect("call");
    assert_ne!(
        resp_b.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "Client B behind trusted local reverse proxy should not be throttled by Client A's quota"
    );
}

#[tokio::test]
async fn val_res_003_rate_limiter_direct_external_connection_ignores_spoofed_x_forwarded_for() {
    let sandbox = TempSandbox::new(&[("test.txt", b"core content")]);
    let app = server::router(server::build_state(&config_for(sandbox.path(), 0)));

    // ConnectInfo is external untrusted peer ([IP])
    let external_peer = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 55)), 54321);

    // Attacker sends 60 requests spoofing different X-Forwarded-For headers
    for i in 0..60 {
        let spoofed_ip = format!("10.0.0.{}", i + 1);
        let req = make_request_with_peer(WORKED_INPUT, Some(external_peer), Some(&spoofed_ip));
        let resp = app.clone().oneshot(req).await.expect("call");
        assert_ne!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    // 61st request from external peer with another spoofed header MUST be throttled
    // because rate limiting binds strictly to the external socket peer IP!
    let req_spoof_throttled =
        make_request_with_peer(WORKED_INPUT, Some(external_peer), Some("10.0.0.61"));
    let resp = app
        .clone()
        .oneshot(req_spoof_throttled)
        .await
        .expect("call");
    assert_eq!(
        resp.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "Direct external requests must enforce rate limit on peer socket IP, ignoring spoofed X-Forwarded-For"
    );
}

#[tokio::test]
async fn val_res_003_rate_limiter_ipv6_loopback_trusted_proxy() {
    let sandbox = TempSandbox::new(&[("test.txt", b"core content")]);
    let app = server::router(server::build_state(&config_for(sandbox.path(), 0)));

    // ConnectInfo is IPv6 loopback (::1)
    let proxy_peer_v6 = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 8080);

    for _ in 0..60 {
        let req = make_request_with_peer(WORKED_INPUT, Some(proxy_peer_v6), Some("198.51.100.99"));
        let resp = app.clone().oneshot(req).await.expect("call");
        assert_ne!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    let req_throttled =
        make_request_with_peer(WORKED_INPUT, Some(proxy_peer_v6), Some("198.51.100.99"));
    let resp = app.clone().oneshot(req_throttled).await.expect("call");
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);

    let req_fresh =
        make_request_with_peer(WORKED_INPUT, Some(proxy_peer_v6), Some("198.51.100.100"));
    let resp_fresh = app.clone().oneshot(req_fresh).await.expect("call");
    assert_ne!(resp_fresh.status(), StatusCode::TOO_MANY_REQUESTS);
}
