//! Outbound WebSocket client for AQVN Workspace Gateway Hub.
//!
//! Initiates an outbound WebSocket connection to upstream `/ws`, executes
//! the `connector_hello` handshake, maintains heartbeat pings, and automatically
//! reconnects on disconnection.

use crate::config::{AppConfig, ConfigWatcher};
use crate::connector::handlers;
use crate::connector::protocol::{
    format_socketio_ack, format_socketio_connect, format_socketio_event, parse_socketio_packet,
    ConnectorHello, SocketIoConnectAuth, EIO_PACKET_OPEN, EIO_PACKET_PING, EIO_PACKET_PONG,
    MSG_TYPE_HELLO, MSG_TYPE_HELLO_ACK, SCOPE_BROWSER, SCOPE_DESKTOP, SCOPE_EXEC, SCOPE_FILES,
    SIO_PACKET_ACK, SIO_PACKET_CONNECT_ERROR, SIO_PACKET_DISCONNECT, SIO_PACKET_EVENT,
};
use axum::http::header::{self, HeaderValue};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

pub const DEFAULT_PING_INTERVAL_SECS: u64 = 25;
pub const DEFAULT_RECONNECT_DELAY_MS: u64 = 500;

/// Resolves the upstream Socket.IO WebSocket endpoint (`/socket.io/?EIO=4&transport=websocket`).
pub fn build_ws_url(base_url: &str) -> Result<String, String> {
    let mut parsed =
        url::Url::parse(base_url).map_err(|e| format!("invalid URL {base_url}: {e}"))?;
    let new_scheme = match parsed.scheme() {
        "http" => "ws",
        "https" => "wss",
        "ws" => "ws",
        "wss" => "wss",
        other => return Err(format!("unsupported URL scheme: {other}")),
    };
    parsed
        .set_scheme(new_scheme)
        .map_err(|_| "failed to set websocket scheme".to_string())?;

    let path = parsed.path().trim_end_matches('/');
    let base_path = if path == "/ws" {
        ""
    } else if let Some(stripped) = path.strip_suffix("/ws") {
        stripped
    } else if let Some(stripped) = path.strip_suffix("/socket.io") {
        stripped
    } else {
        path
    };

    let full_path = if base_path.is_empty() {
        "/socket.io/".to_string()
    } else {
        format!("{base_path}/socket.io/")
    };

    parsed.set_path(&full_path);
    parsed.set_query(Some("EIO=4&transport=websocket"));

    Ok(parsed.to_string())
}

/// Resolves the legacy fallback WebSocket URL (`/ws`).
pub fn build_legacy_ws_url(base_url: &str) -> Result<String, String> {
    let mut parsed =
        url::Url::parse(base_url).map_err(|e| format!("invalid URL {base_url}: {e}"))?;
    let new_scheme = match parsed.scheme() {
        "http" => "ws",
        "https" => "wss",
        "ws" => "ws",
        "wss" => "wss",
        other => return Err(format!("unsupported URL scheme: {other}")),
    };
    parsed
        .set_scheme(new_scheme)
        .map_err(|_| "failed to set websocket scheme".to_string())?;

    let path = parsed.path().trim_end_matches('/');
    if path.is_empty() {
        parsed.set_path("/ws");
    } else if !path.ends_with("/ws") {
        parsed.set_path(&format!("{path}/ws"));
    }
    parsed.set_query(None);
    Ok(parsed.to_string())
}

/// Resolves the session cookie header for a target origin, checking explicit config first
/// then loading from the persisted CookieJar file on disk.
pub fn resolve_cookie_header(server_url: &str, session_cookie: Option<&str>) -> Option<String> {
    if let Some(cookie) = session_cookie {
        if !cookie.trim().is_empty() {
            return Some(cookie.trim().to_string());
        }
    }
    let cookie_file = crate::proxy::upstream::cookie_file_path(server_url);
    if cookie_file.exists() {
        if let Some(jar) = crate::proxy::jar::CookieJar::load_from_file(&cookie_file) {
            return jar.cookie_header();
        }
    }
    None
}

