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
    ConnectorHello, MSG_TYPE_HELLO, MSG_TYPE_HELLO_ACK, MSG_TYPE_REMOTE_TREE_UPDATE, SCOPE_BROWSER,
    SCOPE_DESKTOP, SCOPE_EXEC, SCOPE_FILES,
};
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::{header, HeaderMap};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use futures_util::sink::SinkExt;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

#[derive(Default)]
struct StubGatewayServer {
    hello_count: AtomicUsize,
    last_hello: Mutex<Option<ConnectorHello>>,
    last_cookie: Mutex<Option<String>>,
    last_connect_packet: Mutex<Option<String>>,
    ping_count: AtomicUsize,
    disconnect_first_client: AtomicBool,
}

async fn handle_ws(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(stub): State<Arc<StubGatewayServer>>,
) -> Response {
    if let Some(cookie) = headers.get(header::COOKIE).and_then(|v| v.to_str().ok()) {
        *stub.last_cookie.lock().await = Some(cookie.to_string());
    }
    ws.on_upgrade(move |socket| handle_socket(socket, stub))
}

async fn handle_socket(mut socket: WebSocket, stub: Arc<StubGatewayServer>) {
    // 1. Send Engine.IO open packet
    let open_pkt = serde_json::json!({
        "sid": "mock-stub-sid-1",
        "upgrades": [],
        "pingInterval": 25000,
        "pingTimeout": 20000
    });
    if socket
        .send(axum::extract::ws::Message::Text(
            format!("0{open_pkt}").into(),
        ))
        .await
        .is_err()
    {
        return;
    }

    // 2. Wait for namespace /ws join packet: 40/ws,
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            axum::extract::ws::Message::Text(text) => {
                if text.starts_with("40/ws") {
                    *stub.last_connect_packet.lock().await = Some(text.to_string());
                    // Send namespace connect confirmation
                    let _ = socket
                        .send(axum::extract::ws::Message::Text(
                            "40/ws,{\"sid\":\"mock-client-sid-1\"}".into(),
                        ))
                        .await;
                    break;
                } else if text.contains(MSG_TYPE_HELLO) {
                    // Direct text hello (legacy)
                    if let Ok(hello) = serde_json::from_str::<ConnectorHello>(&text) {
                        stub.hello_count.fetch_add(1, Ordering::SeqCst);
                        *stub.last_hello.lock().await = Some(hello);

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
                    break;
                }
            }
            axum::extract::ws::Message::Close(_) => return,
            _ => {}
        }
    }

    // 3. Receive connector_hello
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            axum::extract::ws::Message::Text(text) => {
                let hello_opt = if let Some(stripped) = text.strip_prefix("42/ws,") {
                    if let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(stripped) {
                        if arr.first().and_then(Value::as_str) == Some("connector_hello") {
                            arr.get(1).and_then(|v| {
                                serde_json::from_value::<ConnectorHello>(v.clone()).ok()
                            })
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    serde_json::from_str::<ConnectorHello>(&text).ok()
                };

                if let Some(hello) = hello_opt {
                    stub.hello_count.fetch_add(1, Ordering::SeqCst);
                    *stub.last_hello.lock().await = Some(hello);

                    let ack_payload = serde_json::json!({
                        "type": MSG_TYPE_HELLO_ACK,
                        "status": "ok"
                    });
                    let ack_msg = format!("42/ws,[\"{MSG_TYPE_HELLO_ACK}\",{ack_payload}]");
                    let _ = socket
                        .send(axum::extract::ws::Message::Text(ack_msg.into()))
                        .await;

                    if stub.disconnect_first_client.swap(false, Ordering::SeqCst) {
                        let _ = socket.close().await;
                        return;
                    }
                    break;
                }
            }
            axum::extract::ws::Message::Close(_) => return,
            _ => {}
        }
    }

    // 4. Receive frames (including pings)
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            axum::extract::ws::Message::Ping(data) => {
                stub.ping_count.fetch_add(1, Ordering::SeqCst);
                let _ = socket.send(axum::extract::ws::Message::Pong(data)).await;
            }
            axum::extract::ws::Message::Text(text) => {
                if text == "2" || text.starts_with("2") {
                    stub.ping_count.fetch_add(1, Ordering::SeqCst);
                    let _ = socket
                        .send(axum::extract::ws::Message::Text("3".into()))
                        .await;
                }
            }
            axum::extract::ws::Message::Close(_) => break,
            _ => {}
        }
    }
}

