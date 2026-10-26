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
use tokio_tungstenite::tungstenite::{
    protocol::CloseFrame as ServerCloseFrame, Message as ServerMessage, Utf8Bytes,
};

/// Runs until either leg closes, then closes the other side.
pub async fn bridge(socket: WebSocket, upstream: Arc<Upstream>, path_and_query: String) {
    let server = match upstream.connect_ws(&path_and_query).await {
        Ok(stream) => stream,
        Err(err) => {
            eprintln!("Bridge proxy: {err}");
            return;
        }
    };
    println!(
        "Bridge proxy: websocket connected to {}",
        upstream.target().ws_url(&path_and_query)
    );

    let (mut client_sink, mut client_stream) = socket.split();
    let (mut server_sink, mut server_stream) = server.split();

    loop {
        tokio::select! {
            message = client_stream.next() => {
                let Some(message) = message else { break };
                let Ok(message) = message else { break };
                let Some(message) = to_server(message) else { continue };
                log_engineio("browser->server", &message);
                if server_sink.send(message).await.is_err() {
                    break;
                }
            }
            message = server_stream.next() => {
                let Some(message) = message else { break };
                let Ok(message) = message else { break };
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
