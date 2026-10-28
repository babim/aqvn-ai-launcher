//! Outbound WebSocket Gateway client tests.
//!
//! Covers validation contract assertions VAL-GATEWAY-001 and VAL-GATEWAY-006:
//! - test_outbound_handshake_hello: Outbound WebSocket connection to /ws, executing
//!   connector_hello handshake with gateway ID, host label, and scoped capabilities.
//! - test_heartbeat_ping: WebSocket ping frame heartbeat.
//! - test_auto_reconnect_on_disconnect: Re-establishing connection and hello on disconnect.
//! - test_gateway_scopes_and_toggle: Scope filtering and termination via master toggle.

use aqvn_workspace_lib::config::{AppConfig, ConfigWatcher};
use aqvn_workspace_lib::connector::client::{
    build_ws_url, run_supervised, GatewayClient, GatewayConfig,
};
use aqvn_workspace_lib::connector::protocol::{
    ConnectorHello, MSG_TYPE_HELLO, MSG_TYPE_HELLO_ACK, SCOPE_BROWSER, SCOPE_DESKTOP, SCOPE_EXEC,
    SCOPE_FILES,
};
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use futures_util::sink::SinkExt;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

#[derive(Default)]
struct StubGatewayServer {
    hello_count: AtomicUsize,
    last_hello: Mutex<Option<ConnectorHello>>,
    ping_count: AtomicUsize,
    disconnect_first_client: AtomicBool,
}

async fn handle_ws(ws: WebSocketUpgrade, State(stub): State<Arc<StubGatewayServer>>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, stub))
}

async fn handle_socket(mut socket: WebSocket, stub: Arc<StubGatewayServer>) {
    // 1. Receive connector_hello
    if let Some(Ok(axum::extract::ws::Message::Text(text))) = socket.recv().await {
        if let Ok(hello) = serde_json::from_str::<ConnectorHello>(&text) {
            stub.hello_count.fetch_add(1, Ordering::SeqCst);
            *stub.last_hello.lock().await = Some(hello);

            // Reply with connector_hello_ack
            let ack = serde_json::json!({
                "type": MSG_TYPE_HELLO_ACK,
                "status": "ok"
            });
            let _ = socket
                .send(axum::extract::ws::Message::Text(ack.to_string().into()))
                .await;

            if stub.disconnect_first_client.swap(false, Ordering::SeqCst) {
                let _ = socket.close().await;
                return;
            }
        }
    }

    // 2. Receive frames (including pings)
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            axum::extract::ws::Message::Ping(data) => {
                stub.ping_count.fetch_add(1, Ordering::SeqCst);
                let _ = socket.send(axum::extract::ws::Message::Pong(data)).await;
            }
            axum::extract::ws::Message::Close(_) => break,
            _ => {}
        }
    }
}

async fn spawn_stub_server() -> (Arc<StubGatewayServer>, u16, tokio::task::JoinHandle<()>) {
    let stub = Arc::new(StubGatewayServer::default());
    let router = Router::new()
        .route("/ws", get(handle_ws))
        .with_state(Arc::clone(&stub));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral test listener");
    let port = listener.local_addr().expect("local addr").port();

    let server_handle = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });

    (stub, port, server_handle)
}