async fn spawn_stub_server() -> (Arc<StubGatewayServer>, u16, tokio::task::JoinHandle<()>) {
    let stub = Arc::new(StubGatewayServer::default());
    let router = Router::new()
        .route("/socket.io/", get(handle_ws))
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
        session_cookie: None,
        scope_files: true,
        scope_exec: true,
        scope_browser: true,
        scope_desktop: true,
        ping_interval_secs: 25,
        reconnect_delay_ms: 200,
        ..Default::default()
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
        session_cookie: None,
        scope_files: true,
        scope_exec: false,
        scope_browser: false,
        scope_desktop: false,
        ping_interval_secs: 1, // 1s interval for fast test
        reconnect_delay_ms: 200,
        ..Default::default()
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
        session_cookie: None,
        scope_files: true,
        scope_exec: true,
        scope_browser: true,
        scope_desktop: true,
        ping_interval_secs: 25,
        reconnect_delay_ms: 50, // fast retry
        ..Default::default()
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
        session_cookie: None,
        scope_files: true,
        scope_exec: false,
        scope_browser: true,
        scope_desktop: false,
        ping_interval_secs: 25,
        reconnect_delay_ms: 200,
        ..Default::default()
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

#[tokio::test]
async fn test_gateway_session_cookie_auth() {
    let (stub, port, _server) = spawn_stub_server().await;

    let config = GatewayConfig {
        enabled: true,
        server_url: format!("http://127.0.0.1:{port}"),
        gateway_id: "test-gw-cookie".to_string(),
        host_label: "AQVN-Test-Host".to_string(),
        scope_files: true,
        scope_exec: false,
        scope_browser: false,
        scope_desktop: false,
        ping_interval_secs: 25,
        reconnect_delay_ms: 200,
        session_cookie: Some("session_runtime123=tok_abc456".to_string()),
        ..Default::default()
    };

    let client = GatewayClient::new(config);
    let (handle, join_handle) = client.start();

    handle
        .wait_for_handshake(Duration::from_secs(5))
        .await
        .expect("handshake should complete within 5s");

    let received_cookie = stub
        .last_cookie
        .lock()
        .await
        .clone()
        .expect("must have received Cookie header");

    assert!(
        received_cookie.contains("session_runtime123=tok_abc456"),
        "cookie header must contain session cookie, got: {received_cookie}"
    );

    handle.stop();
    let _ = join_handle.await;
}

#[tokio::test]
async fn test_gateway_cookie_jar_from_file_auth() {
    let (stub, port, _server) = spawn_stub_server().await;
    let server_url = format!("http://127.0.0.1:{port}");

    // Persist a session cookie into the CookieJar on disk for this origin
    let cookie_file = aqvn_workspace_lib::proxy::upstream::cookie_file_path(&server_url);
    let mut jar = aqvn_workspace_lib::proxy::jar::CookieJar::new();
    jar.store("session_disk123=token_file_789; Path=/; HttpOnly");
    jar.save_to_file(&cookie_file)
        .expect("save test cookie jar");

    let config = GatewayConfig {
        enabled: true,
        server_url: server_url.clone(),
        gateway_id: "test-gw-cookie-jar".to_string(),
        host_label: "AQVN-Test-Host".to_string(),
        scope_files: true,
        scope_exec: false,
        scope_browser: false,
        scope_desktop: false,
        ping_interval_secs: 25,
        reconnect_delay_ms: 200,
        session_cookie: None, // No explicit cookie, must load from disk CookieJar
        ..Default::default()
    };

    let client = GatewayClient::new(config);
    let (handle, join_handle) = client.start();

    handle
        .wait_for_handshake(Duration::from_secs(5))
        .await
        .expect("handshake should complete within 5s");

    let received_cookie = stub
        .last_cookie
        .lock()
        .await
        .clone()
        .expect("must have received Cookie header from disk jar");

    assert!(
        received_cookie.contains("session_disk123=token_file_789"),
        "cookie header must contain session cookie loaded from disk jar, got: {received_cookie}"
    );

    handle.stop();
    let _ = join_handle.await;
    aqvn_workspace_lib::proxy::jar::CookieJar::remove_file(&cookie_file);
}

#[test]
fn test_build_ws_url_variants() {
    assert_eq!(
        build_ws_url("http://127.0.0.1:8080").unwrap(),
        "ws://127.0.0.1:8080/socket.io/?EIO=4&transport=websocket"
    );
    assert_eq!(
        build_ws_url("https://a0.matmagoc.com").unwrap(),
        "wss://a0.matmagoc.com/socket.io/?EIO=4&transport=websocket"
    );
    assert_eq!(
        build_ws_url("http://127.0.0.1:8080/").unwrap(),
        "ws://127.0.0.1:8080/socket.io/?EIO=4&transport=websocket"
    );
    assert_eq!(
        build_ws_url("http://127.0.0.1:8080/ws").unwrap(),
        "ws://127.0.0.1:8080/socket.io/?EIO=4&transport=websocket"
    );
    assert_eq!(
        build_ws_url("ws://localhost:3000").unwrap(),
        "ws://localhost:3000/socket.io/?EIO=4&transport=websocket"
    );
    assert_eq!(
        build_ws_url("wss://localhost:3000/ws").unwrap(),
        "wss://localhost:3000/socket.io/?EIO=4&transport=websocket"
    );
    assert_eq!(
        build_ws_url("wss://localhost:3000/socket.io/?EIO=4&transport=websocket").unwrap(),
        "wss://localhost:3000/socket.io/?EIO=4&transport=websocket"
    );
}

#[tokio::test]
async fn test_socketio_event_dispatch_and_ack() {
    let (tx_event, rx_event) = tokio::sync::mpsc::channel::<String>(16);
    let (tx_reply, mut rx_reply) = tokio::sync::mpsc::channel::<String>(16);
    let rx_event = Arc::new(Mutex::new(rx_event));
    let tx_reply = Arc::new(Mutex::new(tx_reply));

    let router = Router::new().route(
        "/socket.io/",
        get({
            let rx_event = Arc::clone(&rx_event);
            let tx_reply = Arc::clone(&tx_reply);
            move |ws: WebSocketUpgrade| {
                let rx_event = Arc::clone(&rx_event);
                let tx_reply = Arc::clone(&tx_reply);
                async move {
                    ws.on_upgrade(move |mut socket: WebSocket| async move {
                        // 1. Send Engine.IO open packet
                        let open_pkt = serde_json::json!({
                            "sid": "mock-sio-event-sid",
                            "upgrades": [],
                            "pingInterval": 25000,
                            "pingTimeout": 20000
                        });
                        let _ = socket
                            .send(axum::extract::ws::Message::Text(
                                format!("0{open_pkt}").into(),
                            ))
                            .await;

                        // 2. Wait for namespace /ws join packet
                        while let Some(Ok(msg)) = socket.recv().await {
                            if let axum::extract::ws::Message::Text(text) = msg {
                                if text.starts_with("40/ws") {
                                    let _ = socket
                                        .send(axum::extract::ws::Message::Text(
                                            "40/ws,{\"sid\":\"test-sid\"}".into(),
                                        ))
                                        .await;
                                    break;
                                }
                            }
                        }

                        // 3. Receive connector_hello
                        while let Some(Ok(msg)) = socket.recv().await {
                            if let axum::extract::ws::Message::Text(text) = msg {
                                if text.contains("connector_hello") {
                                    let ack = format!(
                                        "42/ws,[\"{MSG_TYPE_HELLO_ACK}\",{{\"status\":\"ok\"}}]"
                                    );
                                    let _ = socket
                                        .send(axum::extract::ws::Message::Text(ack.into()))
                                        .await;
                                    break;
                                }
                            }
                        }

                        // 4. Pump test events from rx_event and forward replies to tx_reply
                        let mut rx_event_lock = rx_event.lock().await;
                        while let Some(event_str) = rx_event_lock.recv().await {
                            if socket
                                .send(axum::extract::ws::Message::Text(event_str.into()))
                                .await
                                .is_err()
                            {
                                break;
                            }
                            // Wait for reply frame
                            while let Some(Ok(msg)) = socket.recv().await {
                                if let axum::extract::ws::Message::Text(reply_text) = msg {
                                    if reply_text.contains(MSG_TYPE_REMOTE_TREE_UPDATE) {
                                        continue;
                                    }
                                    if reply_text.starts_with("42") || reply_text.starts_with("43")
                                    {
                                        let tx = tx_reply.lock().await;
                                        let _ = tx.send(reply_text.to_string()).await;
                                        break;
                                    }
                                }
                            }
                        }
                    })
                }
            }
        }),
    );

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let port = listener.local_addr().expect("port").port();
    let server_task = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });

    let config = GatewayConfig {
        enabled: true,
        server_url: format!("http://127.0.0.1:{port}"),
        gateway_id: "test-gw-sio-events".to_string(),
        host_label: "AQVN-Test-Host".to_string(),
        session_cookie: None,
        scope_files: true,
        scope_exec: true,
        scope_browser: true,
        scope_desktop: true,
        ping_interval_secs: 25,
        reconnect_delay_ms: 200,
        ..Default::default()
    };

    let client = GatewayClient::new(config);
    let (handle, join_handle) = client.start();

    handle
        .wait_for_handshake(Duration::from_secs(5))
        .await
        .expect("handshake");

    // Case A: Send event without ack ID -> Expect 42/ws,["pong", ...]
    tx_event
        .send("42/ws,[\"ping\",{}]".to_string())
        .await
        .unwrap();
    let reply_a = tokio::time::timeout(Duration::from_secs(3), rx_reply.recv())
        .await
        .expect("reply_a timeout")
        .expect("reply_a recv");
    assert!(
        reply_a.starts_with("42/ws,[\"pong\""),
        "reply_a must be 42/ws event, got: {reply_a}"
    );

    // Case B: Send event with ack ID 42 -> Expect 43/ws,42[...]
    tx_event
        .send("42/ws,42[\"ping\",{}]".to_string())
        .await
        .unwrap();
    let reply_b = tokio::time::timeout(Duration::from_secs(3), rx_reply.recv())
        .await
        .expect("reply_b timeout")
        .expect("reply_b recv");
    assert!(
        reply_b.starts_with("43/ws,42["),
        "reply_b must be 43/ws ack, got: {reply_b}"
    );

    handle.stop();
    let _ = join_handle.await;
    server_task.abort();
}

