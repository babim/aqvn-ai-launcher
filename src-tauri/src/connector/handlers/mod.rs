//! Handlers for multiplexed gateway operations.
//!
//! Subordinate modules implement `connector_file_op`, `connector_exec_op`,
//! `connector_browser_op`, and `connector_computer_use_op`.

pub mod browser;
pub mod exec;
pub mod files;

use crate::connector::protocol::{MSG_TYPE_BROWSER_OP, MSG_TYPE_EXEC_OP, MSG_TYPE_FILE_OP};
use serde_json::Value;

/// Dispatches an incoming JSON message with an optional streaming sender.
/// Returns an optional final reply JSON string to send back upstream.
pub async fn dispatch_message_streaming(
    text: &str,
    sender: Option<&tokio::sync::mpsc::Sender<tokio_tungstenite::tungstenite::Message>>,
) -> Option<String> {
    let Ok(envelope) = serde_json::from_str::<Value>(text) else {
        return None;
    };

    let op = envelope
        .get("op")
        .or_else(|| envelope.get("type"))
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    match op {
        "connector_hello_ack" => None,
        "ping" => Some(
            serde_json::json!({
                "type": "pong",
                "timestamp": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0),
            })
            .to_string(),
        ),
        MSG_TYPE_FILE_OP | "file_op" => Some(files::handle_file_op(&envelope).await),
        MSG_TYPE_EXEC_OP | "exec_op" => {
            Some(exec::handle_exec_op_with_sender(&envelope, sender).await)
        }
        MSG_TYPE_BROWSER_OP | "browser_op" => Some(browser::handle_browser_op(&envelope).await),
        _ => None,
    }
}

/// Dispatches an incoming JSON message to the appropriate handler.
/// Returns an optional reply JSON string to send back upstream.
pub async fn dispatch_message(text: &str) -> Option<String> {
    dispatch_message_streaming(text, None).await
}
