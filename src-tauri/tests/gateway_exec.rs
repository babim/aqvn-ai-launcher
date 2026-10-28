//! Integration tests for remote PTY code execution via connector_exec_op.
//!
//! Fulfills VAL-GATEWAY-003:
//! - Handles connector_exec_op with runtime=terminal
//! - Executes shell commands inside stateful PTY session
//! - Streams output back to agent via connector_exec_op_result

use aqvn_workspace_lib::connector::client::{GatewayClient, GatewayConfig};
use aqvn_workspace_lib::connector::handlers::exec;
use aqvn_workspace_lib::connector::protocol::{
    MSG_TYPE_EXEC_OP, MSG_TYPE_EXEC_OP_RESULT, MSG_TYPE_HELLO, MSG_TYPE_HELLO_ACK, RUNTIME_TERMINAL,
};
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::routing::get;
use axum::Router;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};

#[tokio::test]
async fn test_gateway_pty_execution() {
    let session_id = "test-session-stateful-1";
    exec::reset_session(session_id);

    // 1. Basic command execution with runtime=terminal
    let op1 = json!({
        "type": MSG_TYPE_EXEC_OP,
        "op": MSG_TYPE_EXEC_OP,
        "id": "req-exec-1",
        "runtime": RUNTIME_TERMINAL,
        "command": "echo 'Hello from PTY'",
        "session_id": session_id,
    });

    let reply1_str = exec::handle_exec_op(&op1).await;
    let reply1: Value = serde_json::from_str(&reply1_str).expect("parse reply1");

    assert_eq!(reply1["type"], MSG_TYPE_EXEC_OP_RESULT);
    assert_eq!(reply1["id"], "req-exec-1");
    assert_eq!(reply1["session_id"], session_id);
    assert_eq!(reply1["success"], true);
    assert_eq!(reply1["exit_code"], 0);
    assert_eq!(reply1["done"], true);
    assert!(
        reply1["output"]
            .as_str()
            .unwrap()
            .contains("Hello from PTY"),
        "output must contain 'Hello from PTY', got: {:?}",
        reply1["output"]
    );

    // 2. Stateful PTY session: set environment variable in session
    let op2 = json!({
        "type": MSG_TYPE_EXEC_OP,
        "id": "req-exec-2",
        "runtime": "terminal",
        "command": "export AQVN_STATE_KEY=gateway_phase2_success",
        "session_id": session_id,
    });

    let reply2_str = exec::handle_exec_op(&op2).await;
    let reply2: Value = serde_json::from_str(&reply2_str).expect("parse reply2");
    assert_eq!(reply2["success"], true);
    assert_eq!(reply2["exit_code"], 0);

    // 3. Stateful PTY session: verify variable persists in subsequent command
    let op3 = json!({
        "type": MSG_TYPE_EXEC_OP,
        "id": "req-exec-3",
        "runtime": "terminal",
        "command": "echo \"VAL=$AQVN_STATE_KEY\"",
        "session_id": session_id,
    });

    let reply3_str = exec::handle_exec_op(&op3).await;
    let reply3: Value = serde_json::from_str(&reply3_str).expect("parse reply3");
    assert_eq!(reply3["success"], true);
    assert_eq!(reply3["exit_code"], 0);
    assert!(
        reply3["output"]
            .as_str()
            .unwrap()
            .contains("VAL=gateway_phase2_success"),
        "stateful variable must persist across commands, got: {:?}",
        reply3["output"]
    );

    // 4. Command failure / non-zero exit code
    let op4 = json!({
        "type": MSG_TYPE_EXEC_OP,
        "id": "req-exec-4",
        "runtime": "terminal",
        "command": "ls /nonexistent_directory_aqvn_xyz_123",
        "session_id": session_id,
    });

    let reply4_str = exec::handle_exec_op(&op4).await;
    let reply4: Value = serde_json::from_str(&reply4_str).expect("parse reply4");
    assert_eq!(reply4["success"], false);
    assert_ne!(reply4["exit_code"], 0);
    assert_eq!(reply4["done"], true);

    // 5. Unsupported runtime rejection
    let op5 = json!({
        "type": MSG_TYPE_EXEC_OP,
        "id": "req-exec-5",
        "runtime": "unsupported_runtime_ruby",
        "command": "puts 123",
        "session_id": session_id,
    });

    let reply5_str = exec::handle_exec_op(&op5).await;
    let reply5: Value = serde_json::from_str(&reply5_str).expect("parse reply5");
    assert_eq!(reply5["success"], false);
    assert!(
        reply5["error"]
            .as_str()
            .unwrap()
            .contains("unsupported runtime"),
        "must reject unsupported runtime"
    );

    // 6. Reset session
    let op6 = json!({
        "type": MSG_TYPE_EXEC_OP,
        "id": "req-exec-6",
        "action": "reset",
        "session_id": session_id,
    });

    let reply6_str = exec::handle_exec_op(&op6).await;
    let reply6: Value = serde_json::from_str(&reply6_str).expect("parse reply6");
    assert_eq!(reply6["success"], true);
    assert_eq!(reply6["action"], "reset");

    exec::reset_session(session_id);
}

