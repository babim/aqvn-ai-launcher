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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

pub const DEFAULT_PING_INTERVAL_SECS: u64 = 25;
pub const DEFAULT_RECONNECT_DELAY_MS: u64 = 500;

/// Resolves the upstream WebSocket URL with the `/ws` endpoint path.
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
    if path.is_empty() {
        parsed.set_path("/ws");
    } else if !path.ends_with("/ws") {
        parsed.set_path(&format!("{path}/ws"));
    }
    Ok(parsed.to_string())
}

/// Gateway client configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GatewayConfig {
    pub enabled: bool,
    pub server_url: String,
    pub gateway_id: String,
    pub host_label: String,
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
            server_url: app_config.server_url.clone(),
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
        let mut request = ws_url
            .as_str()
            .into_client_request()
            .map_err(|e| format!("failed to build client request: {e}"))?;

        if let Ok(parsed_url) = url::Url::parse(&ws_url) {
            if let Some(host) = parsed_url.host_str() {
                let host_header = match parsed_url.port() {
                    Some(port) => format!("{host}:{port}"),
                    None => host.to_string(),
                };
                if let Ok(val) = HeaderValue::from_str(&host_header) {
                    request.headers_mut().insert(header::HOST, val);
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
                request.headers_mut().insert(header::ORIGIN, val);
            }
        }
        request.headers_mut().insert(
            header::USER_AGENT,
            HeaderValue::from_static("AQVN-Workspace"),
        );

        let (mut stream, _response) = tokio_tungstenite::connect_async(request)
            .await
            .map_err(|e| format!("websocket connect error to {ws_url}: {e}"))?;

        connected.store(true, Ordering::SeqCst);

        // Send connector_hello
        let hello = ConnectorHello::new(
            &self.config.gateway_id,
            &self.config.host_label,
            self.config.active_scopes(),
        );
        let hello_json = serde_json::to_string(&hello)
            .map_err(|e| format!("failed to serialize connector_hello: {e}"))?;

        stream
            .send(Message::Text(hello_json.into()))
            .await
            .map_err(|e| format!("failed to send connector_hello: {e}"))?;

        // Wait for acknowledgment message
        match tokio::time::timeout(Duration::from_secs(10), stream.next()).await {
            Ok(Some(Ok(Message::Text(ack_text)))) => {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&ack_text) {
                    let msg_type = val
                        .get("type")
                        .or_else(|| val.get("op"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();
                    if msg_type == "connector_hello_ack"
                        || val.get("status").and_then(|v| v.as_str()) == Some("ok")
                    {
                        let _ = handshake_tx.send(true);
                    } else {
                        return Err(format!("unexpected handshake response: {ack_text}"));
                    }
                } else {
                    return Err(format!("non-json handshake response: {ack_text}"));
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

        // Split stream for concurrent ping and message handling
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
                    if write.send(msg).await.is_err() {
                        break;
                    }
                }
                _ = ping_interval.tick() => {
                    if write.send(Message::Ping(Default::default())).await.is_err() {
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
                                    if let Some(reply) =
                                        handlers::dispatch_message_streaming(&text, Some(&tx_out)).await
                                    {
                                        let _ = tx_out.send(Message::Text(reply.into())).await;
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
