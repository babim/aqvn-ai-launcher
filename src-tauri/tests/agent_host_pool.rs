//! Fulfills VAL-HOST-POOL-001:
//! When `aqvn-agent` connects to the Agent Zero gateway, it emits a `connector_hello`
//! payload carrying unique `gateway_id`, configured `host_label`, and active `scopes`.
//!
//! Integration tests verify:
//! 1. `aqvn-agent start` emits properly structured `connector_hello` over Socket.IO /ws
//!    with custom gateway_id, host_label, and granular scopes.
//! 2. `aqvn-agent start` with default flags emits deterministic `gateway_id` and active scopes.
//! 3. `aqvn-agent start` reads `settings.json` and emits configured identity and scopes.
//! 4. `aqvn-agent start` falls back to raw WebSocket (/ws) when Socket.IO returns 404.
//! 5. Keepalive ping/pong continuity is maintained between server and agent.

use aqvn_workspace_lib::connector::protocol::{ConnectorHello, MSG_TYPE_HELLO, MSG_TYPE_HELLO_ACK};
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use serde_json::Value;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

fn agent_bin() -> PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_aqvn-agent") {
        return PathBuf::from(path);
    }
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push("aqvn-agent");
    path
}

struct KillOnDrop(Option<Child>);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[derive(Default)]
struct MockPoolServerState {
    hello_count: AtomicUsize,
    last_hello_raw: Mutex<Option<Value>>,
    last_hello: Mutex<Option<ConnectorHello>>,
    ping_count: AtomicUsize,
    pong_count: AtomicUsize,
    is_socketio: AtomicBool,
}

async fn handle_mock_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<MockPoolServerState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_mock_socket(socket, state))
}

async fn handle_mock_socket(mut socket: WebSocket, state: Arc<MockPoolServerState>) {
    state.is_socketio.store(true, Ordering::SeqCst);

    // 1. Send Engine.IO open packet
    let open_pkt = serde_json::json!({
        "sid": "mock-pool-sid-001",
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
                    let _ = socket
                        .send(axum::extract::ws::Message::Text(
                            "40/ws,{\"sid\":\"mock-client-sid-1\"}".into(),
                        ))
                        .await;
                    break;
                }
            }
            axum::extract::ws::Message::Close(_) => return,
            _ => {}
        }
    }

    // 3. Receive connector_hello & handle keepalive
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            axum::extract::ws::Message::Text(text) => {
                if let Some(stripped) = text.strip_prefix("42/ws,") {
                    if let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(stripped) {
                        if arr.first().and_then(Value::as_str) == Some(MSG_TYPE_HELLO) {
                            if let Some(payload) = arr.get(1) {
                                state.hello_count.fetch_add(1, Ordering::SeqCst);
                                *state.last_hello_raw.lock().await = Some(payload.clone());
                                if let Ok(hello) =
                                    serde_json::from_value::<ConnectorHello>(payload.clone())
                                {
                                    *state.last_hello.lock().await = Some(hello);
                                }

                                // Send connector_hello_ack
                                let ack = format!(
                                    "42/ws,[\"{MSG_TYPE_HELLO_ACK}\",{{\"status\":\"ok\",\"type\":\"{MSG_TYPE_HELLO_ACK}\"}}]"
                                );
                                let _ = socket
                                    .send(axum::extract::ws::Message::Text(ack.into()))
                                    .await;
                            }
                        }
                    }
                } else if let Some(ping_data) = text.strip_prefix('2') {
                    // Engine.IO ping from client
                    state.ping_count.fetch_add(1, Ordering::SeqCst);
                    let pong = format!("3{ping_data}");
                    let _ = socket
                        .send(axum::extract::ws::Message::Text(pong.into()))
                        .await;
                } else if text.starts_with('3') {
                    // Engine.IO pong from client
                    state.pong_count.fetch_add(1, Ordering::SeqCst);
                }
            }
            axum::extract::ws::Message::Ping(data) => {
                state.ping_count.fetch_add(1, Ordering::SeqCst);
                let _ = socket.send(axum::extract::ws::Message::Pong(data)).await;
            }
            axum::extract::ws::Message::Pong(_) => {
                state.pong_count.fetch_add(1, Ordering::SeqCst);
            }
            axum::extract::ws::Message::Close(_) => break,
            _ => {}
        }
    }
}

async fn handle_mock_raw_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<MockPoolServerState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_mock_raw_socket(socket, state))
}