#[tokio::test]
async fn test_gateway_pty_streaming() {
    let session_id = "test-session-stream-1";
    exec::reset_session(session_id);

    let (chunk_tx, mut chunk_rx) = mpsc::channel::<String>(32);

    let op = json!({
        "type": MSG_TYPE_EXEC_OP,
        "id": "req-stream-1",
        "runtime": "terminal",
        "command": "for i in 1 2 3; do echo \"item_$i\"; done",
        "session_id": session_id,
    });

    let handle =
        tokio::spawn(async move { exec::handle_exec_op_streaming(&op, Some(chunk_tx)).await });

    let mut streamed_output = String::new();
    let mut chunk_count = 0;

    while let Some(chunk_json) = chunk_rx.recv().await {
        let chunk: Value = serde_json::from_str(&chunk_json).expect("parse chunk");
        assert_eq!(chunk["type"], MSG_TYPE_EXEC_OP_RESULT);
        assert_eq!(chunk["id"], "req-stream-1");
        assert_eq!(chunk["stream"], "stdout");
        assert_eq!(chunk["done"], false);
        if let Some(text) = chunk["output"].as_str() {
            streamed_output.push_str(text);
        }
        chunk_count += 1;
    }

    let final_reply_str = handle.await.expect("join handle");
    let final_reply: Value = serde_json::from_str(&final_reply_str).expect("parse final reply");

    assert_eq!(final_reply["success"], true);
    assert_eq!(final_reply["exit_code"], 0);
    assert_eq!(final_reply["done"], true);
    assert!(
        final_reply["output"].as_str().unwrap().contains("item_1"),
        "final output must contain item_1, got: {:?}",
        final_reply["output"]
    );
    assert!(
        final_reply["output"].as_str().unwrap().contains("item_2"),
        "final output must contain item_2, got: {:?}",
        final_reply["output"]
    );
    assert!(
        final_reply["output"].as_str().unwrap().contains("item_3"),
        "final output must contain item_3, got: {:?}",
        final_reply["output"]
    );

    assert!(chunk_count >= 1, "must have streamed at least one chunk");
    assert!(
        streamed_output.contains("item_1"),
        "streamed chunks must contain output, got: {streamed_output}"
    );

    exec::reset_session(session_id);
}

