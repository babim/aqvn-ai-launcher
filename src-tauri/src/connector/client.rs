//! Outbound WebSocket client for AQVN Workspace Gateway Hub.
//!
//! Initiates an outbound WebSocket connection to upstream `/ws`, executes
//! the `connector_hello` handshake, maintains heartbeat pings, and automatically
//! reconnects on disconnection.

use crate::config::{AppConfig, ConfigWatcher};
use crate::connector::handlers;
use crate::connector::protocol::{
    ConnectorHello, SCOPE_BROWSER, SCOPE_DESKTOP, SCOPE_EXEC, SCOPE_FILES,
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
            gateway_id: format!("aqvn-gw-{:08x}", rand::random::<u32>()),
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

        let (mut stream, _response) = match tokio_tungstenite::connect_async(request).await {
            Ok(pair) => pair,
            Err(tokio_tungstenite::tungstenite::Error::Http(resp))
                if resp.status() == axum::http::StatusCode::NOT_FOUND =>
            {
                let legacy_url = build_legacy_ws_url(&self.config.server_url)?;
                let legacy_req = build_request(&legacy_url)?;
                tokio_tungstenite::connect_async(legacy_req)
                    .await
                    .map_err(|e| format!("websocket connect error to {legacy_url}: {e}"))?
            }
            Err(e) => return Err(format!("websocket connect error to {ws_url}: {e}")),
        };

        connected.store(true, Ordering::SeqCst);

        let mut is_socketio = false;

        let first_frame = tokio::select! {
            res = tokio::time::timeout(Duration::from_millis(500), stream.next()) => {
                match res {
                    Ok(Some(Ok(msg))) => Some(msg),
                    _ => None,
                }
            }
            _ = shutdown_rx.changed() => {
                return Ok(());
            }
        };

        let hello = ConnectorHello::new(
            &self.config.gateway_id,
            &self.config.host_label,
            self.config.active_scopes(),
        );
        let hello_json = serde_json::to_string(&hello)
            .map_err(|e| format!("failed to serialize connector_hello: {e}"))?;

        if let Some(Message::Text(text)) = first_frame {
            if text.starts_with('0') {
                is_socketio = true;

                stream
                    .send(Message::Text("40/ws,".into()))
                    .await
                    .map_err(|e| format!("failed to send namespace connect: {e}"))?;

                let ns_ack = tokio::select! {
                    res = tokio::time::timeout(Duration::from_secs(5), stream.next()) => {
                        res
                    }
                    _ = shutdown_rx.changed() => {
                        let _ = stream.send(Message::Close(None)).await;
                        return Ok(());
                    }
                };

                match ns_ack {
                    Ok(Some(Ok(Message::Text(ns_text)))) if ns_text.starts_with("40/ws") => {}
                    Ok(Some(Ok(other))) => {
                        return Err(format!("unexpected namespace connect response: {other:?}"));
                    }
                    Ok(Some(Err(e))) => {
                        return Err(format!("error waiting for namespace connect: {e}"));
                    }
                    Ok(None) => {
                        return Err("connection closed waiting for namespace connect".to_string());
                    }
                    Err(_) => {
                        return Err("timed out waiting for namespace connect ack".to_string());
                    }
                }

                let hello_event = format!("42/ws,[\"connector_hello\",{hello_json}]");
                stream
                    .send(Message::Text(hello_event.into()))
                    .await
                    .map_err(|e| format!("failed to send connector_hello event: {e}"))?;
            } else if text.starts_with("40/ws") {
                is_socketio = true;
                let hello_event = format!("42/ws,[\"connector_hello\",{hello_json}]");
                stream
                    .send(Message::Text(hello_event.into()))
                    .await
                    .map_err(|e| format!("failed to send connector_hello event: {e}"))?;
            } else {
                stream
                    .send(Message::Text(hello_json.into()))
                    .await
                    .map_err(|e| format!("failed to send connector_hello: {e}"))?;
            }
        } else {
            stream
                .send(Message::Text(hello_json.into()))
                .await
                .map_err(|e| format!("failed to send connector_hello: {e}"))?;
        }

        let ack_opt = tokio::select! {
            res = tokio::time::timeout(Duration::from_secs(10), stream.next()) => {
                res
            }
            _ = shutdown_rx.changed() => {
                let _ = stream.send(Message::Close(None)).await;
                return Ok(());
            }
        };

        match ack_opt {
            Ok(Some(Ok(Message::Text(ack_text)))) => {
                let ack_payload = if let Some(stripped) = ack_text.strip_prefix("42/ws,") {
                    is_socketio = true;
                    if let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(stripped) {
                        arr.get(1).cloned().unwrap_or(Value::Null)
                    } else {
                        Value::Null
                    }
                } else if ack_text.starts_with("43/ws,") {
                    is_socketio = true;
                    Value::Null
                } else {
                    serde_json::from_str::<Value>(&ack_text).unwrap_or(Value::Null)
                };

                let is_ack_ok = if ack_payload != Value::Null {
                    let msg_type = ack_payload
                        .get("type")
                        .or_else(|| ack_payload.get("op"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();
                    msg_type == "connector_hello_ack"
                        || ack_payload.get("status").and_then(|v| v.as_str()) == Some("ok")
                } else {
                    ack_text.contains("connector_hello_ack") || ack_text.contains("\"ok\"")
                };

                if is_ack_ok {
                    let _ = handshake_tx.send(true);
                } else {
                    return Err(format!("unexpected handshake response: {ack_text}"));
                }
            }
            Ok(Some(Ok(other))) => {
                return Err(format!("unexpected handshake frame type: {other:?}"));
            }
            Ok(Some(Err(e))) => {
                return Err(format!("handshake error: {e}"));
            }
            Ok(None) => {
                return Err("connection closed before handshake ack".to_string());
            }
            Err(_) => {
                return Err("handshake timed out waiting for acknowledgment".to_string());
            }
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
                            if !text.starts_with("42/ws,") && !text.starts_with("2") && !text.starts_with("3") && !text.starts_with("40/ws") {
                                let event_name = serde_json::from_str::<Value>(text)
                                    .ok()
                                    .and_then(|v| v.get("type").or_else(|| v.get("op")).and_then(Value::as_str).map(String::from))
                                    .unwrap_or_else(|| "message".to_string());
                                Message::Text(format!("42/ws,[\"{event_name}\",{text}]").into())
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
                                    if text == "2" || text.starts_with("2") {
                                        if write.send(Message::Text("3".into())).await.is_err() {
                                            break;
                                        }
                                    } else if text == "3" || text.starts_with("3") {
                                    } else if let Some(payload_slice) = text.strip_prefix("42/ws,") {
                                        if let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(payload_slice) {
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
                                                    let reply_val: Value = serde_json::from_str(&reply).unwrap_or(Value::Null);
                                                    let reply_event = reply_val.get("type")
                                                        .or_else(|| reply_val.get("op"))
                                                        .and_then(Value::as_str)
                                                        .unwrap_or(event_name);
                                                    let reply_sio = format!("42/ws,[\"{reply_event}\",{reply}]");
                                                    let _ = tx_out.send(Message::Text(reply_sio.into())).await;
                                                }
                                            }
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
                                                let reply_sio = format!("42/ws,[\"{reply_event}\",{reply}]");
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