#[tokio::test]
async fn test_outbound_handshake_hello() {
    let (stub, port, _server) = spawn_stub_server().await;

    let config = GatewayConfig {
        enabled: true,
        server_url: format!("http://127.0.0.1:{port}"),
        gateway_id: "test-gw-12345".to_string(),
        host_label: "AQVN-Test-Host".to_string(),
        scope_files: true,
        scope_exec: true,
        scope_browser: true,
        scope_desktop: true,
        ping_interval_secs: 25,
        reconnect_delay_ms: 200,
    };

    let client = GatewayClient::new(config);
    let (handle, join_handle) = client.start();

    // Wait for connector_hello handshake to complete successfully
    handle
        .wait_for_handshake(Duration::from_secs(5))
        .await
        .expect("handshake should complete within 5s");

    assert!(handle.is_connected(), "client must be connected");
    assert!(
        handle.is_handshake_complete(),
        "handshake must be marked complete"
    );
    assert_eq!(stub.hello_count.load(Ordering::SeqCst), 1);

    let hello = stub
        .last_hello
        .lock()
        .await
        .clone()
        .expect("must have received connector_hello");

    assert_eq!(hello.msg_type, MSG_TYPE_HELLO);
    assert_eq!(hello.gateway_id, "test-gw-12345");
    assert_eq!(hello.host_label, "AQVN-Test-Host");

    // Declares all 4 required scopes
    assert!(hello.scopes.contains(&SCOPE_FILES.to_string()));
    assert!(hello.scopes.contains(&SCOPE_EXEC.to_string()));
    assert!(hello.scopes.contains(&SCOPE_BROWSER.to_string()));
    assert!(hello.scopes.contains(&SCOPE_DESKTOP.to_string()));

    // Capabilities map
    assert_eq!(hello.capabilities.get(SCOPE_FILES), Some(&true));
    assert_eq!(hello.capabilities.get(SCOPE_EXEC), Some(&true));
    assert_eq!(hello.capabilities.get(SCOPE_BROWSER), Some(&true));
    assert_eq!(hello.capabilities.get(SCOPE_DESKTOP), Some(&true));

    handle.stop();
    let _ = join_handle.await;
}

#[tokio::test]
async fn test_heartbeat_ping() {
    let (stub, port, _server) = spawn_stub_server().await;

    let config = GatewayConfig {
        enabled: true,
        server_url: format!("http://127.0.0.1:{port}"),
        gateway_id: "test-gw-ping".to_string(),
        host_label: "AQVN-Test-Host".to_string(),
        scope_files: true,
        scope_exec: false,
        scope_browser: false,
        scope_desktop: false,
        ping_interval_secs: 1, // 1s interval for fast test
        reconnect_delay_ms: 200,
    };

    let client = GatewayClient::new(config);
    let (handle, join_handle) = client.start();

    handle
        .wait_for_handshake(Duration::from_secs(5))
        .await
        .expect("handshake completed");

    // Wait 2.5s for ping ticks
    tokio::time::sleep(Duration::from_millis(2500)).await;

    let pings = stub.ping_count.load(Ordering::SeqCst);
    assert!(
        pings >= 2,
        "must receive at least 2 pings in 2.5s, got {pings}"
    );

    handle.stop();
    let _ = join_handle.await;
}

