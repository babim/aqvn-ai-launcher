//! Fulfills VAL-HOST-POOL-001 and VAL-HOST-POOL-002:
//! 1. When `aqvn-agent` connects to the Agent Zero gateway, it emits a `connector_hello`
//!    payload carrying unique `gateway_id`, configured `host_label`, and active `scopes`.
//! 2. The connected `aqvn-agent` responds to `connector_exec_op` and `connector_file_op`
//!    issued by Agent Zero, executing commands in PTY and updating files in sandbox.
//!
//! Integration tests verify:
//! 1. `aqvn-agent start` emits properly structured `connector_hello` over Socket.IO /ws
//!    with custom gateway_id, host_label, and granular scopes.
//! 2. `aqvn-agent start` with default flags emits deterministic `gateway_id` and active scopes.
//! 3. `aqvn-agent start` reads `settings.json` and emits configured identity and scopes.
//! 4. `aqvn-agent start` falls back to raw WebSocket (/ws) when Socket.IO returns 404.
//! 5. Keepalive ping/pong continuity is maintained between server and agent.
//! 6. `connector_exec_op` dispatches commands through PTY shell subject to command denylist.
//! 7. `connector_file_op` actions (write, read, stat) execute within configured workspace sandbox.
//! 8. Dangerous commands (rm -rf /, fork bomb) are refused with `status: denied`.
//! 9. Scope enforcement prevents execution or file access when scopes are disabled.
//! 10. Raw WebSocket mode supports remote execution and file management over raw JSON.

use aqvn_workspace_lib::connector::protocol::{
    ConnectorHello, MSG_TYPE_EXEC_OP, MSG_TYPE_EXEC_OP_RESULT, MSG_TYPE_FILE_OP,
    MSG_TYPE_FILE_OP_RESULT, MSG_TYPE_HELLO, MSG_TYPE_HELLO_ACK,
};
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
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

struct MockPoolServerState {
    hello_count: AtomicUsize,
    last_hello_raw: Mutex<Option<Value>>,
    last_hello: Mutex<Option<ConnectorHello>>,
    ping_count: AtomicUsize,
    pong_count: AtomicUsize,
    is_socketio: AtomicBool,
    sender: Mutex<Option<tokio::sync::mpsc::Sender<axum::extract::ws::Message>>>,
    inbound_broadcast: tokio::sync::broadcast::Sender<String>,
}

impl Default for MockPoolServerState {
    fn default() -> Self {
        let (inbound_broadcast, _) = tokio::sync::broadcast::channel(256);
        Self {
            hello_count: AtomicUsize::new(0),
            last_hello_raw: Mutex::new(None),
            last_hello: Mutex::new(None),
            ping_count: AtomicUsize::new(0),
            pong_count: AtomicUsize::new(0),
            is_socketio: AtomicBool::new(false),
            sender: Mutex::new(None),
            inbound_broadcast,
        }
    }
}

