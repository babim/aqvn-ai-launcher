//! Handlers for multiplexed gateway operations.
//!
//! Subordinate modules implement `connector_file_op`, `connector_exec_op`,
//! `connector_browser_op`, and `connector_computer_use_op`.

use serde_json::Value;

/// Dispatches an incoming JSON message to the appropriate handler.
/// Returns an optional reply JSON string to send back upstream.
pub async fn dispatch_message(text: &str) -> Option<String> {
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
        _ => None,
    }
}