/// Gateway client configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GatewayConfig {
    pub enabled: bool,
    pub server_url: String,
    pub gateway_id: String,
    pub host_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_cookie: Option<String>,
    pub scope_files: bool,
    pub scope_exec: bool,
    pub scope_browser: bool,
    pub scope_desktop: bool,
    pub ping_interval_secs: u64,
    pub reconnect_delay_ms: u64,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            server_url: "https://a0.matmagoc.com".to_string(),
            gateway_id: crate::config::default_gateway_id(),
            host_label: "AQVN-Workspace-Host".to_string(),
            session_cookie: None,
            scope_files: true,
            scope_exec: true,
            scope_browser: true,
            scope_desktop: true,
            ping_interval_secs: DEFAULT_PING_INTERVAL_SECS,
            reconnect_delay_ms: DEFAULT_RECONNECT_DELAY_MS,
        }
    }
}

impl GatewayConfig {
    pub fn from_app_config(app_config: &AppConfig) -> Self {
        Self {
            enabled: app_config.connector_enabled,
            server_url: app_config.server_url.clone(),
            gateway_id: if app_config.gateway_id.trim().is_empty() {
                crate::config::default_gateway_id()
            } else {
                app_config.gateway_id.clone()
            },
            scope_files: app_config.scope_files,
            scope_exec: app_config.scope_exec,
            scope_browser: app_config.scope_browser,
            scope_desktop: app_config.scope_desktop,
            ..Default::default()
        }
    }

    pub fn active_scopes(&self) -> Vec<String> {
        let mut scopes = Vec::new();
        if self.scope_files {
            scopes.push(SCOPE_FILES.to_string());
        }
        if self.scope_exec {
            scopes.push(SCOPE_EXEC.to_string());
        }
        if self.scope_browser {
            scopes.push(SCOPE_BROWSER.to_string());
        }
        if self.scope_desktop {
            scopes.push(SCOPE_DESKTOP.to_string());
        }
        scopes
    }

    pub fn ws_url(&self) -> Result<String, String> {
        build_ws_url(&self.server_url)
    }
}

/// Handle to control and monitor a running gateway client connection.
#[derive(Clone)]
pub struct GatewayHandle {
    connected: Arc<AtomicBool>,
    handshake_rx: watch::Receiver<bool>,
    shutdown_tx: watch::Sender<bool>,
}

impl GatewayHandle {
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    pub fn is_handshake_complete(&self) -> bool {
        *self.handshake_rx.borrow()
    }

    pub async fn wait_for_handshake(&self, timeout: Duration) -> Result<(), String> {
        if *self.handshake_rx.borrow() {
            return Ok(());
        }
        let mut rx = self.handshake_rx.clone();
        tokio::time::timeout(timeout, async {
            while !*rx.borrow() {
                if rx.changed().await.is_err() {
                    return Err("gateway closed while waiting for handshake".to_string());
                }
            }
            Ok(())
        })
        .await
        .map_err(|_| format!("handshake timed out after {}ms", timeout.as_millis()))?
    }

    pub fn stop(&self) {
        let _ = self.shutdown_tx.send(true);
    }
}

/// Outbound gateway client.
#[derive(Clone)]
pub struct GatewayClient {
    pub config: GatewayConfig,
}

impl GatewayClient {
    pub fn new(config: GatewayConfig) -> Self {
        Self { config }
    }

    /// Spawns the client connection and reconnect supervisor loop.
    pub fn start(&self) -> (GatewayHandle, tokio::task::JoinHandle<()>) {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let (handshake_tx, handshake_rx) = watch::channel(false);
        let connected = Arc::new(AtomicBool::new(false));

        let handle = GatewayHandle {
            connected: Arc::clone(&connected),
            handshake_rx,
            shutdown_tx,
        };

        let client = self.clone();
        let join_handle = tokio::spawn(async move {
            client.run_loop(connected, handshake_tx, shutdown_rx).await;
        });

        (handle, join_handle)
    }