#[tokio::test]
async fn test_socketio_interleaved_ping_pong() {
    let pongs_received = Arc::new(AtomicUsize::new(0));
    let pongs_clone = Arc::clone(&pongs_received);

    let router = Router::new().route(
        "/socket.io/",
        get({
            move |ws: WebSocketUpgrade| {
                let pongs = Arc::clone(&pongs_clone);
                async move {
                    ws.on_upgrade(move |mut socket: WebSocket| async move {
                        // 1. Send Engine.IO open packet
                        let open_pkt = serde_json::json!({
                            "sid": "mock-ping-pong-sid",
                            "upgrades": [],
                            "pingInterval": 25000,
                            "pingTimeout": 20000
                        });
                        let _ = socket
                            .send(axum::extract::ws::Message::Text(
                                format!("0{open_pkt}").into(),
                            ))
                            .await;

                        // 2. Interleave a ping before namespace connect ack!
                        let _ = socket
                            .send(axum::extract::ws::Message::Text("2".into()))
                            .await;

                        // Wait for namespace /ws join packet or pong
                        while let Some(Ok(msg)) = socket.recv().await {
                            if let axum::extract::ws::Message::Text(text) = msg {
                                if text == "3" {
                                    pongs.fetch_add(1, Ordering::SeqCst);
                                } else if text.starts_with("40/ws") {
                                    let _ = socket
                                        .send(axum::extract::ws::Message::Text(
                                            "40/ws,{\"sid\":\"test-sid\"}".into(),
                                        ))
                                        .await;
                                    break;
                                }
                            }
                        }

                        // 3. Receive connector_hello
                        while let Some(Ok(msg)) = socket.recv().await {
                            if let axum::extract::ws::Message::Text(text) = msg {
                                if text.contains("connector_hello") {
                                    let ack = format!(
                                        "42/ws,[\"{MSG_TYPE_HELLO_ACK}\",{{\"status\":\"ok\"}}]"
                                    );
                                    let _ = socket
                                        .send(axum::extract::ws::Message::Text(ack.into()))
                                        .await;
                                    break;
                                }
                            }
                        }

                        // 4. Send probe ping
                        let _ = socket
                            .send(axum::extract::ws::Message::Text("2probe".into()))
                            .await;

                        while let Some(Ok(msg)) = socket.recv().await {
                            if let axum::extract::ws::Message::Text(text) = msg {
                                if text == "3probe" {
                                    pongs.fetch_add(1, Ordering::SeqCst);
                                    break;
                                } else if text == "3" {
                                    pongs.fetch_add(1, Ordering::SeqCst);
                                }
                            }
                        }
                    })
                }
            }
        }),
    );

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let port = listener.local_addr().expect("port").port();
    let server_task = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });

    let config = GatewayConfig {
        enabled: true,
        server_url: format!("http://127.0.0.1:{port}"),
        gateway_id: "test-gw-ping-pong".to_string(),
        host_label: "AQVN-Test-Host".to_string(),
        session_cookie: None,
        scope_files: true,
        scope_exec: true,
        scope_browser: true,
        scope_desktop: true,
        ping_interval_secs: 25,
        reconnect_delay_ms: 200,
        ..Default::default()
    };

    let client = GatewayClient::new(config);
    let (handle, join_handle) = client.start();

    handle
        .wait_for_handshake(Duration::from_secs(5))
        .await
        .expect("handshake completed");

    // Wait for 3probe to arrive
    tokio::time::sleep(Duration::from_millis(500)).await;

    assert!(
        pongs_received.load(Ordering::SeqCst) >= 2,
        "must have received both pongs (normal and probe)"
    );

    handle.stop();
    let _ = join_handle.await;
    server_task.abort();
}

