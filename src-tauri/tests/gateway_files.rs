//! Integration tests for remote file operations via connector_file_op.
//!
//! Fulfills VAL-GATEWAY-002:
//! - Handles connector_file_op for read, write, patch, and files_list
//! - Rejects path traversal or symlink escapes outside sandbox
//! - Replies with connector_file_op_result over WebSocket
//! - Transfer Protocol v1 support (checksum, base64, chunking, protocol_version)

use aqvn_workspace_lib::connector::client::{GatewayClient, GatewayConfig};
use aqvn_workspace_lib::connector::handlers::files::{self, set_test_sandbox};
use aqvn_workspace_lib::connector::protocol::{
    MSG_TYPE_FILE_OP, MSG_TYPE_FILE_OP_RESULT, MSG_TYPE_HELLO, MSG_TYPE_HELLO_ACK,
    TRANSFER_PROTOCOL_V1,
};
use aqvn_workspace_lib::rfc::sandbox::{Sandbox, DEFAULT_SANDBOX};
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::routing::get;
use axum::Router;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};

fn create_temp_sandbox() -> (PathBuf, Sandbox) {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "aqvn-gw-files-test-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("create temp sandbox root");
    let sandbox = Sandbox::new(root.clone());
    (root, sandbox)
}

#[tokio::test]
async fn test_gateway_file_operations() {
    let (sandbox_dir, sandbox) = create_temp_sandbox();
    set_test_sandbox(Some(sandbox.clone()));

    // ------------------------------------------------------------- 1. Write file
    let write_op = json!({
        "type": MSG_TYPE_FILE_OP,
        "op": MSG_TYPE_FILE_OP,
        "id": "req-write-1",
        "action": "write",
        "path": "hello.txt",
        "content": "Hello World!\nLine 2 here.\n",
        "encoding": "utf-8"
    });

    let reply_str = files::handle_file_op_with_sandbox(&write_op, &sandbox).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse write reply");

    assert_eq!(reply["type"], MSG_TYPE_FILE_OP_RESULT);
    assert_eq!(reply["id"], "req-write-1");
    assert_eq!(reply["success"], true);
    assert_eq!(reply["status"], "ok");
    assert_eq!(reply["action"], "write");
    assert_eq!(reply["transfer_protocol"], TRANSFER_PROTOCOL_V1);
    assert_eq!(reply["protocol_version"], "v1");
    assert!(reply["bytes_written"].as_u64().unwrap_or(0) > 0);
    assert!(reply["checksum"].is_string());

    // Verify file actually created on disk in sandbox
    let on_disk = sandbox_dir.join("hello.txt");
    assert!(on_disk.is_file(), "file must be created in sandbox");
    let disk_content = std::fs::read_to_string(&on_disk).unwrap();
    assert_eq!(disk_content, "Hello World!\nLine 2 here.\n");

    // ------------------------------------------------------------- 2. Read file
    let read_op = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "req-read-1",
        "action": "read",
        "path": "hello.txt"
    });

    let reply_str = files::handle_file_op_with_sandbox(&read_op, &sandbox).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse read reply");

    assert_eq!(reply["type"], MSG_TYPE_FILE_OP_RESULT);
    assert_eq!(reply["id"], "req-read-1");
    assert_eq!(reply["success"], true);
    assert_eq!(reply["status"], "ok");
    assert_eq!(reply["action"], "read");
    assert_eq!(reply["content"], "Hello World!\nLine 2 here.\n");
    assert_eq!(reply["transfer_protocol"], TRANSFER_PROTOCOL_V1);
    assert_eq!(reply["protocol_version"], "v1");
    assert_eq!(reply["encoding"], "utf-8");
    assert!(reply["checksum"].is_string());
    assert_eq!(reply["total_lines"], 2);

    // ------------------------------------------------------------- 3. Patch file (exact replacement)
    let patch_op = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "req-patch-1",
        "action": "patch",
        "path": "hello.txt",
        "old_text": "Hello World!",
        "new_text": "Greetings AQVN!"
    });

    let reply_str = files::handle_file_op_with_sandbox(&patch_op, &sandbox).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse patch reply");

    assert_eq!(reply["type"], MSG_TYPE_FILE_OP_RESULT);
    assert_eq!(reply["id"], "req-patch-1");
    assert_eq!(reply["success"], true);
    assert_eq!(reply["status"], "ok");
    assert_eq!(reply["action"], "patch");
    assert_eq!(reply["replacement_count"], 1);

    let disk_content = std::fs::read_to_string(&on_disk).unwrap();
    assert_eq!(disk_content, "Greetings AQVN!\nLine 2 here.\n");

    // Patch file with context diff
    let context_patch_op = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "req-patch-2",
        "action": "patch",
        "path": "hello.txt",
        "patch_text": "*** Begin Patch\n*** Update File: hello.txt\n@@ Greetings AQVN!\n-Greetings AQVN!\n+Hello Universe!\n*** End Patch"
    });
    let reply_str = files::handle_file_op_with_sandbox(&context_patch_op, &sandbox).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse context patch reply");
    assert_eq!(reply["success"], true);
    let disk_content = std::fs::read_to_string(&on_disk).unwrap();
    assert_eq!(disk_content, "Hello Universe!\nLine 2 here.\n");

    // ------------------------------------------------------------- 4. List files (files_list)
    let list_op = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "req-list-1",
        "action": "files_list",
        "path": ""
    });

    let reply_str = files::handle_file_op_with_sandbox(&list_op, &sandbox).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse list reply");

    assert_eq!(reply["type"], MSG_TYPE_FILE_OP_RESULT);
    assert_eq!(reply["id"], "req-list-1");
    assert_eq!(reply["success"], true);
    assert_eq!(reply["status"], "ok");
    assert_eq!(reply["action"], "files_list");
    let entries = reply["entries"].as_array().expect("entries array");
    assert!(entries
        .iter()
        .any(|e| e["name"] == "hello.txt" && e["is_file"] == true));

    // ------------------------------------------------------------- 5. Reject path traversal outside sandbox
    let traversal_op = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "req-traversal-1",
        "action": "read",
        "path": "../../etc/passwd"
    });

    let reply_str = files::handle_file_op_with_sandbox(&traversal_op, &sandbox).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse traversal reply");

    assert_eq!(reply["type"], MSG_TYPE_FILE_OP_RESULT);
    assert_eq!(reply["id"], "req-traversal-1");
    assert_eq!(reply["success"], false);
    assert_eq!(reply["status"], "error");
    assert!(reply["error"]
        .as_str()
        .unwrap()
        .contains("outside the allowed sandbox"));

    // ------------------------------------------------------------- 6. Reject absolute path outside sandbox
    let abs_op = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "req-abs-1",
        "action": "write",
        "path": "/etc/evil.txt",
        "content": "malicious"
    });

    let reply_str = files::handle_file_op_with_sandbox(&abs_op, &sandbox).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse abs reply");
    assert_eq!(reply["success"], false);
    assert_eq!(reply["status"], "error");
    assert!(reply["error"]
        .as_str()
        .unwrap()
        .contains("outside the allowed sandbox"));

    // ------------------------------------------------------------- 7. Reject symlink escape outside sandbox
    #[cfg(unix)]
    {
        let symlink_path = sandbox_dir.join("escape_link");
        let _ = std::os::unix::fs::symlink("/etc", &symlink_path);

        let symlink_op = json!({
            "type": MSG_TYPE_FILE_OP,
            "id": "req-symlink-1",
            "action": "read",
            "path": "escape_link/passwd"
        });

        let reply_str = files::handle_file_op_with_sandbox(&symlink_op, &sandbox).await;
        let reply: Value = serde_json::from_str(&reply_str).expect("parse symlink reply");
        assert_eq!(reply["success"], false);
        assert_eq!(reply["status"], "error");
        assert!(reply["error"]
            .as_str()
            .unwrap()
            .contains("outside the allowed sandbox"));
    }

    set_test_sandbox(None);
    let _ = std::fs::remove_dir_all(&sandbox_dir);
}