    async fn run_loop(
        &self,
        connected: Arc<AtomicBool>,
        handshake_tx: watch::Sender<bool>,
        mut shutdown_rx: watch::Receiver<bool>,
    ) {
        loop {
            if *shutdown_rx.borrow() {
                break;
            }

            let _ = self
                .connect_and_serve(&connected, &handshake_tx, &mut shutdown_rx)
                .await;

            connected.store(false, Ordering::SeqCst);
            let _ = handshake_tx.send(false);

            if *shutdown_rx.borrow() {
                break;
            }

            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        break;
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(self.config.reconnect_delay_ms)) => {}
            }
        }
    }

    async fn connect_and_serve(
        &self,
        connected: &Arc<AtomicBool>,
        handshake_tx: &watch::Sender<bool>,
        shutdown_rx: &mut watch::Receiver<bool>,
    ) -> Result<(), String> {
        let ws_url = self.config.ws_url()?;

        let build_request = |target_url: &str| -> Result<
            tokio_tungstenite::tungstenite::handshake::client::Request,
            String,
        > {
            let mut req = target_url
                .into_client_request()
                .map_err(|e| format!("failed to build client request: {e}"))?;

            if let Ok(parsed_url) = url::Url::parse(target_url) {
                if let Some(host) = parsed_url.host_str() {
                    let host_header = match parsed_url.port() {
                        Some(port) => format!("{host}:{port}"),
                        None => host.to_string(),
                    };
                    if let Ok(val) = HeaderValue::from_str(&host_header) {
                        req.headers_mut().insert(header::HOST, val);
                    }
                }
                let origin_scheme = match parsed_url.scheme() {
                    "wss" => "https",
                    _ => "http",
                };
                let origin_str = match parsed_url.port() {
                    Some(p) => format!(
                        "{}://{}:{}",
                        origin_scheme,
                        parsed_url.host_str().unwrap_or(""),
                        p
                    ),
                    None => format!(
                        "{}://{}",
                        origin_scheme,
                        parsed_url.host_str().unwrap_or("")
                    ),
                };
                if let Ok(val) = HeaderValue::from_str(&origin_str) {
                    req.headers_mut().insert(header::ORIGIN, val);
                }
            }
            req.headers_mut().insert(
                header::USER_AGENT,
                HeaderValue::from_static("AQVN-Workspace"),
            );

            if let Some(cookie) = resolve_cookie_header(
                &self.config.server_url,
                self.config.session_cookie.as_deref(),
            ) {
                if let Ok(val) = HeaderValue::from_str(&cookie) {
                    req.headers_mut().insert(header::COOKIE, val);
                }
            }

            Ok(req)
        };

        if *shutdown_rx.borrow() {
            return Ok(());
        }

        let request = build_request(&ws_url)?;

        let (mut stream, is_legacy) = match tokio_tungstenite::connect_async(request).await {
            Ok(pair) => (pair.0, false),
            Err(tokio_tungstenite::tungstenite::Error::Http(resp))
                if resp.status() == axum::http::StatusCode::NOT_FOUND =>
            {
                let legacy_url = build_legacy_ws_url(&self.config.server_url)?;
                let legacy_req = build_request(&legacy_url)?;
                let pair = tokio_tungstenite::connect_async(legacy_req)
                    .await
                    .map_err(|e| format!("websocket connect error to {legacy_url}: {e}"))?;
                (pair.0, true)
            }
            Err(e) => return Err(format!("websocket connect error to {ws_url}: {e}")),
        };

        connected.store(true, Ordering::SeqCst);

        let mut is_socketio = !is_legacy;
        let mut active_namespace: Option<String> = if is_socketio {
            Some("/ws".to_string())
        } else {
            None
        };

        let hello = ConnectorHello::new(
            &self.config.gateway_id,
            &self.config.host_label,
            self.config.active_scopes(),
        );
        let hello_json = serde_json::to_string(&hello)
            .map_err(|e| format!("failed to serialize connector_hello: {e}"))?;

        if is_socketio {
            // 1. Wait for Engine.IO open packet ('0')
            let mut open_packet_received = false;
            let open_start = tokio::time::Instant::now();
            let open_timeout = Duration::from_secs(5);

            while !open_packet_received && open_start.elapsed() < open_timeout {
                tokio::select! {
                    _ = shutdown_rx.changed() => {
                        let _ = stream.send(Message::Close(None)).await;
                        return Ok(());
                    }
                    frame = stream.next() => {
                        match frame {
                            Some(Ok(Message::Text(text))) => {
                                if text.starts_with(EIO_PACKET_OPEN) {
                                    open_packet_received = true;
                                } else if text.starts_with(EIO_PACKET_PING) {
                                    let pong = format!("3{}", &text[1..]);
                                    let _ = stream.send(Message::Text(pong.into())).await;
                                } else if text.contains(MSG_TYPE_HELLO) {
                                    is_socketio = false;
                                    break;
                                }
                            }
                            Some(Ok(Message::Ping(data))) => {
                                let _ = stream.send(Message::Pong(data)).await;
                            }
                            Some(Ok(Message::Close(_))) | None => {
                                return Err("connection closed waiting for engine.io open packet".to_string());
                            }
                            Some(Err(e)) => {
                                return Err(format!("error waiting for engine.io open packet: {e}"));
                            }
                            _ => {}
                        }
                    }
                }
            }

            if is_socketio && !open_packet_received {
                return Err("timed out waiting for engine.io open packet".to_string());
            }

            if is_socketio {
                // 2. Send namespace connect packet with auth payload:
                // 40/ws,{"handlers":["plugins/_a0_connector/ws_connector"],"client_type":"launcher"}
                let connect_packet = format_socketio_connect(
                    active_namespace.as_deref(),
                    Some(&SocketIoConnectAuth::default()),
                );
                stream
                    .send(Message::Text(connect_packet.into()))
                    .await
                    .map_err(|e| format!("failed to send namespace connect: {e}"))?;

                // 3. Wait for namespace connect ack (40/ws or 40)
                let mut ns_connected = false;
                let ns_start = tokio::time::Instant::now();
                let ns_timeout = Duration::from_secs(5);

                while !ns_connected && ns_start.elapsed() < ns_timeout {
                    tokio::select! {
                        _ = shutdown_rx.changed() => {
                            let _ = stream.send(Message::Close(None)).await;
                            return Ok(());
                        }
                        frame = stream.next() => {
                            match frame {
                                Some(Ok(Message::Text(text))) => {
                                    if text.starts_with("40/ws") {
                                        active_namespace = Some("/ws".to_string());
                                        ns_connected = true;
                                    } else if text.starts_with("40") {
                                        active_namespace = None;
                                        ns_connected = true;
                                    } else if text.starts_with(EIO_PACKET_PING) {
                                        let pong = format!("3{}", &text[1..]);
                                        let _ = stream.send(Message::Text(pong.into())).await;
                                    } else if text.starts_with("44") {
                                        return Err(format!("namespace connect error: {text}"));
                                    }
                                }
                                Some(Ok(Message::Ping(data))) => {
                                    let _ = stream.send(Message::Pong(data)).await;
                                }
                                Some(Ok(Message::Close(_))) | None => {
                                    return Err("connection closed waiting for namespace connect".to_string());
                                }
                                Some(Err(e)) => {
                                    return Err(format!("error waiting for namespace connect: {e}"));
                                }
                                _ => {}
                            }
                        }
                    }
                }

                if !ns_connected {
                    return Err("timed out waiting for namespace connect ack".to_string());
                }

                // 4. Send connector_hello event
                let hello_event =
                    format_socketio_event(active_namespace.as_deref(), MSG_TYPE_HELLO, &hello_json);
                stream
                    .send(Message::Text(hello_event.into()))
                    .await
                    .map_err(|e| format!("failed to send connector_hello event: {e}"))?;
            } else {
                stream
                    .send(Message::Text(hello_json.clone().into()))
                    .await
                    .map_err(|e| format!("failed to send connector_hello: {e}"))?;
            }
        } else {
            // Legacy /ws
            stream
                .send(Message::Text(hello_json.clone().into()))
                .await
                .map_err(|e| format!("failed to send connector_hello: {e}"))?;
        }

        // 5. Wait for handshake acknowledgment (connector_hello_ack)
        let ack_start = tokio::time::Instant::now();
        let ack_timeout = Duration::from_secs(10);
        let mut handshake_ok = false;

        while !handshake_ok && ack_start.elapsed() < ack_timeout {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    let _ = stream.send(Message::Close(None)).await;
                    return Ok(());
                }
                frame = stream.next() => {
                    match frame {
                        Some(Ok(Message::Text(ack_text))) => {
                            if ack_text.starts_with(EIO_PACKET_PING) {
                                let pong = format!("3{}", &ack_text[1..]);
                                let _ = stream.send(Message::Text(pong.into())).await;
                                continue;
                            }
                            if let Some(pkt) = parse_socketio_packet(&ack_text) {
                                is_socketio = true;
                                if pkt.packet_type == SIO_PACKET_EVENT {
                                    if let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(pkt.data) {
                                        let event_name = arr.first().and_then(Value::as_str).unwrap_or_default();
                                        let payload = arr.get(1).cloned().unwrap_or(Value::Null);
                                        let is_status_ok = payload.get("status").and_then(Value::as_str) == Some("ok")
                                            || payload.get("type").and_then(Value::as_str) == Some(MSG_TYPE_HELLO_ACK);
                                        if event_name == MSG_TYPE_HELLO_ACK || is_status_ok {
                                            handshake_ok = true;
                                            let _ = handshake_tx.send(true);
                                            break;
                                        }
                                    }
                                } else if pkt.packet_type == SIO_PACKET_ACK {
                                    handshake_ok = true;
                                    let _ = handshake_tx.send(true);
                                    break;
                                }
                            } else {
                                if ack_text.contains(MSG_TYPE_HELLO_ACK) || ack_text.contains("\"ok\"") {
                                    handshake_ok = true;
                                    let _ = handshake_tx.send(true);
                                    break;
                                }
                            }
                        }
                        Some(Ok(Message::Ping(data))) => {
                            let _ = stream.send(Message::Pong(data)).await;
                        }
                        Some(Ok(Message::Close(_))) | None => {
                            return Err("connection closed before handshake ack".to_string());
                        }
                        Some(Err(e)) => {
                            return Err(format!("handshake error: {e}"));
                        }
                        _ => {}
                    }
                }
            }
        }

        if !handshake_ok {
            return Err("handshake timed out waiting for acknowledgment".to_string());
        }

        let (mut write, mut read) = stream.split();
        let (tx_out, mut rx_out) = tokio::sync::mpsc::channel::<Message>(32);

        let mut ping_interval =
            tokio::time::interval(Duration::from_secs(self.config.ping_interval_secs));
        ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        ping_interval.tick().await;

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        let _ = write.send(Message::Close(None)).await;
                        break;
                    }
                }
                Some(msg) = rx_out.recv() => {
                    let msg_to_send = if is_socketio {
                        if let Message::Text(ref text) = msg {
                            if !text.starts_with("42") && !text.starts_with("43") && !text.starts_with("2") && !text.starts_with("3") && !text.starts_with("40") {
                                let event_name = serde_json::from_str::<Value>(text)
                                    .ok()
                                    .and_then(|v| v.get("type").or_else(|| v.get("op")).and_then(Value::as_str).map(String::from))
                                    .unwrap_or_else(|| "message".to_string());
                                Message::Text(format_socketio_event(active_namespace.as_deref(), &event_name, text).into())
                            } else {
                                msg
                            }
                        } else {
                            msg
                        }
                    } else {
                        msg
                    };
                    if write.send(msg_to_send).await.is_err() {
                        break;
                    }
                }
                _ = ping_interval.tick() => {
                    if write.send(Message::Ping(Default::default())).await.is_err() {
                        break;
                    }
                    if is_socketio && write.send(Message::Text("2".into())).await.is_err() {
                        break;
                    }
                }
                msg_res = read.next() => {
                    match msg_res {
                        Some(Ok(msg)) => {
                            match msg {
                                Message::Ping(data) => {
                                    if write.send(Message::Pong(data)).await.is_err() {
                                        break;
                                    }
                                }
                                Message::Pong(_) => {}
                                Message::Text(text) => {
                                    if text.starts_with(EIO_PACKET_PING) {
                                        let pong = format!("3{}", &text[1..]);
                                        if write.send(Message::Text(pong.into())).await.is_err() {
                                            break;
                                        }
                                    } else if text.starts_with(EIO_PACKET_PONG) {
                                    } else if let Some(packet) = parse_socketio_packet(&text) {
                                        match packet.packet_type {
                                            SIO_PACKET_EVENT => {
                                                if let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(packet.data) {
                                                    if let Some(event_name) = arr.first().and_then(Value::as_str) {
                                                        let payload = arr.get(1).cloned().unwrap_or(Value::Null);
                                                        let mut payload_obj = match payload {
                                                            Value::Object(map) => map,
                                                            _ => serde_json::Map::new(),
                                                        };
                                                        if !payload_obj.contains_key("type") && !payload_obj.contains_key("op") {
                                                            payload_obj.insert("op".to_string(), Value::String(event_name.to_string()));
                                                            payload_obj.insert("type".to_string(), Value::String(event_name.to_string()));
                                                        }
                                                        let dispatch_payload = serde_json::to_string(&Value::Object(payload_obj)).unwrap_or_default();
                                                        if let Some(reply) =
                                                            handlers::dispatch_message_scoped(&dispatch_payload, Some(&tx_out), &self.config).await
                                                        {
                                                            let reply_msg = if let Some(ack_id) = packet.ack_id {
                                                                format_socketio_ack(packet.namespace.or(active_namespace.as_deref()), ack_id, &reply)
                                                            } else {
                                                                let reply_val: Value = serde_json::from_str(&reply).unwrap_or(Value::Null);
                                                                let reply_event = reply_val.get("type")
                                                                    .or_else(|| reply_val.get("op"))
                                                                    .and_then(Value::as_str)
                                                                    .unwrap_or(event_name);
                                                                format_socketio_event(packet.namespace.or(active_namespace.as_deref()), reply_event, &reply)
                                                            };
                                                            let _ = tx_out.send(Message::Text(reply_msg.into())).await;
                                                        }
                                                    }
                                                }
                                            }
                                            SIO_PACKET_DISCONNECT => {
                                                break;
                                            }
                                            SIO_PACKET_CONNECT_ERROR => {
                                                break;
                                            }
                                            _ => {}
                                        }
                                    } else {
                                        if let Some(reply) =
                                            handlers::dispatch_message_scoped(&text, Some(&tx_out), &self.config).await
                                        {
                                            if is_socketio {
                                                let reply_val: Value = serde_json::from_str(&reply).unwrap_or(Value::Null);
                                                let reply_event = reply_val.get("type")
                                                    .or_else(|| reply_val.get("op"))
                                                    .and_then(Value::as_str)
                                                    .unwrap_or("message");
                                                let reply_sio = format_socketio_event(active_namespace.as_deref(), reply_event, &reply);
                                                let _ = tx_out.send(Message::Text(reply_sio.into())).await;
                                            } else {
                                                let _ = tx_out.send(Message::Text(reply.into())).await;
                                            }
                                        }
                                    }
                                }
                                Message::Binary(_) => {}
                                Message::Close(_) => {
                                    break;
                                }
                                Message::Frame(_) => {}
                            }
                        }
                        Some(Err(_)) | None => {
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

/// Supervisor running in background, watching configuration changes.
pub async fn run_supervised(watcher: ConfigWatcher) -> Result<(), String> {
    let mut receiver = watcher.subscribe();
    let mut current: Option<(GatewayHandle, tokio::task::JoinHandle<()>)> = None;

    loop {
        let app_config = receiver.borrow_and_update().clone();
        if let Some((handle, join_handle)) = current.take() {
            handle.stop();
            let _ = join_handle.await;
        }

        let config = GatewayConfig::from_app_config(&app_config);
        if config.enabled {
            let client = GatewayClient::new(config);
            current = Some(client.start());
        }

        if receiver.changed().await.is_err() {
            if let Some((handle, join_handle)) = current.take() {
                handle.stop();
                let _ = join_handle.await;
            }
            return Ok(());
        }
    }
}
