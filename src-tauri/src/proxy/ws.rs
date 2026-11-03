//! Two-leg WebSocket proxy: browser ⇄ this process ⇄ target chat server.
//!
//! Frames are forwarded verbatim, so Engine.IO handshake, namespace CONNECT,
//! `state_request`/ACK traffic, and the 45 s ping cadence all pass through
//! unchanged. Only the transport is re-established upstream, where the cookie
//! jar and spoofed `Origin` satisfy the server's handshake checks.

use crate::proxy::upstream::Upstream;
use axum::extract::ws::{CloseFrame as ClientCloseFrame, Message as ClientMessage, WebSocket};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::tungstenite::{
    protocol::CloseFrame as ServerCloseFrame, Message as ServerMessage, Utf8Bytes,
};

pub const HEARTBEAT_INTERVAL_SECS: u64 = 15;
pub const INACTIVITY_TIMEOUT_SECS: u64 = 60;
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(HEARTBEAT_INTERVAL_SECS);
pub const INACTIVITY_TIMEOUT: Duration = Duration::from_secs(INACTIVITY_TIMEOUT_SECS);
pub const WATCHDOG_PROBE: &[u8] = b"aqvn-heartbeat";

/// Parses heartbeat interval and inactivity timeout overrides from query parameters
/// or environment variables, returning (heartbeat_interval, inactivity_timeout, clean_path_and_query).
pub fn parse_watchdog_config(path_and_query: &str) -> (Duration, Duration, String) {
    let mut heartbeat = std::env::var("AQVN_WS_HEARTBEAT_INTERVAL_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(HEARTBEAT_INTERVAL);

    let mut timeout = std::env::var("AQVN_WS_INACTIVITY_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(INACTIVITY_TIMEOUT);

    let (path, query) = match path_and_query.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (path_and_query, None),
    };

    let clean_path = if let Some(q) = query {
        let mut clean_pairs = Vec::new();
        for (k, v) in url::form_urlencoded::parse(q.as_bytes()) {
            match k.as_ref() {
                "inactivity_timeout_ms" | "watchdog_timeout_ms" => {
                    if let Ok(ms) = v.parse::<u64>() {
                        timeout = Duration::from_millis(ms);
                    }
                }
                "heartbeat_interval_ms" | "watchdog_interval_ms" => {
                    if let Ok(ms) = v.parse::<u64>() {
                        heartbeat = Duration::from_millis(ms);
                    }
                }
                _ => clean_pairs.push((k, v)),
            }
        }
        if clean_pairs.is_empty() {
            path.to_string()
        } else {
            let mut encoded = String::new();
            let mut serializer = url::form_urlencoded::Serializer::new(&mut encoded);
            for (k, v) in clean_pairs {
                serializer.append_pair(&k, &v);
            }
            format!("{path}?{encoded}")
        }
    } else {
        path_and_query.to_string()
    };

    (heartbeat, timeout, clean_path)
}

/// Runs until either leg closes or watchdog timeout expires, then closes the other side.
pub async fn bridge(socket: WebSocket, upstream: Arc<Upstream>, path_and_query: String) {
    let (heartbeat, timeout, clean_path) = parse_watchdog_config(&path_and_query);
    bridge_with_watchdog(socket, upstream, clean_path, heartbeat, timeout).await;
}

/// Runs WebSocket bridge with explicit watchdog heartbeat and inactivity timeout.
pub async fn bridge_with_watchdog(
    socket: WebSocket,
    upstream: Arc<Upstream>,
    path_and_query: String,
    heartbeat_interval: Duration,
    inactivity_timeout: Duration,
) {
    let server = match upstream.connect_ws(&path_and_query).await {
        Ok(stream) => stream,
        Err(err) => {
            eprintln!("Bridge proxy: {err}");
            let close_frame = ClientCloseFrame {
                code: 1011,
                reason: err.into(),
            };
            let mut socket = socket;
            let _ = socket.send(ClientMessage::Close(Some(close_frame))).await;
            return;
        }
    };
    println!(
        "Bridge proxy: websocket connected to {}",
        upstream.target().ws_url(&path_and_query)
    );

    let (mut client_sink, mut client_stream) = socket.split();
    let (mut server_sink, mut server_stream) = server.split();

    let mut last_client_activity = tokio::time::Instant::now();
    let mut last_server_activity = tokio::time::Instant::now();
    let mut interval = tokio::time::interval(heartbeat_interval);
    interval.tick().await;

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let now = tokio::time::Instant::now();
                let client_elapsed = now.duration_since(last_client_activity);
                let server_elapsed = now.duration_since(last_server_activity);

                if client_elapsed >= inactivity_timeout || server_elapsed >= inactivity_timeout {
                    println!(
                        "Bridge proxy: websocket inactivity timeout (client: {:.1}s, server: {:.1}s >= {:.1}s), terminating bridge",
                        client_elapsed.as_secs_f64(),
                        server_elapsed.as_secs_f64(),
                        inactivity_timeout.as_secs_f64()
                    );
                    break;
                }

                let probe = axum::body::Bytes::from_static(WATCHDOG_PROBE);
                if client_sink.send(ClientMessage::Ping(probe.clone())).await.is_err() {
                    println!("Bridge proxy: failed to send ping probe to client, terminating bridge");
                    break;
                }
                if server_sink.send(ServerMessage::Ping(probe)).await.is_err() {
                    println!("Bridge proxy: failed to send ping probe to server, terminating bridge");
                    break;
                }
            }
            message = client_stream.next() => {
                let Some(message) = message else { break };
                let Ok(message) = message else { break };
                last_client_activity = tokio::time::Instant::now();
                if is_watchdog_pong(&message) {
                    continue;
                }
                let Some(message) = to_server(message) else { continue };
                log_engineio("browser->server", &message);
                if server_sink.send(message).await.is_err() {
                    break;
                }
            }
            message = server_stream.next() => {
                let Some(message) = message else { break };
                let Ok(message) = message else { break };
                last_server_activity = tokio::time::Instant::now();
                if is_server_watchdog_pong(&message) {
                    continue;
                }
                log_engineio("server->browser", &message);
                let Some(message) = to_client(message) else { continue };
                if client_sink.send(message).await.is_err() {
                    break;
                }
            }
        }
    }

    let _ = server_sink.send(ServerMessage::Close(None)).await;
    let _ = client_sink.send(ClientMessage::Close(None)).await;
}