#[tokio::test]
async fn test_transfer_protocol_v1_binary_base64() {
    let (sandbox_dir, sandbox) = create_temp_sandbox();
    set_test_sandbox(Some(sandbox.clone()));

    // Binary payload with null bytes: [0, 1, 2, 255, 128, 0, 42]
    // Base64 encoded: "AAEC/4AAKg=="
    let write_binary_op = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "req-bin-write",
        "action": "write",
        "path": "binary.dat",
        "content": "AAEC/4AAKg==",
        "encoding": "base64"
    });

    let reply_str = files::handle_file_op_with_sandbox(&write_binary_op, &sandbox).await;
    let reply: Value = serde_json::from_str(&reply_str).unwrap();
    assert_eq!(reply["success"], true);
    assert_eq!(reply["bytes_written"], 7);
    assert_eq!(reply["transfer_protocol"], "v1");

    // Read back: binary auto-detection should return base64
    let read_binary_op = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "req-bin-read",
        "action": "read",
        "path": "binary.dat"
    });

    let reply_str = files::handle_file_op_with_sandbox(&read_binary_op, &sandbox).await;
    let reply: Value = serde_json::from_str(&reply_str).unwrap();
    assert_eq!(reply["success"], true);
    assert_eq!(reply["encoding"], "base64");
    assert_eq!(reply["content"], "AAEC/4AAKg==");
    assert_eq!(reply["size"], 7);
    assert_eq!(reply["total_size"], 7);

    // Chunked read: offset 3, length 2 (bytes [255, 128]) -> base64 "/4A="
    let chunk_op = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "req-chunk-read",
        "action": "read",
        "path": "binary.dat",
        "offset": 3,
        "length": 2,
        "encoding": "base64"
    });
    let reply_str = files::handle_file_op_with_sandbox(&chunk_op, &sandbox).await;
    let reply: Value = serde_json::from_str(&reply_str).unwrap();
    assert_eq!(reply["success"], true);
    assert_eq!(reply["content"], "/4A=");
    assert_eq!(reply["size"], 2);
    assert_eq!(reply["total_size"], 7);

    set_test_sandbox(None);
    let _ = std::fs::remove_dir_all(&sandbox_dir);
}