#[tokio::test]
async fn test_auto_reconnect_on_disconnect() {
    let (stub, port, _server) = spawn_stub_server().await;
    stub.disconnect_first_client.store(true, Ordering::SeqCst);

    let config = GatewayConfig {
        enabled: true,
        server_url: format!("http://127.0.0.1:{port}"),
        gateway_id: "test-gw-reconnect".to_string(),
        host_label: "AQVN-Test-Host".to_string(),
        scope_files: true,
        scope_exec: true,
        scope_browser: true,
        scope_desktop: true,
        ping_interval_secs: 25,
        reconnect_delay_ms: 50, // fast retry
    };

    let client = GatewayClient::new(config);
    let (handle, join_handle) = client.start();

    // First connection will get disconnected by stub server, then auto-reconnect
    // Wait for at least 2 hello handshakes
    let start = std::time::Instant::now();
    let mut reconnected = false;
    while start.elapsed() < Duration::from_secs(5) {
        if stub.hello_count.load(Ordering::SeqCst) >= 2 {
            reconnected = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert!(
        reconnected,
        "client must automatically reconnect after disconnection"
    );

    handle.stop();
    let _ = join_handle.await;
}

#[tokio::test]
async fn test_gateway_scopes_and_toggle() {
    let (stub, port, _server) = spawn_stub_server().await;

    // Scope configuration: only files and browser
    let config = GatewayConfig {
        enabled: true,
        server_url: format!("http://127.0.0.1:{port}"),
        gateway_id: "test-gw-scopes".to_string(),
        host_label: "AQVN-Test-Host".to_string(),
        scope_files: true,
        scope_exec: false,
        scope_browser: true,
        scope_desktop: false,
        ping_interval_secs: 25,
        reconnect_delay_ms: 200,
    };

    let client = GatewayClient::new(config);
    let (handle, join_handle) = client.start();

    handle
        .wait_for_handshake(Duration::from_secs(5))
        .await
        .expect("handshake completed");

    let hello = stub
        .last_hello
        .lock()
        .await
        .clone()
        .expect("must have received hello");

    assert_eq!(
        hello.scopes,
        vec![SCOPE_FILES.to_string(), SCOPE_BROWSER.to_string()]
    );
    assert_eq!(hello.capabilities.get(SCOPE_FILES), Some(&true));
    assert_eq!(hello.capabilities.get(SCOPE_EXEC), None);
    assert_eq!(hello.capabilities.get(SCOPE_BROWSER), Some(&true));
    assert_eq!(hello.capabilities.get(SCOPE_DESKTOP), None);

    // Master toggle: stop connection immediately
    handle.stop();
    let _ = join_handle.await;
    assert!(
        !handle.is_connected(),
        "connection terminated after master stop"
    );
}

#[tokio::test]
async fn test_gateway_supervisor_hot_reload_and_master_toggle() {
    let (stub, port, _server) = spawn_stub_server().await;

    let initial = AppConfig {
        server_url: format!("http://127.0.0.1:{port}"),
        connector_enabled: true,
        scope_files: true,
        scope_exec: true,
        scope_browser: false,
        scope_desktop: false,
        ..Default::default()
    };
    let watcher = ConfigWatcher::new(initial.clone());

    let sup_watcher = watcher.clone();
    let sup_handle = tokio::spawn(async move {
        let _ = run_supervised(sup_watcher).await;
    });

    // 1. Initial connect: scopes should be [files, code_execution]
    let start = std::time::Instant::now();
    let mut hello1_recvd = None;
    while start.elapsed() < Duration::from_secs(5) {
        if let Some(h) = stub.last_hello.lock().await.clone() {
            hello1_recvd = Some(h);
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let h1 = hello1_recvd.expect("initial handshake must arrive");
    assert_eq!(
        h1.scopes,
        vec![SCOPE_FILES.to_string(), SCOPE_EXEC.to_string()]
    );

    // 2. Hot-reload config: enable only browser
    let mut updated = initial.clone();
    updated.scope_files = false;
    updated.scope_exec = false;
    updated.scope_browser = true;
    *stub.last_hello.lock().await = None;
    watcher.update(updated.clone());

    let start = std::time::Instant::now();
    let mut hello2_recvd = None;
    while start.elapsed() < Duration::from_secs(5) {
        if let Some(h) = stub.last_hello.lock().await.clone() {
            hello2_recvd = Some(h);
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let h2 = hello2_recvd.expect("handshake after scope reload must arrive");
    assert_eq!(h2.scopes, vec![SCOPE_BROWSER.to_string()]);

    // 3. Master toggle off: connector_enabled = false terminates connection
    let mut disabled = updated.clone();
    disabled.connector_enabled = false;
    *stub.last_hello.lock().await = None;
    watcher.update(disabled);

    tokio::time::sleep(Duration::from_millis(600)).await;
    assert!(
        stub.last_hello.lock().await.is_none(),
        "no connection after master toggle disabled"
    );

    sup_handle.abort();
}

#[test]
fn test_build_ws_url_variants() {
    assert_eq!(
        build_ws_url("http://127.0.0.1:8080").unwrap(),
        "ws://127.0.0.1:8080/ws"
    );
    assert_eq!(
        build_ws_url("https://a0.matmagoc.com").unwrap(),
        "wss://a0.matmagoc.com/ws"
    );
    assert_eq!(
        build_ws_url("http://127.0.0.1:8080/ws").unwrap(),
        "ws://127.0.0.1:8080/ws"
    );
    assert_eq!(
        build_ws_url("ws://localhost:3000").unwrap(),
        "ws://localhost:3000/ws"
    );
    assert_eq!(
        build_ws_url("wss://localhost:3000/ws").unwrap(),
        "wss://localhost:3000/ws"
    );
}