#[tokio::test]
async fn test_namespace_connect_carries_auth_handlers() {
    let (stub, port, _server) = spawn_stub_server().await;

    let config = GatewayConfig {
        enabled: true,
        server_url: format!("http://127.0.0.1:{port}"),
        gateway_id: "test-gw-auth".to_string(),
        host_label: "AQVN-Test-Host".to_string(),
        session_cookie: None,
        scope_files: true,
        scope_exec: true,
        scope_browser: true,
        scope_desktop: true,
        ping_interval_secs: 25,
        reconnect_delay_ms: 200,
        ..Default::default()
    };

    let client = GatewayClient::new(config);
    let (handle, join_handle) = client.start();

    handle
        .wait_for_handshake(Duration::from_secs(5))
        .await
        .expect("handshake should complete within 5s");

    let connect_pkt = stub
        .last_connect_packet
        .lock()
        .await
        .clone()
        .expect("must have received namespace connect packet");

    // Verify Socket.IO namespace connect packet 40/ws, framing
    assert!(
        connect_pkt.starts_with("40/ws,"),
        "packet must start with 40/ws,, but got: {connect_pkt}"
    );

    // Verify auth payload
    let auth_str = connect_pkt.strip_prefix("40/ws,").expect("prefix");
    let auth: Value = serde_json::from_str(auth_str).expect("parse auth json");

    assert_eq!(auth["client_type"], "launcher");
    let handlers = auth["handlers"].as_array().expect("handlers array");
    assert!(
        handlers.contains(&Value::String(
            "plugins/_a0_connector/ws_connector".to_string()
        )),
        "handlers must contain plugins/_a0_connector/ws_connector, got: {:?}",
        handlers
    );

    handle.stop();
    let _ = join_handle.await;
}