fn is_watchdog_pong(message: &ClientMessage) -> bool {
    matches!(message, ClientMessage::Pong(bytes) if bytes.as_ref() == WATCHDOG_PROBE)
}

fn is_server_watchdog_pong(message: &ServerMessage) -> bool {
    matches!(message, ServerMessage::Pong(bytes) if bytes.as_ref() == WATCHDOG_PROBE)
}

/// Engine.IO pings are text frames `"2"`; the client answers `"3"`.
fn log_engineio(direction: &str, message: &ServerMessage) {
    match message {
        ServerMessage::Text(text) if text.as_str() == "2" => {
            println!("Bridge proxy: engine.io ping {direction} (45s keepalive)")
        }
        ServerMessage::Text(text) if text.as_str() == "3" => {
            println!("Bridge proxy: engine.io pong {direction}")
        }
        ServerMessage::Ping(_) => println!("Bridge proxy: websocket ping {direction}"),
        ServerMessage::Pong(_) => println!("Bridge proxy: websocket pong {direction}"),
        _ => {}
    }
}

fn to_server(message: ClientMessage) -> Option<ServerMessage> {
    match message {
        ClientMessage::Text(text) => Some(ServerMessage::Text(Utf8Bytes::from(text.to_string()))),
        ClientMessage::Binary(bytes) => Some(ServerMessage::Binary(bytes)),
        ClientMessage::Ping(bytes) => Some(ServerMessage::Ping(bytes)),
        ClientMessage::Pong(bytes) => Some(ServerMessage::Pong(bytes)),
        ClientMessage::Close(frame) => Some(ServerMessage::Close(frame.map(to_server_close))),
    }
}

fn to_client(message: ServerMessage) -> Option<ClientMessage> {
    match message {
        ServerMessage::Text(text) => Some(ClientMessage::Text(text.to_string().into())),
        ServerMessage::Binary(bytes) => Some(ClientMessage::Binary(bytes)),
        ServerMessage::Ping(bytes) => Some(ClientMessage::Ping(bytes)),
        ServerMessage::Pong(bytes) => Some(ClientMessage::Pong(bytes)),
        ServerMessage::Close(frame) => Some(ClientMessage::Close(frame.map(to_client_close))),
        ServerMessage::Frame(_) => None,
    }
}

fn to_server_close(frame: ClientCloseFrame) -> ServerCloseFrame {
    ServerCloseFrame {
        code: frame.code.into(),
        reason: Utf8Bytes::from(frame.reason.to_string()),
    }
}

fn to_client_close(frame: ServerCloseFrame) -> ClientCloseFrame {
    ClientCloseFrame {
        code: frame.code.into(),
        reason: frame.reason.as_str().into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_and_binary_frames_round_trip() {
        let text = ClientMessage::Text("2".to_string().into());
        let converted = to_server(text).expect("text frame");
        assert!(matches!(converted, ServerMessage::Text(ref value) if value.as_str() == "2"));

        let binary = ClientMessage::Binary(vec![0, 1, 2].into());
        let converted = to_client(to_server(binary).expect("binary frame")).expect("binary back");
        assert!(
            matches!(converted, ClientMessage::Binary(ref value) if value.as_ref() == [0, 1, 2])
        );
    }

    #[test]
    fn close_frames_preserve_code_and_reason() {
        let close = ClientMessage::Close(Some(ClientCloseFrame {
            code: 1000,
            reason: "done".to_string().into(),
        }));
        let converted = to_server(close).expect("close frame");
        let back = to_client(converted).expect("close back");
        let ClientMessage::Close(Some(frame)) = back else {
            panic!("expected a close frame");
        };
        assert_eq!(frame.code, 1000);
        assert_eq!(frame.reason.as_str(), "done");
    }
}
