//! Anti-replay sliding-window verification tests for `/api/rfc` (VAL-SEC-REPLAY-001).
//!
//! Validates:
//! - RFC payload envelope requires timestamp and nonce.
//! - Requests with identical nonce or timestamp drifting >300s are rejected with HTTP 401.
//! - Valid requests with fresh timestamp and unique nonce succeed with HTTP 200.

use aqvn_core::config::AppConfig;
use aqvn_rfc::{auth, server, AntiReplayWindow, ReplayError};
use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

const TEST_PASSWORD: &str = "test-anti-replay-secret";
const RFC_PATH: &str = "/api/rfc";
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
            std::env::temp_dir().join(format!("aqvn-rfc-replay-{}-{unique}", std::process::id()));
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
    body: String,
}

async fn send_request(app: &Router, request: Request<Body>) -> HttpResponse {
    let response = app.clone().oneshot(request).await.expect("router call");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    HttpResponse {
        status,
        body: String::from_utf8_lossy(&bytes).to_string(),
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time after epoch")
        .as_secs()
}

fn make_request(body: Value) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(RFC_PATH)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request")
}

fn signed_envelope(rfc_input: &str, timestamp: u64, nonce: &str) -> Value {
    let hash = auth::compute_hash(TEST_PASSWORD, rfc_input);
    json!({
        "rfc_input": rfc_input,
        "timestamp": timestamp,
        "nonce": nonce,
        "hash": hash,
    })
}

// -----------------------------------------------------------------------------
// VAL-SEC-REPLAY-001 Test Cases
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_replay_valid_request_with_fresh_timestamp_and_nonce() {
    let sandbox = TempSandbox::new(&[("test.txt", b"hello world")]);
    let app = app_for(sandbox.path());
    let ts = current_timestamp();
    let nonce = "nonce-valid-001";

    let resp = send_request(&app, make_request(signed_envelope(WORKED_INPUT, ts, nonce))).await;
    assert_eq!(resp.status, StatusCode::OK, "response body: {}", resp.body);
}

#[tokio::test]
async fn test_replay_rejects_identical_replayed_request() {
    let sandbox = TempSandbox::new(&[("test.txt", b"hello world")]);
    let app = app_for(sandbox.path());
    let ts = current_timestamp();
    let nonce = "nonce-replay-target";

    // 1st request: fresh, succeeds with 200
    let resp1 = send_request(&app, make_request(signed_envelope(WORKED_INPUT, ts, nonce))).await;
    assert_eq!(resp1.status, StatusCode::OK, "body: {}", resp1.body);

    // 2nd request: replayed identical payload -> rejected with 401 Unauthorized
    let resp2 = send_request(&app, make_request(signed_envelope(WORKED_INPUT, ts, nonce))).await;
    assert_eq!(
        resp2.status,
        StatusCode::UNAUTHORIZED,
        "replayed request must be rejected with 401"
    );
    assert!(
        resp2.body.contains("duplicate nonce") || resp2.body.contains("replay attack"),
        "error body must explain replay rejection: {}",
        resp2.body
    );
}

#[tokio::test]
async fn test_replay_rejects_missing_timestamp() {
    let sandbox = TempSandbox::new(&[("test.txt", b"hello world")]);
    let app = app_for(sandbox.path());

    let hash = auth::compute_hash(TEST_PASSWORD, WORKED_INPUT);
    let body = json!({
        "rfc_input": WORKED_INPUT,
        "nonce": "nonce-no-ts",
        "hash": hash,
    });

    let resp = send_request(&app, make_request(body)).await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
    assert!(resp.body.contains("missing required `timestamp`"));
}

