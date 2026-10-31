//! Handlers for multiplexed gateway operations.
//!
//! Subordinate modules implement `connector_file_op`, `connector_exec_op`,
//! `connector_browser_op`, and `connector_computer_use_op`.

pub mod browser;
pub mod computer_use;
pub mod exec;
pub mod files;

use crate::connector::client::GatewayConfig;
use crate::connector::protocol::{
    MSG_TYPE_BROWSER_OP, MSG_TYPE_BROWSER_OP_RESULT, MSG_TYPE_COMPUTER_USE_OP,
    MSG_TYPE_COMPUTER_USE_OP_RESULT, MSG_TYPE_EXEC_OP, MSG_TYPE_EXEC_OP_RESULT, MSG_TYPE_FILE_OP,
    MSG_TYPE_FILE_OP_RESULT, MSG_TYPE_GATEWAY_CONTROL, MSG_TYPE_GATEWAY_CONTROL_RESULT,
};
use serde_json::Value;

/// Dispatches an incoming JSON message with scope enforcement and an optional streaming sender.
/// Returns an optional final reply JSON string to send back upstream.
pub async fn dispatch_message_scoped(
    text: &str,
    sender: Option<&tokio::sync::mpsc::Sender<tokio_tungstenite::tungstenite::Message>>,
    config: &GatewayConfig,
) -> Option<String> {
    let Ok(envelope) = serde_json::from_str::<Value>(text) else {
        return None;
    };

    let op = envelope
        .get("op")
        .or_else(|| envelope.get("type"))
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    let id = envelope
        .get("id")
        .or_else(|| envelope.get("op_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

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
        MSG_TYPE_GATEWAY_CONTROL | "gateway_control" => {
            let op_id = envelope
                .get("op_id")
                .or_else(|| envelope.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or(&id)
                .to_string();

            let mut scopes = serde_json::Map::new();
            scopes.insert("files".to_string(), Value::Bool(config.scope_files));
            scopes.insert("file_write".to_string(), Value::Bool(config.scope_files));
            scopes.insert("code_execution".to_string(), Value::Bool(config.scope_exec));
            scopes.insert("browser".to_string(), Value::Bool(config.scope_browser));
            scopes.insert(
                "computer_use".to_string(),
                Value::Bool(config.scope_desktop),
            );

            let state = serde_json::json!({
                "gateway_id": config.gateway_id,
                "host_label": config.host_label,
                "connected": true,
                "master_enabled": config.enabled,
                "scopes": Value::Object(scopes),
            });

            Some(
                serde_json::json!({
                    "type": MSG_TYPE_GATEWAY_CONTROL_RESULT,
                    "op": MSG_TYPE_GATEWAY_CONTROL_RESULT,
                    "id": id,
                    "op_id": op_id,
                    "ok": true,
                    "success": true,
                    "status": "ok",
                    "state": state,
                })
                .to_string(),
            )
        }
        MSG_TYPE_FILE_OP | "file_op" => {
            if !config.scope_files {
                return Some(
                    serde_json::json!({
                        "type": MSG_TYPE_FILE_OP_RESULT,
                        "op": MSG_TYPE_FILE_OP_RESULT,
                        "id": id,
                        "success": false,
                        "status": "error",
                        "action": envelope.get("action").and_then(|v| v.as_str()).unwrap_or(""),
                        "transfer_protocol": "v1",
                        "protocol_version": "v1",
                        "error": "files scope disabled by configuration"
                    })
                    .to_string(),
                );
            }
            Some(files::handle_file_op(&envelope).await)
        }
        MSG_TYPE_EXEC_OP | "exec_op" => {
            if !config.scope_exec {
                return Some(
                    serde_json::json!({
                        "type": MSG_TYPE_EXEC_OP_RESULT,
                        "op": MSG_TYPE_EXEC_OP_RESULT,
                        "id": id,
                        "session_id": envelope.get("session_id").and_then(|v| v.as_str()).unwrap_or(""),
                        "success": false,
                        "status": "error",
                        "done": true,
                        "error": "code_execution scope disabled by configuration"
                    })
                    .to_string(),
                );
            }
            Some(exec::handle_exec_op_with_sender(&envelope, sender).await)
        }
        MSG_TYPE_BROWSER_OP | "browser_op" => {
            if !config.scope_browser {
                return Some(
                    serde_json::json!({
                        "type": MSG_TYPE_BROWSER_OP_RESULT,
                        "op": MSG_TYPE_BROWSER_OP_RESULT,
                        "id": id,
                        "success": false,
                        "status": "error",
                        "action": envelope.get("action").and_then(|v| v.as_str()).unwrap_or(""),
                        "error": "browser scope disabled by configuration"
                    })
                    .to_string(),
                );
            }
            Some(browser::handle_browser_op(&envelope).await)
        }
        MSG_TYPE_COMPUTER_USE_OP | "computer_use_op" | "computer_use" => {
            if !config.scope_desktop {
                return Some(
                    serde_json::json!({
                        "type": MSG_TYPE_COMPUTER_USE_OP_RESULT,
                        "op": MSG_TYPE_COMPUTER_USE_OP_RESULT,
                        "id": id,
                        "success": false,
                        "status": "error",
                        "action": envelope.get("action").and_then(|v| v.as_str()).unwrap_or(""),
                        "error": "computer_use scope disabled by configuration"
                    })
                    .to_string(),
                );
            }
            Some(computer_use::handle_computer_use_op(&envelope).await)
        }
        _ => None,
    }
}

/// Dispatches an incoming JSON message with an optional streaming sender.
/// Returns an optional final reply JSON string to send back upstream.
pub async fn dispatch_message_streaming(
    text: &str,
    sender: Option<&tokio::sync::mpsc::Sender<tokio_tungstenite::tungstenite::Message>>,
) -> Option<String> {
    dispatch_message_scoped(text, sender, &GatewayConfig::default()).await
}

/// Dispatches an incoming JSON message to the appropriate handler.
/// Returns an optional reply JSON string to send back upstream.
pub async fn dispatch_message(text: &str) -> Option<String> {
    dispatch_message_streaming(text, None).await
}