#[tokio::test]
async fn test_gateway_pty_websocket_roundtrip() {
    let session_id = "ws-session-1";
    exec::reset_session(session_id);

    let (tx_reply, rx_reply) = mpsc::channel::<String>(32);
    let rx_reply_mutex = Arc::new(Mutex::new(rx_reply));

    let (tx_request, rx_request) = mpsc::channel::<String>(16);
    let rx_request_mutex = Arc::new(Mutex::new(rx_request));

    let router = Router::new().route(
        "/ws",
        get({
            let tx_reply = tx_reply.clone();
            let rx_request_mutex = Arc::clone(&rx_request_mutex);
            move |ws: WebSocketUpgrade| {
                let tx_reply = tx_reply.clone();
                let rx_request_mutex = Arc::clone(&rx_request_mutex);
                async move {
                    ws.on_upgrade(move |mut socket: WebSocket| async move {
                        // 1. Handshake hello
                        if let Some(Ok(axum::extract::ws::Message::Text(text))) =
                            socket.recv().await
                        {
                            if text.contains(MSG_TYPE_HELLO) {
                                let ack = json!({ "type": MSG_TYPE_HELLO_ACK, "status": "ok" });
                                let _ = socket
                                    .send(axum::extract::ws::Message::Text(ack.to_string().into()))
                                    .await;
                            }
                        }

                        // 2. Exchange requests and replies
                        let mut rx_req = rx_request_mutex.lock().await;
                        while let Some(req_text) = rx_req.recv().await {
                            if socket
                                .send(axum::extract::ws::Message::Text(req_text.into()))
                                .await
                                .is_err()
                            {
                                break;
                            }
                            // Read responses until done: true
                            while let Some(Ok(axum::extract::ws::Message::Text(reply_text))) =
                                socket.recv().await
                            {
                                let is_done =
                                    if let Ok(v) = serde_json::from_str::<Value>(&reply_text) {
                                        v.get("done").and_then(Value::as_bool).unwrap_or(true)
                                    } else {
                                        true
                                    };
                                let _ = tx_reply.send(reply_text.to_string()).await;
                                if is_done {
                                    break;
                                }
                            }
                        }
                    })
                }
            }
        }),
    );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server_task = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });

    let config = GatewayConfig {
        enabled: true,
        server_url: format!("http://127.0.0.1:{port}"),
        gateway_id: "test-gw-ws-exec".to_string(),
        host_label: "AQVN-Test-Host".to_string(),
        scope_files: false,
        scope_exec: true,
        scope_browser: false,
        scope_desktop: false,
        ping_interval_secs: 25,
        reconnect_delay_ms: 50,
    };

    let client = GatewayClient::new(config);
    let (handle, join_handle) = client.start();

    handle
        .wait_for_handshake(Duration::from_millis(2000))
        .await
        .expect("handshake completed");

    // Send exec operation over WebSocket
    let exec_op = json!({
        "type": MSG_TYPE_EXEC_OP,
        "id": "ws-exec-1",
        "runtime": "terminal",
        "command": "echo 'WebSocket PTY Success'",
        "session_id": session_id,
    });

    tx_request.send(exec_op.to_string()).await.unwrap();

    let reply_str = tokio::time::timeout(Duration::from_millis(3000), async {
        let mut rx = rx_reply_mutex.lock().await;
        // Collect messages until final done
        let mut last = String::new();
        while let Some(msg) = rx.recv().await {
            last = msg.clone();
            if let Ok(val) = serde_json::from_str::<Value>(&msg) {
                if val.get("done").and_then(Value::as_bool).unwrap_or(false) {
                    break;
                }
            }
        }
        last
    })
    .await
    .expect("receive reply within timeout");

    let reply: Value = serde_json::from_str(&reply_str).expect("parse ws reply");
    assert_eq!(reply["type"], MSG_TYPE_EXEC_OP_RESULT);
    assert_eq!(reply["id"], "ws-exec-1");
    assert_eq!(reply["success"], true);
    assert_eq!(reply["exit_code"], 0);
    assert!(
        reply["output"]
            .as_str()
            .unwrap()
            .contains("WebSocket PTY Success"),
        "must contain expected output"
    );

    handle.stop();
    let _ = join_handle.await;
    server_task.abort();
    exec::reset_session(session_id);
}