#[tokio::test]
async fn test_gateway_websocket_roundtrip() {
    let (sandbox_dir, sandbox) = create_temp_sandbox();
    set_test_sandbox(Some(sandbox.clone()));

    // Test channels for bidirectional communication
    let (tx_reply, rx_reply) = mpsc::channel::<String>(16);
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
                        // 1. Receive hello handshake
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

                        // 2. Pump requests from test and receive replies
                        let mut rx_req = rx_request_mutex.lock().await;
                        while let Some(req_text) = rx_req.recv().await {
                            if socket
                                .send(axum::extract::ws::Message::Text(req_text.into()))
                                .await
                                .is_err()
                            {
                                break;
                            }
                            if let Some(Ok(axum::extract::ws::Message::Text(reply_text))) =
                                socket.recv().await
                            {
                                let _ = tx_reply.send(reply_text.to_string()).await;
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
        gateway_id: "test-gw-ws-files".to_string(),
        host_label: "AQVN-Test-Host".to_string(),
        session_cookie: None,
        scope_files: true,
        scope_exec: false,
        scope_browser: false,
        scope_desktop: false,
        ping_interval_secs: 25,
        reconnect_delay_ms: 200,
    };

    let client = GatewayClient::new(config);
    let (handle, join_handle) = client.start();

    handle
        .wait_for_handshake(Duration::from_secs(5))
        .await
        .expect("handshake must complete over ws");

    // Write file over WebSocket
    let write_op = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "ws-req-write",
        "action": "write",
        "path": "ws_file.txt",
        "content": "Hello from WebSocket test!\n"
    });
    tx_request.send(write_op.to_string()).await.unwrap();

    let reply_text = rx_reply_mutex
        .lock()
        .await
        .recv()
        .await
        .expect("receive ws reply");
    let reply: Value = serde_json::from_str(&reply_text).unwrap();
    assert_eq!(reply["type"], MSG_TYPE_FILE_OP_RESULT);
    assert_eq!(reply["id"], "ws-req-write");
    assert_eq!(reply["success"], true);
    assert_eq!(reply["transfer_protocol"], "v1");

    // Read file over WebSocket
    let read_op = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "ws-req-read",
        "action": "read",
        "path": "ws_file.txt"
    });
    tx_request.send(read_op.to_string()).await.unwrap();

    let reply_text = rx_reply_mutex
        .lock()
        .await
        .recv()
        .await
        .expect("receive ws reply");
    let reply: Value = serde_json::from_str(&reply_text).unwrap();
    assert_eq!(reply["type"], MSG_TYPE_FILE_OP_RESULT);
    assert_eq!(reply["id"], "ws-req-read");
    assert_eq!(reply["success"], true);
    assert_eq!(reply["content"], "Hello from WebSocket test!\n");
    assert_eq!(reply["transfer_protocol"], "v1");

    handle.stop();
    let _ = join_handle.await;
    server_task.abort();

    set_test_sandbox(None);
    let _ = std::fs::remove_dir_all(&sandbox_dir);
}