#[tokio::test]
async fn test_replay_rejects_missing_nonce() {
    let sandbox = TempSandbox::new(&[("test.txt", b"hello world")]);
    let app = app_for(sandbox.path());
    let ts = current_timestamp();

    let hash = auth::compute_hash(TEST_PASSWORD, WORKED_INPUT);
    let body = json!({
        "rfc_input": WORKED_INPUT,
        "timestamp": ts,
        "hash": hash,
    });

    let resp = send_request(&app, make_request(body)).await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
    assert!(resp.body.contains("missing required `nonce`"));
}

#[tokio::test]
async fn test_replay_rejects_empty_nonce() {
    let sandbox = TempSandbox::new(&[("test.txt", b"hello world")]);
    let app = app_for(sandbox.path());
    let ts = current_timestamp();

    let resp = send_request(&app, make_request(signed_envelope(WORKED_INPUT, ts, "   "))).await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
    assert!(resp.body.contains("invalid `nonce`"));
}

#[tokio::test]
async fn test_replay_rejects_invalid_timestamp_format() {
    let sandbox = TempSandbox::new(&[("test.txt", b"hello world")]);
    let app = app_for(sandbox.path());
    let hash = auth::compute_hash(TEST_PASSWORD, WORKED_INPUT);

    let body = json!({
        "rfc_input": WORKED_INPUT,
        "timestamp": "invalid-timestamp",
        "nonce": "nonce-bad-ts",
        "hash": hash,
    });

    let resp = send_request(&app, make_request(body)).await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
    assert!(resp.body.contains("invalid `timestamp`"));
}

#[tokio::test]
async fn test_replay_rejects_expired_timestamp_past_300s() {
    let sandbox = TempSandbox::new(&[("test.txt", b"hello world")]);
    let app = app_for(sandbox.path());
    let ts_past_expired = current_timestamp() - 305;
    let nonce = "nonce-expired-past";

    let resp = send_request(
        &app,
        make_request(signed_envelope(WORKED_INPUT, ts_past_expired, nonce)),
    )
    .await;
    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "timestamp >300s in the past must be rejected with 401"
    );
    assert!(
        resp.body.contains("timestamp drift") || resp.body.contains("exceeds allowable window"),
        "error body: {}",
        resp.body
    );
}

#[tokio::test]
async fn test_replay_rejects_expired_timestamp_future_300s() {
    let sandbox = TempSandbox::new(&[("test.txt", b"hello world")]);
    let app = app_for(sandbox.path());
    let ts_future_expired = current_timestamp() + 305;
    let nonce = "nonce-expired-future";

    let resp = send_request(
        &app,
        make_request(signed_envelope(WORKED_INPUT, ts_future_expired, nonce)),
    )
    .await;
    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "timestamp >300s in the future must be rejected with 401"
    );
    assert!(
        resp.body.contains("timestamp drift") || resp.body.contains("exceeds allowable window"),
        "error body: {}",
        resp.body
    );
}

#[tokio::test]
async fn test_replay_accepts_timestamp_within_300s_window() {
    let sandbox = TempSandbox::new(&[("test.txt", b"hello world")]);
    let app = app_for(sandbox.path());
    let now = current_timestamp();

    // 100 seconds in past (well within 300s window)
    let resp_past = send_request(
        &app,
        make_request(signed_envelope(
            WORKED_INPUT,
            now - 100,
            "nonce-within-past",
        )),
    )
    .await;
    assert_eq!(
        resp_past.status,
        StatusCode::OK,
        "past within 300s accepted"
    );

    // 100 seconds in future (well within 300s window)
    let resp_future = send_request(
        &app,
        make_request(signed_envelope(
            WORKED_INPUT,
            now + 100,
            "nonce-within-future",
        )),
    )
    .await;
    assert_eq!(
        resp_future.status,
        StatusCode::OK,
        "future within 300s accepted"
    );
}

