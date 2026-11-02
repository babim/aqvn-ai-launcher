//! Tests for connector handlers and resilience contracts.
//!
//! Covers validation contract assertion VAL-RES-005:
//! Gateway client decorrelated jitter backoff.

use aqvn_workspace_lib::connector::client::{
    compute_jitter_backoff, next_decorrelated_jitter_delay, GatewayClient, GatewayConfig,
    MAX_RECONNECT_DELAY_MS, MIN_RECONNECT_DELAY_MS, RECONNECT_JITTER_MS,
};
use aqvn_workspace_lib::connector::protocol::{MSG_TYPE_HELLO, MSG_TYPE_HELLO_ACK};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::routing::get;
use axum::Router;
use futures_util::sink::SinkExt;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;

#[test]
fn test_reconnect_constants() {
    assert_eq!(MIN_RECONNECT_DELAY_MS, 500);
    assert_eq!(MAX_RECONNECT_DELAY_MS, 15000);
    assert_eq!(RECONNECT_JITTER_MS, 500);
}

#[test]
fn test_decorrelated_jitter_backoff_scaling() {
    // Initial attempt (current_delay = 0) with zero jitter yields min_delay
    let initial_zero = compute_jitter_backoff(0, 500, 15000, 0);
    assert_eq!(initial_zero, 500);

    // Initial attempt with max jitter yields 1000ms
    let initial_max = compute_jitter_backoff(0, 500, 15000, 500);
    assert_eq!(initial_max, 1000);

    // Step 1: from 500ms baseline, delay scales by 1.5x (750ms + jitter)
    let step1_min = compute_jitter_backoff(500, 500, 15000, 0);
    let step1_max = compute_jitter_backoff(500, 500, 15000, 500);
    assert_eq!(step1_min, 750);
    assert_eq!(step1_max, 1250);

    // Step 2: from 1000ms, scales to 1500ms + jitter
    let step2_min = compute_jitter_backoff(1000, 500, 15000, 0);
    let step2_max = compute_jitter_backoff(1000, 500, 15000, 500);
    assert_eq!(step2_min, 1500);
    assert_eq!(step2_max, 2000);

    // Scaling up reaches and clamps at 15000ms (15s)
    let clamped = compute_jitter_backoff(12000, 500, 15000, 500);
    assert_eq!(clamped, 15000);

    let over_max = compute_jitter_backoff(20000, 500, 15000, 500);
    assert_eq!(over_max, 15000);
}

#[test]
fn test_decorrelated_jitter_randomness_and_distribution() {
    let mut sampled_delays = HashSet::new();
    for _ in 0..100 {
        let delay = next_decorrelated_jitter_delay(500, 500, 15000);
        assert!(
            (750..=1250).contains(&delay),
            "delay {delay} must be within [750, 1250]"
        );
        sampled_delays.insert(delay);
    }

    // Jitter must be randomized across multiple samples to prevent connection storms
    assert!(
        sampled_delays.len() > 10,
        "decorrelated jitter should produce randomized values, got {} distinct values",
        sampled_delays.len()
    );
}

#[test]
fn test_full_backoff_progression_to_max() {
    let mut current = 0;
    let mut iterations = 0;

    while current < MAX_RECONNECT_DELAY_MS && iterations < 50 {
        let next =
            next_decorrelated_jitter_delay(current, MIN_RECONNECT_DELAY_MS, MAX_RECONNECT_DELAY_MS);
        assert!(next >= MIN_RECONNECT_DELAY_MS);
        assert!(next <= MAX_RECONNECT_DELAY_MS);
        current = next;
        iterations += 1;
    }

    assert_eq!(current, MAX_RECONNECT_DELAY_MS);
    assert!(
        (5..=15).contains(&iterations),
        "should scale to max within 5-15 steps"
    );
}

#[derive(Default)]
struct MockGatewayServer {
    hello_count: AtomicUsize,
    disconnect_first: AtomicBool,
}

async fn handle_mock_ws(
    ws: WebSocketUpgrade,
    State(mock): State<Arc<MockGatewayServer>>,
) -> axum::response::Response {
    ws.on_upgrade(move |socket| handle_mock_socket(socket, mock))
}

async fn handle_mock_socket(mut socket: WebSocket, mock: Arc<MockGatewayServer>) {
    let open_pkt = serde_json::json!({
        "sid": "mock-sid-test",
        "upgrades": [],
        "pingInterval": 25000,
        "pingTimeout": 20000
    });
    if socket
        .send(Message::Text(format!("0{open_pkt}").into()))
        .await
        .is_err()
    {
        return;
    }

    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                if text.starts_with("40/ws") {
                    let _ = socket
                        .send(Message::Text("40/ws,{\"sid\":\"mock-client-sid\"}".into()))
                        .await;
                } else if text.contains(MSG_TYPE_HELLO) {
                    mock.hello_count.fetch_add(1, Ordering::SeqCst);
                    let ack = serde_json::json!({
                        "type": MSG_TYPE_HELLO_ACK,
                        "status": "ok"
                    });
                    let ack_msg = format!("42/ws,[\"{MSG_TYPE_HELLO_ACK}\",{ack}]");
                    let _ = socket.send(Message::Text(ack_msg.into())).await;

                    if mock.disconnect_first.swap(false, Ordering::SeqCst) {
                        let _ = socket.close().await;
                        return;
                    }
                }
            }
            Message::Close(_) => return,
            _ => {}
        }
    }
}

#[tokio::test]
async fn test_gateway_client_reconnects_with_jitter_backoff() {
    let mock = Arc::new(MockGatewayServer {
        disconnect_first: AtomicBool::new(true),
        ..Default::default()
    });

    let router = Router::new()
        .route("/socket.io/", get(handle_mock_ws))
        .route("/ws", get(handle_mock_ws))
        .with_state(Arc::clone(&mock));

    let listener = TcpListener::bind(std::net::SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .expect("bind listener");
    let port = listener.local_addr().expect("local addr").port();
    let server_handle = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });

    let config = GatewayConfig {
        enabled: true,
        server_url: format!("http://127.0.0.1:{port}"),
        gateway_id: "test-gw-jitter-reconnect".to_string(),
        host_label: "AQVN-Test-Host".to_string(),
        reconnect_delay_ms: 100, // fast retry for test execution
        ..Default::default()
    };

    let client = GatewayClient::new(config);
    let (handle, join_handle) = client.start();

    // Verify client auto-reconnects after the initial forced disconnect
    let start = std::time::Instant::now();
    let mut reconnected = false;
    while start.elapsed() < Duration::from_secs(5) {
        if mock.hello_count.load(Ordering::SeqCst) >= 2 {
            reconnected = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert!(reconnected, "client must reconnect after connection drop");

    handle.stop();
    let _ = join_handle.await;
    server_handle.abort();
}