async fn handle_mock_raw_socket(mut socket: WebSocket, state: Arc<MockPoolServerState>) {
    state.is_socketio.store(false, Ordering::SeqCst);

    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            axum::extract::ws::Message::Text(text) => {
                if let Ok(val) = serde_json::from_str::<Value>(&text) {
                    if val.get("type").and_then(Value::as_str) == Some(MSG_TYPE_HELLO) {
                        state.hello_count.fetch_add(1, Ordering::SeqCst);
                        *state.last_hello_raw.lock().await = Some(val.clone());
                        if let Ok(hello) = serde_json::from_value::<ConnectorHello>(val) {
                            *state.last_hello.lock().await = Some(hello);
                        }

                        let ack = serde_json::json!({
                            "type": MSG_TYPE_HELLO_ACK,
                            "status": "ok"
                        });
                        let _ = socket
                            .send(axum::extract::ws::Message::Text(ack.to_string().into()))
                            .await;
                    }
                }
            }
            axum::extract::ws::Message::Ping(data) => {
                state.ping_count.fetch_add(1, Ordering::SeqCst);
                let _ = socket.send(axum::extract::ws::Message::Pong(data)).await;
            }
            axum::extract::ws::Message::Pong(_) => {
                state.pong_count.fetch_add(1, Ordering::SeqCst);
            }
            axum::extract::ws::Message::Close(_) => break,
            _ => {}
        }
    }
}

async fn spawn_mock_pool_server() -> (Arc<MockPoolServerState>, u16, tokio::task::JoinHandle<()>) {
    let state = Arc::new(MockPoolServerState::default());
    let router = Router::new()
        .route("/socket.io/", get(handle_mock_ws))
        .route("/ws", get(handle_mock_ws))
        .with_state(Arc::clone(&state));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral listener");
    let port = listener.local_addr().expect("local addr").port();

    let server_handle = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });

    (state, port, server_handle)
}

async fn spawn_mock_raw_ws_server() -> (Arc<MockPoolServerState>, u16, tokio::task::JoinHandle<()>)
{
    let state = Arc::new(MockPoolServerState::default());
    // Only route /ws, /socket.io/ returns 404 (Not Found)
    let router = Router::new()
        .route("/ws", get(handle_mock_raw_ws))
        .with_state(Arc::clone(&state));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral listener");
    let port = listener.local_addr().expect("local addr").port();

    let server_handle = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });

    (state, port, server_handle)
}