#[tokio::test]
async fn test_replay_different_nonces_all_succeed() {
    let sandbox = TempSandbox::new(&[("test.txt", b"hello world")]);
    let app = app_for(sandbox.path());
    let now = current_timestamp();

    for i in 0..10 {
        let nonce = format!("unique-batch-nonce-{i}");
        let resp = send_request(
            &app,
            make_request(signed_envelope(WORKED_INPUT, now, &nonce)),
        )
        .await;
        assert_eq!(resp.status, StatusCode::OK, "nonce {nonce} failed");
    }
}

#[tokio::test]
async fn test_replay_canonical_envelope_hmac_accepted() {
    let sandbox = TempSandbox::new(&[("test.txt", b"hello world")]);
    let app = app_for(sandbox.path());
    let now = current_timestamp();
    let nonce = "nonce-canonical-hmac-test";

    // Hash computed with canonical string: rfc_input:timestamp:nonce
    let canonical_hash = auth::compute_envelope_hash(TEST_PASSWORD, WORKED_INPUT, now, nonce);
    let body = json!({
        "rfc_input": WORKED_INPUT,
        "timestamp": now,
        "nonce": nonce,
        "hash": canonical_hash,
    });

    let resp = send_request(&app, make_request(body)).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "canonical HMAC envelope accepted: {}",
        resp.body
    );
}

#[tokio::test]
async fn test_replay_rejects_invalid_hmac_hash() {
    let sandbox = TempSandbox::new(&[("test.txt", b"hello world")]);
    let app = app_for(sandbox.path());
    let now = current_timestamp();
    let nonce = "nonce-bad-hash";

    let mut envelope = signed_envelope(WORKED_INPUT, now, nonce);
    envelope["hash"] = json!("0000000000000000000000000000000000000000000000000000000000000000");

    let resp = send_request(&app, make_request(envelope)).await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "invalid HMAC must be 403 Forbidden"
    );
    assert!(resp.body.contains("invalid HMAC hash"));
}

#[test]
fn test_replay_sliding_window_cache_capacity_and_eviction() {
    let max = 10;
    let window = AntiReplayWindow::new(Duration::from_secs(300), max);
    let base_time = 1_700_000_000;
    let now_instant = Instant::now();

    for i in 0..max {
        let nonce = format!("nonce-cap-{i}");
        assert!(window
            .check_and_record_with_time(base_time, &nonce, base_time, now_instant)
            .is_ok());
    }
    assert_eq!(window.len(), max);

    // Replay on existing item -> rejected
    let err = window.check_and_record_with_time(base_time, "nonce-cap-0", base_time, now_instant);
    assert_eq!(
        err,
        Err(ReplayError::DuplicateNonce("nonce-cap-0".to_string()))
    );

    // Insert new item -> capacity max is enforced by evicting oldest (nonce-cap-0)
    assert!(window
        .check_and_record_with_time(base_time, "nonce-cap-overflow", base_time, now_instant)
        .is_ok());
    assert_eq!(window.len(), max);

    // Now nonce-cap-0 is evicted and can be accepted again
    assert!(window
        .check_and_record_with_time(base_time, "nonce-cap-0", base_time, now_instant)
        .is_ok());
}

#[test]
fn test_replay_sliding_window_cache_ttl_purge() {
    let ttl = Duration::from_secs(300);
    let window = AntiReplayWindow::new(ttl, 100);
    let base_time = 1_700_000_000;
    let t0 = Instant::now();

    assert!(window
        .check_and_record_with_time(base_time, "nonce-ttl-1", base_time, t0)
        .is_ok());

    // Advance instant past TTL (301 seconds later)
    let t1 = t0 + Duration::from_secs(301);
    let time1 = base_time + 301;

    // Check with new time: old nonce is purged from sliding window
    assert!(window
        .check_and_record_with_time(time1, "nonce-ttl-fresh", time1, t1)
        .is_ok());

    // Since nonce-ttl-1 was purged after 300s TTL, it is no longer duplicate in cache
    assert!(window
        .check_and_record_with_time(time1, "nonce-ttl-1", time1, t1)
        .is_ok());
}