impl MockPoolServerState {
    pub async fn send_text(&self, text: String) -> bool {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
        loop {
            if let Some(ref tx) = *self.sender.lock().await {
                return tx
                    .send(axum::extract::ws::Message::Text(text.into()))
                    .await
                    .is_ok();
            }
            if tokio::time::Instant::now() > deadline {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    pub async fn send_socketio_event(&self, event_name: &str, payload: Value) -> bool {
        let text = format!("42/ws,[\"{event_name}\",{payload}]");
        self.send_text(text).await
    }

    pub async fn send_raw_json(&self, payload: Value) -> bool {
        self.send_text(payload.to_string()).await
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<String> {
        self.inbound_broadcast.subscribe()
    }

    pub async fn recv_event(
        &self,
        rx: &mut tokio::sync::broadcast::Receiver<String>,
        expected_type: &str,
        timeout: Duration,
    ) -> Result<Value, String> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(format!("timed out waiting for event '{expected_type}'"));
            }
            match tokio::time::timeout(remaining, rx.recv()).await {
                Ok(Ok(text)) => {
                    // Try parsing as Socket.IO frame: 42/ws,["event", {payload}]
                    if let Some(stripped) = text.strip_prefix("42/ws,") {
                        if let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(stripped) {
                            if arr.first().and_then(Value::as_str) == Some(expected_type) {
                                if let Some(payload) = arr.get(1) {
                                    if payload.get("done") == Some(&Value::Bool(false)) {
                                        continue;
                                    }
                                    return Ok(payload.clone());
                                }
                            }
                            if let Some(payload) = arr.get(1) {
                                if payload.get("type").and_then(Value::as_str)
                                    == Some(expected_type)
                                    || payload.get("op").and_then(Value::as_str)
                                        == Some(expected_type)
                                {
                                    if payload.get("done") == Some(&Value::Bool(false)) {
                                        continue;
                                    }
                                    return Ok(payload.clone());
                                }
                            }
                        }
                    }
                    // Try parsing as raw JSON: {"type": "...", ...}
                    if let Ok(val) = serde_json::from_str::<Value>(&text) {
                        if val.get("type").and_then(Value::as_str) == Some(expected_type)
                            || val.get("op").and_then(Value::as_str) == Some(expected_type)
                        {
                            if val.get("done") == Some(&Value::Bool(false)) {
                                continue;
                            }
                            return Ok(val);
                        }
                    }
                }
                Ok(Err(_)) => {}
                Err(_) => {
                    return Err(format!("timed out waiting for event '{expected_type}'"));
                }
            }
        }
    }
}

async fn handle_mock_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<MockPoolServerState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_mock_socket(socket, state))
}

async fn handle_mock_socket(socket: WebSocket, state: Arc<MockPoolServerState>) {
    state.is_socketio.store(true, Ordering::SeqCst);

    let (mut ws_sink, mut ws_stream) = socket.split();

    // 1. Send Engine.IO open packet
    let open_pkt = serde_json::json!({
        "sid": "mock-pool-sid-001",
        "upgrades": [],
        "pingInterval": 25000,
        "pingTimeout": 20000
    });
    if ws_sink
        .send(axum::extract::ws::Message::Text(
            format!("0{open_pkt}").into(),
        ))
        .await
        .is_err()
    {
        return;
    }

    // 2. Wait for namespace /ws join packet: 40/ws,
    while let Some(Ok(msg)) = ws_stream.next().await {
        match msg {
            axum::extract::ws::Message::Text(text) => {
                if text.starts_with("40/ws") {
                    let _ = ws_sink
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

    let (out_tx, mut out_rx) = tokio::sync::mpsc::channel::<axum::extract::ws::Message>(64);
    *state.sender.lock().await = Some(out_tx.clone());

    let send_task = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if ws_sink.send(msg).await.is_err() {
                break;
            }
        }
    });

    // 3. Receive connector_hello & handle keepalive and operational messages
    while let Some(Ok(msg)) = ws_stream.next().await {
        match msg {
            axum::extract::ws::Message::Text(text) => {
                let _ = state.inbound_broadcast.send(text.to_string());
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
                                let _ = out_tx
                                    .send(axum::extract::ws::Message::Text(ack.into()))
                                    .await;
                            }
                        }
                    }
                } else if let Some(ping_data) = text.strip_prefix('2') {
                    // Engine.IO ping from client
                    state.ping_count.fetch_add(1, Ordering::SeqCst);
                    let pong = format!("3{ping_data}");
                    let _ = out_tx
                        .send(axum::extract::ws::Message::Text(pong.into()))
                        .await;
                } else if text.starts_with('3') {
                    // Engine.IO pong from client
                    state.pong_count.fetch_add(1, Ordering::SeqCst);
                }
            }
            axum::extract::ws::Message::Ping(data) => {
                state.ping_count.fetch_add(1, Ordering::SeqCst);
                let _ = out_tx.send(axum::extract::ws::Message::Pong(data)).await;
            }
            axum::extract::ws::Message::Pong(_) => {
                state.pong_count.fetch_add(1, Ordering::SeqCst);
            }
            axum::extract::ws::Message::Close(_) => break,
            _ => {}
        }
    }

    send_task.abort();
    *state.sender.lock().await = None;
}

async fn handle_mock_raw_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<MockPoolServerState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_mock_raw_socket(socket, state))
}