#[tokio::test]
async fn test_gateway_control_acknowledged() {
    let (tx_event, rx_event) = tokio::sync::mpsc::channel::<String>(16);
    let (tx_reply, mut rx_reply) = tokio::sync::mpsc::channel::<String>(16);
    let rx_event = Arc::new(Mutex::new(rx_event));
    let tx_reply = Arc::new(Mutex::new(tx_reply));

    let router = Router::new().route(
        "/socket.io/",
        get({
            let rx_event = Arc::clone(&rx_event);
            let tx_reply = Arc::clone(&tx_reply);
            move |ws: WebSocketUpgrade| {
                let rx_event = Arc::clone(&rx_event);
                let tx_reply = Arc::clone(&tx_reply);
                async move {
                    ws.on_upgrade(move |mut socket: WebSocket| async move {
                        // 1. Send Engine.IO open packet
                        let open_pkt = serde_json::json!({
                            "sid": "mock-sio-ctrl-sid",
                            "upgrades": [],
                            "pingInterval": 25000,
                            "pingTimeout": 20000
                        });
                        let _ = socket
                            .send(axum::extract::ws::Message::Text(
                                format!("0{open_pkt}").into(),
                            ))
                            .await;

                        // 2. Wait for namespace /ws join packet
                        while let Some(Ok(msg)) = socket.recv().await {
                            if let axum::extract::ws::Message::Text(text) = msg {
                                if text.starts_with("40/ws") {
                                    let _ = socket
                                        .send(axum::extract::ws::Message::Text(
                                            "40/ws,{\"sid\":\"test-ctrl-sid\"}".into(),
                                        ))
                                        .await;
                                    break;
                                }
                            }
                        }

                        // 3. Receive connector_hello and reply with ack
                        while let Some(Ok(msg)) = socket.recv().await {
                            if let axum::extract::ws::Message::Text(text) = msg {
                                if text.contains("connector_hello") {
                                    let ack = format!(
                                        "42/ws,[\"{MSG_TYPE_HELLO_ACK}\",{{\"status\":\"ok\"}}]"
                                    );
                                    let _ = socket
                                        .send(axum::extract::ws::Message::Text(ack.into()))
                                        .await;
                                    break;
                                }
                            }
                        }

                        // 4. Pump test events from rx_event and forward replies to tx_reply
                        let mut rx_event_lock = rx_event.lock().await;
                        while let Some(event_str) = rx_event_lock.recv().await {
                            if socket
                                .send(axum::extract::ws::Message::Text(event_str.into()))
                                .await
                                .is_err()
                            {
                                break;
                            }
                            // Wait for reply frame
                            while let Some(Ok(msg)) = socket.recv().await {
                                if let axum::extract::ws::Message::Text(reply_text) = msg {
                                    if reply_text.contains(MSG_TYPE_REMOTE_TREE_UPDATE) {
                                        continue;
                                    }
                                    if reply_text.starts_with("42") || reply_text.starts_with("43")
                                    {
                                        let tx = tx_reply.lock().await;
                                        let _ = tx.send(reply_text.to_string()).await;
                                        break;
                                    }
                                }
                            }
                        }
                    })
                }
            }
        }),
    );

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let port = listener.local_addr().expect("port").port();
    let server_task = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });

    let config = GatewayConfig {
        enabled: true,
        server_url: format!("http://127.0.0.1:{port}"),
        gateway_id: "test-gw-ctrl-id".to_string(),
        host_label: "AQVN-Test-Host".to_string(),
        session_cookie: None,
        scope_files: true,
        scope_exec: true,
        scope_browser: false,
        scope_desktop: false,
        ping_interval_secs: 25,
        reconnect_delay_ms: 200,
        ..Default::default()
    };

    let client = GatewayClient::new(config);
    let (handle, join_handle) = client.start();

    handle
        .wait_for_handshake(Duration::from_secs(5))
        .await
        .expect("handshake");

    // 1. Send connector_gateway_control request without ack id
    let req = serde_json::json!({
        "id": "req-ctrl-1",
        "op_id": "op-ctrl-1",
        "action": "status"
    });
    tx_event
        .send(format!("42/ws,[\"connector_gateway_control\",{req}]"))
        .await
        .unwrap();

    let reply = tokio::time::timeout(Duration::from_secs(3), rx_reply.recv())
        .await
        .expect("reply timeout")
        .expect("reply recv");

    assert!(
        reply.starts_with("42/ws,[\"connector_gateway_control_result\""),
        "expected connector_gateway_control_result, got: {reply}"
    );

    let stripped = reply.strip_prefix("42/ws,").expect("prefix");
    let arr: Value = serde_json::from_str(stripped).expect("parse arr");
    let payload = arr.get(1).expect("payload in arr");

    assert_eq!(payload["ok"], true);
    assert_eq!(payload["status"], "ok");
    assert_eq!(payload["id"], "req-ctrl-1");
    assert_eq!(payload["op_id"], "op-ctrl-1");
    assert_eq!(payload["state"]["gateway_id"], "test-gw-ctrl-id");
    assert_eq!(payload["state"]["connected"], true);
    assert_eq!(payload["state"]["master_enabled"], true);
    assert_eq!(payload["state"]["scopes"]["files"], true);
    assert_eq!(payload["state"]["scopes"]["file_write"], true);
    assert_eq!(payload["state"]["scopes"]["code_execution"], true);
    assert_eq!(payload["state"]["scopes"]["browser"], false);
    assert_eq!(payload["state"]["scopes"]["computer_use"], false);

    // 2. Also send connector_gateway_control request WITH ack id 99
    let req2 = serde_json::json!({
        "id": "req-ctrl-2",
        "action": "query"
    });
    tx_event
        .send(format!("42/ws,99[\"connector_gateway_control\",{req2}]"))
        .await
        .unwrap();

    let reply2 = tokio::time::timeout(Duration::from_secs(3), rx_reply.recv())
        .await
        .expect("reply2 timeout")
        .expect("reply2 recv");

    assert!(
        reply2.starts_with("43/ws,99["),
        "expected ack 43/ws,99, got: {reply2}"
    );
    let stripped2 = reply2.strip_prefix("43/ws,99").expect("prefix");
    let arr2: Value = serde_json::from_str(stripped2).expect("parse arr2");
    let payload2 = arr2.as_array().and_then(|a| a.first()).expect("payload2");
    assert_eq!(payload2["ok"], true);
    assert_eq!(payload2["id"], "req-ctrl-2");

    handle.stop();
    let _ = join_handle.await;
    server_task.abort();
}