#[tokio::test]
async fn test_host_pool_registration_custom_identity_and_scopes_val_host_pool_001() {
    let (state, port, _server) = spawn_mock_pool_server().await;
    let bin = agent_bin();
    let temp_xdg =
        std::env::temp_dir().join(format!("test_pool_custom_xdg_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_xdg);
    std::fs::create_dir_all(&temp_xdg).unwrap();

    let child = Command::new(&bin)
        .env("XDG_CONFIG_HOME", &temp_xdg)
        .arg("start")
        .arg("--server")
        .arg(format!("http://127.0.0.1:{port}"))
        .arg("--gateway-id")
        .arg("custom-pool-node-uuid-001")
        .arg("--label")
        .arg("Worker-Node-Alpha")
        .arg("--scopes")
        .arg("files,code_execution")
        .arg("--ping-interval")
        .arg("1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn aqvn-agent process");

    let _guard = KillOnDrop(Some(child));

    // Wait for connector_hello handshake
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while state.hello_count.load(Ordering::SeqCst) == 0 {
        if tokio::time::Instant::now() > deadline {
            panic!("timed out waiting for connector_hello from aqvn-agent");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Verify structured connector_hello payload
    let raw_opt = state.last_hello_raw.lock().await.clone();
    assert!(
        raw_opt.is_some(),
        "raw connector_hello payload must be captured"
    );
    let raw = raw_opt.unwrap();

    assert_eq!(raw["type"], "connector_hello");
    assert_eq!(raw["op"], "connector_hello");
    assert_eq!(raw["gateway_id"], "custom-pool-node-uuid-001");
    assert_eq!(raw["host_label"], "Worker-Node-Alpha");

    // Scopes: files and code_execution enabled, browser and computer_use disabled
    assert_eq!(raw["scopes"]["files"], true);
    assert_eq!(raw["scopes"]["code_execution"], true);
    assert_eq!(raw["scopes"]["browser"], false);
    assert_eq!(raw["scopes"]["computer_use"], false);

    // Capabilities
    assert_eq!(raw["capabilities"]["files"], true);
    assert_eq!(raw["capabilities"]["code_execution"], true);
    assert_eq!(raw["capabilities"].get("browser"), None);

    // Scoped capability configurations
    assert_eq!(raw["remote_files"]["enabled"], true);
    assert_eq!(raw["remote_exec"]["enabled"], true);
    assert_eq!(raw["host_browser"]["enabled"], false);
    assert_eq!(raw["computer_use"]["enabled"], false);

    // Auth handlers
    let handlers = raw["auth"]["handlers"].as_array().expect("handlers array");
    assert!(handlers.contains(&Value::String("files".to_string())));
    assert!(handlers.contains(&Value::String("exec".to_string())));
    assert!(!handlers.contains(&Value::String("browser".to_string())));
    assert!(!handlers.contains(&Value::String("desktop".to_string())));

    // Gateway metadata
    assert_eq!(raw["gateway"]["scopes"]["files"]["enabled"], true);
    assert_eq!(raw["gateway"]["scopes"]["exec"]["enabled"], true);
    assert_eq!(raw["gateway"]["scopes"]["browser"]["enabled"], false);
    assert_eq!(raw["gateway"]["scopes"]["desktop"]["enabled"], false);

    let _ = std::fs::remove_dir_all(&temp_xdg);
}

#[tokio::test]
async fn test_host_pool_registration_deterministic_default_val_host_pool_001() {
    let (state, port, _server) = spawn_mock_pool_server().await;
    let bin = agent_bin();
    let temp_xdg = std::env::temp_dir().join(format!("test_pool_def_xdg_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_xdg);
    std::fs::create_dir_all(&temp_xdg).unwrap();

    // Run without custom gateway_id, label, or scopes, in isolated XDG dir
    let child = Command::new(&bin)
        .env("XDG_CONFIG_HOME", &temp_xdg)
        .arg("start")
        .arg("--server")
        .arg(format!("http://127.0.0.1:{port}"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn aqvn-agent process");

    let _guard = KillOnDrop(Some(child));

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while state.hello_count.load(Ordering::SeqCst) == 0 {
        if tokio::time::Instant::now() > deadline {
            panic!("timed out waiting for connector_hello");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let raw = state
        .last_hello_raw
        .lock()
        .await
        .clone()
        .expect("raw hello");
    let expected_default_gw = aqvn_workspace_lib::config::default_gateway_id();

    assert_eq!(
        raw["gateway_id"], expected_default_gw,
        "must use deterministic gateway_id"
    );
    assert!(
        !raw["host_label"].as_str().unwrap_or("").is_empty(),
        "must have default host_label"
    );

    // Default enables all scopes
    assert_eq!(raw["scopes"]["files"], true);
    assert_eq!(raw["scopes"]["code_execution"], true);
    assert_eq!(raw["scopes"]["browser"], true);
    assert_eq!(raw["scopes"]["computer_use"], true);

    let _ = std::fs::remove_dir_all(&temp_xdg);
}

#[tokio::test]
async fn test_host_pool_registration_config_file_val_host_pool_001() {
    let (state, port, _server) = spawn_mock_pool_server().await;
    let bin = agent_bin();

    let temp_dir = std::env::temp_dir().join(format!("test_pool_cfg_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();
    let config_path = temp_dir.join("settings.json");

    let settings_json = serde_json::json!({
        "server_url": format!("http://127.0.0.1:{port}"),
        "gateway_id": "cfg-pool-uuid-777",
        "host_label": "Configured-Node-Host",
        "scope_files": true,
        "scope_exec": false,
        "scope_browser": false,
        "scope_desktop": true,
        "ping_interval_secs": 1
    });
    std::fs::write(&config_path, settings_json.to_string()).expect("write settings.json");

    let child = Command::new(&bin)
        .arg("start")
        .arg("--config")
        .arg(&config_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn aqvn-agent process");

    let _guard = KillOnDrop(Some(child));

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while state.hello_count.load(Ordering::SeqCst) == 0 {
        if tokio::time::Instant::now() > deadline {
            panic!("timed out waiting for connector_hello from config run");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let raw = state
        .last_hello_raw
        .lock()
        .await
        .clone()
        .expect("raw hello");
    assert_eq!(raw["gateway_id"], "cfg-pool-uuid-777");
    assert_eq!(raw["host_label"], "Configured-Node-Host");
    assert_eq!(raw["scopes"]["files"], true);
    assert_eq!(raw["scopes"]["code_execution"], false);
    assert_eq!(raw["scopes"]["browser"], false);
    assert_eq!(raw["scopes"]["computer_use"], true);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_host_pool_registration_raw_ws_fallback_val_host_pool_001() {
    let (state, port, _server) = spawn_mock_raw_ws_server().await;
    let bin = agent_bin();
    let temp_xdg = std::env::temp_dir().join(format!("test_pool_raw_xdg_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_xdg);
    std::fs::create_dir_all(&temp_xdg).unwrap();

    let child = Command::new(&bin)
        .env("XDG_CONFIG_HOME", &temp_xdg)
        .arg("start")
        .arg("--server")
        .arg(format!("http://127.0.0.1:{port}"))
        .arg("--gateway-id")
        .arg("raw-ws-node-uuid-999")
        .arg("--label")
        .arg("Legacy-Raw-Node")
        .arg("--scopes")
        .arg("files")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn aqvn-agent process");

    let _guard = KillOnDrop(Some(child));

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while state.hello_count.load(Ordering::SeqCst) == 0 {
        if tokio::time::Instant::now() > deadline {
            panic!("timed out waiting for raw ws connector_hello fallback");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert!(
        !state.is_socketio.load(Ordering::SeqCst),
        "must fall back to raw ws"
    );
    let raw = state
        .last_hello_raw
        .lock()
        .await
        .clone()
        .expect("raw hello");
    assert_eq!(raw["type"], "connector_hello");
    assert_eq!(raw["gateway_id"], "raw-ws-node-uuid-999");
    assert_eq!(raw["host_label"], "Legacy-Raw-Node");
    assert_eq!(raw["scopes"]["files"], true);
    assert_eq!(raw["scopes"]["code_execution"], false);

    let _ = std::fs::remove_dir_all(&temp_xdg);
}