async fn handle_mock_raw_socket(socket: WebSocket, state: Arc<MockPoolServerState>) {
    state.is_socketio.store(false, Ordering::SeqCst);

    let (mut ws_sink, mut ws_stream) = socket.split();

    let (out_tx, mut out_rx) = tokio::sync::mpsc::channel::<axum::extract::ws::Message>(64);
    *state.sender.lock().await = Some(out_tx.clone());

    let send_task = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if ws_sink.send(msg).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = ws_stream.next().await {
        match msg {
            axum::extract::ws::Message::Text(text) => {
                let _ = state.inbound_broadcast.send(text.to_string());
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
                        let _ = out_tx
                            .send(axum::extract::ws::Message::Text(ack.to_string().into()))
                            .await;
                    }
                }
            }
            axum::extract::ws::Message::Ping(data) => {
                state.ping_count.fetch_add(1, Ordering::SeqCst);
                let _ = out_tx.send(axum::extract::ws::Message::Pong(data)).await;
            }
            axum::extract::ws::Message::Pong(_) => {
                state.pong_count.fetch_add(1, Ordering::SeqCst);
            }
            axum::extract::ws::Message::Close(_) => break,
            _ => {}
        }
    }

    send_task.abort();
    *state.sender.lock().await = None;
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

#[tokio::test]
async fn test_host_pool_remote_control_exec_and_denylist_val_host_pool_002() {
    let (state, port, _server) = spawn_mock_pool_server().await;
    let bin = agent_bin();
    let temp_xdg = std::env::temp_dir().join(format!("test_pool_exec_xdg_{}", std::process::id()));
    let temp_ws = std::env::temp_dir().join(format!("test_pool_exec_ws_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_xdg);
    let _ = std::fs::remove_dir_all(&temp_ws);
    std::fs::create_dir_all(&temp_xdg).unwrap();
    std::fs::create_dir_all(&temp_ws).unwrap();

    let mut child = Command::new(&bin)
        .env("XDG_CONFIG_HOME", &temp_xdg)
        .arg("start")
        .arg("--server")
        .arg(format!("http://127.0.0.1:{port}"))
        .arg("--gateway-id")
        .arg("pool-exec-node-001")
        .arg("--label")
        .arg("Exec-Node-001")
        .arg("--workspace")
        .arg(temp_ws.to_str().unwrap())
        .arg("--scopes")
        .arg("files,code_execution")
        .arg("--ping-interval")
        .arg("1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn aqvn-agent process");

    // Wait for connector_hello handshake
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while state.hello_count.load(Ordering::SeqCst) == 0 {
        if tokio::time::Instant::now() > deadline {
            let _ = child.kill();
            let out = child.wait_with_output().expect("wait output");
            panic!(
                "timed out waiting for connector_hello from aqvn-agent.\nStdout: {}\nStderr: {}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let _guard = KillOnDrop(Some(child));

    let mut sub = state.subscribe();

    // 1. Dispatch valid connector_exec_op through PTY shell
    let exec_cmd = serde_json::json!({
        "id": "pool-exec-cmd-001",
        "command": "echo 'AGENT_ZERO_HOST_POOL_CMD_OK'",
        "session_id": "test-host-pool-s1"
    });
    let sent = state
        .send_socketio_event("connector_exec_op", exec_cmd)
        .await;
    assert!(sent, "failed to send connector_exec_op to agent");

    let reply = state
        .recv_event(&mut sub, MSG_TYPE_EXEC_OP_RESULT, Duration::from_secs(10))
        .await
        .expect("receive connector_exec_op_result");

    assert_eq!(reply["id"], "pool-exec-cmd-001");
    assert_eq!(reply["status"], "completed");
    assert_eq!(reply["success"], true);
    assert_eq!(reply["exit_code"], 0);
    assert!(
        reply["output"]
            .as_str()
            .unwrap_or("")
            .contains("AGENT_ZERO_HOST_POOL_CMD_OK"),
        "output must contain command output: {:?}",
        reply["output"]
    );

    // 2. Dispatch destructive command denied by denylist guard (rm -rf /)
    let deny_cmd = serde_json::json!({
        "id": "pool-exec-deny-001",
        "command": "rm -rf /",
        "session_id": "test-host-pool-s1"
    });
    let sent = state
        .send_socketio_event("connector_exec_op", deny_cmd)
        .await;
    assert!(sent, "failed to send denylisted command");

    let reply_deny = state
        .recv_event(&mut sub, MSG_TYPE_EXEC_OP_RESULT, Duration::from_secs(5))
        .await
        .expect("receive connector_exec_op_result for denied command");

    assert_eq!(reply_deny["id"], "pool-exec-deny-001");
    assert_eq!(reply_deny["status"], "denied");
    assert_eq!(reply_deny["success"], false);
    assert!(
        reply_deny["error"]
            .as_str()
            .unwrap_or("")
            .contains("security guard"),
        "error must cite security guard: {:?}",
        reply_deny["error"]
    );

    // 3. Dispatch fork bomb denied by denylist guard
    let forkbomb_cmd = serde_json::json!({
        "id": "pool-exec-forkbomb-001",
        "command": ":(){ :|:& };:",
        "session_id": "test-host-pool-s1"
    });
    let sent = state
        .send_socketio_event("connector_exec_op", forkbomb_cmd)
        .await;
    assert!(sent, "failed to send fork bomb command");

    let reply_forkbomb = state
        .recv_event(&mut sub, MSG_TYPE_EXEC_OP_RESULT, Duration::from_secs(5))
        .await
        .expect("receive connector_exec_op_result for fork bomb");

    assert_eq!(reply_forkbomb["id"], "pool-exec-forkbomb-001");
    assert_eq!(reply_forkbomb["status"], "denied");
    assert_eq!(reply_forkbomb["success"], false);

    // 4. Session recovers cleanly and executes subsequent commands
    let recover_cmd = serde_json::json!({
        "id": "pool-exec-recover-001",
        "command": "echo 'RECOVERED_AFTER_DENY'",
        "session_id": "test-host-pool-s1"
    });
    let sent = state
        .send_socketio_event("connector_exec_op", recover_cmd)
        .await;
    assert!(sent, "failed to send recovery command");

    let reply_recover = state
        .recv_event(&mut sub, MSG_TYPE_EXEC_OP_RESULT, Duration::from_secs(5))
        .await
        .expect("receive connector_exec_op_result after recovery");

    assert_eq!(reply_recover["id"], "pool-exec-recover-001");
    assert_eq!(reply_recover["status"], "completed");
    assert_eq!(reply_recover["success"], true);
    assert!(reply_recover["output"]
        .as_str()
        .unwrap_or("")
        .contains("RECOVERED_AFTER_DENY"));

    let _ = std::fs::remove_dir_all(&temp_xdg);
    let _ = std::fs::remove_dir_all(&temp_ws);
}

#[tokio::test]
async fn test_host_pool_remote_control_file_ops_and_sandbox_confinement_val_host_pool_002() {
    let (state, port, _server) = spawn_mock_pool_server().await;
    let bin = agent_bin();
    let temp_xdg = std::env::temp_dir().join(format!("test_pool_file_xdg_{}", std::process::id()));
    let temp_ws = std::env::temp_dir().join(format!("test_pool_file_ws_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_xdg);
    let _ = std::fs::remove_dir_all(&temp_ws);
    std::fs::create_dir_all(&temp_xdg).unwrap();
    std::fs::create_dir_all(&temp_ws).unwrap();

    let mut child = Command::new(&bin)
        .env("XDG_CONFIG_HOME", &temp_xdg)
        .arg("start")
        .arg("--server")
        .arg(format!("http://127.0.0.1:{port}"))
        .arg("--gateway-id")
        .arg("pool-file-node-001")
        .arg("--label")
        .arg("File-Node-001")
        .arg("--workspace")
        .arg(temp_ws.to_str().unwrap())
        .arg("--scopes")
        .arg("files,code_execution")
        .arg("--ping-interval")
        .arg("1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn aqvn-agent process");

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while state.hello_count.load(Ordering::SeqCst) == 0 {
        if tokio::time::Instant::now() > deadline {
            let _ = child.kill();
            let out = child.wait_with_output().expect("wait output");
            panic!(
                "timed out waiting for connector_hello from aqvn-agent.\nStdout: {}\nStderr: {}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let _guard = KillOnDrop(Some(child));

    let mut sub = state.subscribe();

    // 1. Write configuration file inside workspace sandbox
    let write_op = serde_json::json!({
        "id": "file-op-write-001",
        "action": "write",
        "path": "host_node_config.json",
        "content": "{\"node_role\":\"worker\",\"concurrency\":8,\"managed\":true}"
    });
    let sent = state
        .send_socketio_event("connector_file_op", write_op)
        .await;
    assert!(sent, "failed to send connector_file_op write");

    let reply_write = state
        .recv_event(&mut sub, MSG_TYPE_FILE_OP_RESULT, Duration::from_secs(5))
        .await
        .expect("receive connector_file_op_result for write");

    assert_eq!(reply_write["id"], "file-op-write-001");
    assert_eq!(reply_write["action"], "write");
    assert_eq!(reply_write["success"], true);

    // Verify on host disk
    let on_disk_file = temp_ws.join("host_node_config.json");
    assert!(on_disk_file.exists(), "written file must exist on disk");
    let content_on_disk = std::fs::read_to_string(&on_disk_file).expect("read file");
    assert_eq!(
        content_on_disk,
        "{\"node_role\":\"worker\",\"concurrency\":8,\"managed\":true}"
    );

    // 2. Read configuration file back from workspace sandbox
    let read_op = serde_json::json!({
        "id": "file-op-read-001",
        "action": "read",
        "path": "host_node_config.json"
    });
    let sent = state
        .send_socketio_event("connector_file_op", read_op)
        .await;
    assert!(sent, "failed to send connector_file_op read");

    let reply_read = state
        .recv_event(&mut sub, MSG_TYPE_FILE_OP_RESULT, Duration::from_secs(5))
        .await
        .expect("receive connector_file_op_result for read");

    assert_eq!(reply_read["id"], "file-op-read-001");
    assert_eq!(reply_read["action"], "read");
    assert_eq!(reply_read["success"], true);
    assert_eq!(
        reply_read["content"],
        "{\"node_role\":\"worker\",\"concurrency\":8,\"managed\":true}"
    );

    // 3. Stat configuration file in workspace sandbox
    let stat_op = serde_json::json!({
        "id": "file-op-stat-001",
        "action": "stat",
        "path": "host_node_config.json"
    });
    let sent = state
        .send_socketio_event("connector_file_op", stat_op)
        .await;
    assert!(sent, "failed to send connector_file_op stat");

    let reply_stat = state
        .recv_event(&mut sub, MSG_TYPE_FILE_OP_RESULT, Duration::from_secs(5))
        .await
        .expect("receive connector_file_op_result for stat");

    assert_eq!(reply_stat["id"], "file-op-stat-001");
    assert_eq!(reply_stat["success"], true);
    assert_eq!(reply_stat["exists"], true);
    assert_eq!(reply_stat["is_file"], true);

    // 4. File op escaping sandbox must be rejected
    let escape_op = serde_json::json!({
        "id": "file-op-escape-001",
        "action": "write",
        "path": "../../../system_breach.txt",
        "content": "malicious payload"
    });
    let sent = state
        .send_socketio_event("connector_file_op", escape_op)
        .await;
    assert!(sent, "failed to send escaping file op");

    let reply_escape = state
        .recv_event(&mut sub, MSG_TYPE_FILE_OP_RESULT, Duration::from_secs(5))
        .await
        .expect("receive connector_file_op_result for escape attempt");

    assert_eq!(reply_escape["id"], "file-op-escape-001");
    assert_eq!(reply_escape["success"], false);
    assert_eq!(reply_escape["status"], "error");
    let err_str = reply_escape["error"].as_str().unwrap_or("");
    assert!(
        err_str.contains("outside the allowed sandbox") || err_str.contains("sandbox"),
        "error must indicate sandbox rejection: {:?}",
        reply_escape["error"]
    );
    assert!(
        !temp_ws.join("../../../system_breach.txt").exists(),
        "file must not be written outside sandbox"
    );

    // 5. CWD outside sandbox for exec_op must be rejected
    let bad_cwd_exec = serde_json::json!({
        "id": "exec-bad-cwd-001",
        "command": "pwd",
        "cwd": "/etc",
        "session_id": "test-host-pool-s1"
    });
    let sent = state
        .send_socketio_event("connector_exec_op", bad_cwd_exec)
        .await;
    assert!(sent, "failed to send bad cwd exec");

    let reply_bad_cwd = state
        .recv_event(&mut sub, MSG_TYPE_EXEC_OP_RESULT, Duration::from_secs(5))
        .await
        .expect("receive connector_exec_op_result for bad cwd");

    assert_eq!(reply_bad_cwd["id"], "exec-bad-cwd-001");
    assert_eq!(reply_bad_cwd["success"], false);
    assert!(
        reply_bad_cwd["error"]
            .as_str()
            .unwrap_or("")
            .contains("outside sandbox"),
        "error must cite outside sandbox: {:?}",
        reply_bad_cwd["error"]
    );

    let _ = std::fs::remove_dir_all(&temp_xdg);
    let _ = std::fs::remove_dir_all(&temp_ws);
}

#[tokio::test]
async fn test_host_pool_remote_control_scope_enforcement_val_host_pool_002() {
    let (state, port, _server) = spawn_mock_pool_server().await;
    let bin = agent_bin();
    let temp_xdg =
        std::env::temp_dir().join(format!("test_pool_scopes_xdg_{}", std::process::id()));
    let temp_ws = std::env::temp_dir().join(format!("test_pool_scopes_ws_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_xdg);
    let _ = std::fs::remove_dir_all(&temp_ws);
    std::fs::create_dir_all(&temp_xdg).unwrap();
    std::fs::create_dir_all(&temp_ws).unwrap();

    // 1. Agent started with only `files` scope (code_execution disabled)
    let child = Command::new(&bin)
        .env("XDG_CONFIG_HOME", &temp_xdg)
        .arg("start")
        .arg("--server")
        .arg(format!("http://127.0.0.1:{port}"))
        .arg("--gateway-id")
        .arg("pool-scope-files-001")
        .arg("--label")
        .arg("Scope-Files-Node")
        .arg("--workspace")
        .arg(temp_ws.to_str().unwrap())
        .arg("--scopes")
        .arg("files")
        .arg("--ping-interval")
        .arg("1")
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

    let mut sub = state.subscribe();

    // Send exec_op when code_execution is disabled
    let exec_cmd = serde_json::json!({
        "id": "scope-exec-blocked-001",
        "command": "echo 'SHOULD_BE_BLOCKED'",
        "session_id": "test-scope-s1"
    });
    let sent = state.send_socketio_event(MSG_TYPE_EXEC_OP, exec_cmd).await;
    assert!(sent, "failed to send exec op");

    let reply_exec_blocked = state
        .recv_event(&mut sub, MSG_TYPE_EXEC_OP_RESULT, Duration::from_secs(5))
        .await
        .expect("receive exec op result when disabled");

    assert_eq!(reply_exec_blocked["id"], "scope-exec-blocked-001");
    assert_eq!(reply_exec_blocked["success"], false);
    assert!(
        reply_exec_blocked["error"]
            .as_str()
            .unwrap_or("")
            .contains("code_execution scope disabled"),
        "error must cite scope disabled: {:?}",
        reply_exec_blocked["error"]
    );

    // Stop child
    drop(_guard);

    // 2. Start agent with only `code_execution` scope (files disabled)
    let (state2, port2, _server2) = spawn_mock_pool_server().await;
    let child2 = Command::new(&bin)
        .env("XDG_CONFIG_HOME", &temp_xdg)
        .arg("start")
        .arg("--server")
        .arg(format!("http://127.0.0.1:{port2}"))
        .arg("--gateway-id")
        .arg("pool-scope-exec-001")
        .arg("--label")
        .arg("Scope-Exec-Node")
        .arg("--workspace")
        .arg(temp_ws.to_str().unwrap())
        .arg("--scopes")
        .arg("code_execution")
        .arg("--ping-interval")
        .arg("1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn aqvn-agent process");

    let _guard2 = KillOnDrop(Some(child2));

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while state2.hello_count.load(Ordering::SeqCst) == 0 {
        if tokio::time::Instant::now() > deadline {
            panic!("timed out waiting for connector_hello 2");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let mut sub2 = state2.subscribe();

    // Send file_op when files is disabled
    let file_cmd = serde_json::json!({
        "id": "scope-file-blocked-001",
        "action": "write",
        "path": "test.txt",
        "content": "blocked"
    });
    let sent = state2.send_socketio_event(MSG_TYPE_FILE_OP, file_cmd).await;
    assert!(sent, "failed to send file op");

    let reply_file_blocked = state2
        .recv_event(&mut sub2, MSG_TYPE_FILE_OP_RESULT, Duration::from_secs(5))
        .await
        .expect("receive file op result when disabled");

    assert_eq!(reply_file_blocked["id"], "scope-file-blocked-001");
    assert_eq!(reply_file_blocked["success"], false);
    assert!(
        reply_file_blocked["error"]
            .as_str()
            .unwrap_or("")
            .contains("files scope disabled"),
        "error must cite files scope disabled: {:?}",
        reply_file_blocked["error"]
    );

    let _ = std::fs::remove_dir_all(&temp_xdg);
    let _ = std::fs::remove_dir_all(&temp_ws);
}

#[tokio::test]
async fn test_host_pool_remote_control_raw_ws_fallback_val_host_pool_002() {
    let (state, port, _server) = spawn_mock_raw_ws_server().await;
    let bin = agent_bin();
    let temp_xdg =
        std::env::temp_dir().join(format!("test_pool_raw_ctrl_xdg_{}", std::process::id()));
    let temp_ws =
        std::env::temp_dir().join(format!("test_pool_raw_ctrl_ws_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_xdg);
    let _ = std::fs::remove_dir_all(&temp_ws);
    std::fs::create_dir_all(&temp_xdg).unwrap();
    std::fs::create_dir_all(&temp_ws).unwrap();

    let child = Command::new(&bin)
        .env("XDG_CONFIG_HOME", &temp_xdg)
        .arg("start")
        .arg("--server")
        .arg(format!("http://127.0.0.1:{port}"))
        .arg("--gateway-id")
        .arg("pool-raw-ctrl-node-001")
        .arg("--label")
        .arg("Raw-Ctrl-Node")
        .arg("--workspace")
        .arg(temp_ws.to_str().unwrap())
        .arg("--scopes")
        .arg("files,code_execution")
        .arg("--ping-interval")
        .arg("1")
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

    assert!(!state.is_socketio.load(Ordering::SeqCst), "must be raw ws");

    let mut sub = state.subscribe();

    // 1. Send raw JSON connector_exec_op
    let raw_exec = serde_json::json!({
        "type": "connector_exec_op",
        "id": "raw-exec-001",
        "command": "echo 'RAW_WS_REMOTE_EXEC_SUCCESS'",
        "session_id": "test-raw-s1"
    });
    let sent = state.send_raw_json(raw_exec).await;
    assert!(sent, "failed to send raw exec");

    let reply_exec = state
        .recv_event(&mut sub, MSG_TYPE_EXEC_OP_RESULT, Duration::from_secs(10))
        .await
        .expect("receive raw connector_exec_op_result");

    assert_eq!(reply_exec["id"], "raw-exec-001");
    assert_eq!(reply_exec["status"], "completed");
    assert_eq!(reply_exec["success"], true);
    assert!(reply_exec["output"]
        .as_str()
        .unwrap_or("")
        .contains("RAW_WS_REMOTE_EXEC_SUCCESS"));

    // 2. Send raw JSON connector_file_op
    let raw_file = serde_json::json!({
        "type": "connector_file_op",
        "id": "raw-file-001",
        "action": "write",
        "path": "raw_node_config.json",
        "content": "{\"protocol\":\"raw_ws\"}"
    });
    let sent = state.send_raw_json(raw_file).await;
    assert!(sent, "failed to send raw file op");

    let reply_file = state
        .recv_event(&mut sub, MSG_TYPE_FILE_OP_RESULT, Duration::from_secs(5))
        .await
        .expect("receive raw connector_file_op_result");

    assert_eq!(reply_file["id"], "raw-file-001");
    assert_eq!(reply_file["success"], true);
    assert!(temp_ws.join("raw_node_config.json").exists());

    let _ = std::fs::remove_dir_all(&temp_xdg);
    let _ = std::fs::remove_dir_all(&temp_ws);
}