#[tokio::test]
async fn test_default_sandbox_workspace_confinement() {
    let sandbox = Sandbox::from_allowed_paths(&[DEFAULT_SANDBOX.to_string()]);

    assert!(
        sandbox.roots().iter().any(|r| r.ends_with("Workspace")),
        "default sandbox root must resolve to ~/Workspace, got: {:?}",
        sandbox.roots()
    );

    // 1. Strict rejection of traversal outside ~/Workspace
    let traversal_op = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "req-default-trav",
        "action": "read",
        "path": "../../etc/shadow"
    });
    let reply_str = files::handle_file_op_with_sandbox(&traversal_op, &sandbox).await;
    let reply: Value = serde_json::from_str(&reply_str).unwrap();
    assert_eq!(reply["success"], false);
    assert_eq!(reply["status"], "error");
    assert!(
        reply["error"]
            .as_str()
            .unwrap()
            .contains("outside the allowed sandbox"),
        "path traversal must be rejected by default sandbox"
    );

    // 2. Strict rejection of absolute paths outside ~/Workspace
    let abs_op = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "req-default-abs",
        "action": "write",
        "path": "/etc/malicious_workspace_test.txt",
        "content": "deny"
    });
    let reply_str = files::handle_file_op_with_sandbox(&abs_op, &sandbox).await;
    let reply: Value = serde_json::from_str(&reply_str).unwrap();
    assert_eq!(reply["success"], false);
    assert_eq!(reply["status"], "error");
    assert!(
        reply["error"]
            .as_str()
            .unwrap()
            .contains("outside the allowed sandbox"),
        "absolute paths outside sandbox must be rejected"
    );

    // 3. Normal file write and read inside ~/Workspace is jailed and cleanable
    let test_filename = format!(".aqvn_test_sandbox_{}.txt", std::process::id());
    let write_op = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "req-default-write",
        "action": "write",
        "path": test_filename,
        "content": "sandboxed inside Workspace\n"
    });
    let reply_str = files::handle_file_op_with_sandbox(&write_op, &sandbox).await;
    let reply: Value = serde_json::from_str(&reply_str).unwrap();
    assert_eq!(reply["success"], true);

    let read_op = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "req-default-read",
        "action": "read",
        "path": test_filename
    });
    let reply_str = files::handle_file_op_with_sandbox(&read_op, &sandbox).await;
    let reply: Value = serde_json::from_str(&reply_str).unwrap();
    assert_eq!(reply["success"], true);
    assert_eq!(reply["content"], "sandboxed inside Workspace\n");

    // Clean up test file
    let remove_op = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "req-default-remove",
        "action": "files_remove",
        "path": test_filename
    });
    let _ = files::handle_file_op_with_sandbox(&remove_op, &sandbox).await;
}
