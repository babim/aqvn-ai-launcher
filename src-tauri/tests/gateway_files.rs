//! Integration tests for remote file operations via connector_file_op.
//!
//! Fulfills VAL-GATEWAY-002:
//! - Handles connector_file_op for read, write, patch, and files_list
//! - Rejects path traversal or symlink escapes outside sandbox
//! - Replies with connector_file_op_result over WebSocket
//! - Transfer Protocol v1 support (checksum, base64, chunking, protocol_version)

use aqvn_workspace_lib::config::AppConfig;
use aqvn_workspace_lib::connector::client::{GatewayClient, GatewayConfig};
use aqvn_workspace_lib::connector::handlers::files::{self, set_test_sandbox};
use aqvn_workspace_lib::connector::protocol::{
    MSG_TYPE_FILE_OP, MSG_TYPE_FILE_OP_RESULT, MSG_TYPE_HELLO, MSG_TYPE_HELLO_ACK,
    MSG_TYPE_REMOTE_TREE_UPDATE, TRANSFER_PROTOCOL_V1,
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

static TEST_SANDBOX_LOCK: Mutex<()> = Mutex::const_new(());

#[tokio::test]
async fn test_gateway_file_operations() {
    let _lock = TEST_SANDBOX_LOCK.lock().await;
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
    let _lock = TEST_SANDBOX_LOCK.lock().await;
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
    let _lock = TEST_SANDBOX_LOCK.lock().await;
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
                            while let Some(Ok(axum::extract::ws::Message::Text(reply_text))) =
                                socket.recv().await
                            {
                                if !reply_text.contains(MSG_TYPE_REMOTE_TREE_UPDATE) {
                                    let _ = tx_reply.send(reply_text.to_string()).await;
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
        gateway_id: "test-gw-ws-files".to_string(),
        host_label: "AQVN-Test-Host".to_string(),
        session_cookie: None,
        scope_files: true,
        scope_exec: false,
        scope_browser: false,
        scope_desktop: false,
        ping_interval_secs: 25,
        reconnect_delay_ms: 200,
        ..Default::default()
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
    let _lock = TEST_SANDBOX_LOCK.lock().await;
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

#[tokio::test]
async fn test_configurable_workspace_path() {
    let _lock = TEST_SANDBOX_LOCK.lock().await;
    // 1. Verify AppConfig default workspace_path is ~/Workspace
    let default_config = AppConfig::default();
    assert_eq!(default_config.workspace_path, "~/Workspace");

    // 2. Verify AppConfig serialization and deserialization
    let custom_config_json = json!({
        "workspace_path": "/tmp/custom-workspace-test"
    });
    let parsed: AppConfig = serde_json::from_value(custom_config_json).expect("parse AppConfig");
    assert_eq!(parsed.workspace_path, "/tmp/custom-workspace-test");

    // 3. Verify dynamic binding to current_sandbox()
    set_test_sandbox(None);

    let (temp_dir, _) = create_temp_sandbox();
    let temp_str = temp_dir.to_string_lossy().to_string();

    files::set_workspace_path(Some(&temp_str));
    assert_eq!(files::get_workspace_path(), temp_str);

    let active_sandbox = files::current_sandbox();
    let roots = active_sandbox.canonical_roots().expect("canonical roots");
    let temp_canonical = std::fs::canonicalize(&temp_dir).expect("canonicalize temp");
    assert!(
        roots.iter().any(|r| r == &temp_canonical),
        "current_sandbox root must match configured workspace_path"
    );

    // Verify file operations work within this dynamic sandbox root
    let test_file = "configured_ws_test.txt";
    let write_op = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "req-cfg-write",
        "action": "write",
        "path": test_file,
        "content": "dynamic workspace test content\n"
    });
    let reply_str = files::handle_file_op(&write_op).await;
    let reply: Value = serde_json::from_str(&reply_str).unwrap();
    assert_eq!(reply["success"], true);

    let on_disk = temp_dir.join(test_file);
    assert!(
        on_disk.is_file(),
        "file must be created in dynamically configured workspace"
    );

    // Path traversal outside the configured workspace must be rejected
    let traversal_op = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "req-cfg-traversal",
        "action": "read",
        "path": "../../etc/shadow"
    });
    let reply_str = files::handle_file_op(&traversal_op).await;
    let reply: Value = serde_json::from_str(&reply_str).unwrap();
    assert_eq!(reply["success"], false);
    assert_eq!(reply["status"], "error");

    // 4. Reset to default and clean up
    files::set_workspace_path(None);
    assert_eq!(files::get_workspace_path(), "~/Workspace");
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_remote_tree_snapshot_generation() {
    let _lock = TEST_SANDBOX_LOCK.lock().await;
    let (temp_dir, sandbox) = create_temp_sandbox();

    // Setup multi-level folder structure
    // depth 1: file_d1.txt, dir_d1
    // depth 2: dir_d1/file_d2.txt, dir_d1/dir_d2
    // depth 3: dir_d1/dir_d2/file_d3.txt, dir_d1/dir_d2/dir_d3
    // depth 4: dir_d1/dir_d2/dir_d3/file_d4.txt, dir_d1/dir_d2/dir_d3/dir_d4
    // depth 5: dir_d1/dir_d2/dir_d3/dir_d4/file_d5.txt
    let d1 = temp_dir.join("dir_d1");
    let d2 = d1.join("dir_d2");
    let d3 = d2.join("dir_d3");
    let d4 = d3.join("dir_d4");
    std::fs::create_dir_all(&d4).expect("create nested dirs");

    std::fs::write(temp_dir.join("file_d1.txt"), b"d1").unwrap();
    std::fs::write(d1.join("file_d2.txt"), b"d2").unwrap();
    std::fs::write(d2.join("file_d3.txt"), b"d3").unwrap();
    std::fs::write(d3.join("file_d4.txt"), b"d4").unwrap();
    std::fs::write(d4.join("file_d5.txt"), b"d5").unwrap();

    // 1. Generate snapshot bounded to max_depth = 3
    let snapshot = files::generate_remote_tree_update(&sandbox, 3);
    assert_eq!(snapshot["type"], MSG_TYPE_REMOTE_TREE_UPDATE);
    assert_eq!(snapshot["op"], MSG_TYPE_REMOTE_TREE_UPDATE);
    assert_eq!(snapshot["max_depth"], 3);

    let entries = snapshot["entries"].as_array().expect("entries array");

    // Depths 1, 2, 3 must be present
    assert!(entries
        .iter()
        .any(|e| e["name"] == "file_d1.txt" && e["depth"] == 1));
    assert!(entries
        .iter()
        .any(|e| e["name"] == "file_d2.txt" && e["depth"] == 2));
    assert!(entries
        .iter()
        .any(|e| e["name"] == "file_d3.txt" && e["depth"] == 3));

    // Depths > 3 must NOT be present
    assert!(!entries.iter().any(|e| e["name"] == "file_d4.txt"));
    assert!(!entries.iter().any(|e| e["name"] == "file_d5.txt"));

    let snapshot_str = serde_json::to_string(&snapshot).unwrap();
    assert!(!snapshot_str.contains("file_d4.txt"));
    assert!(!snapshot_str.contains("file_d5.txt"));

    // 2. Test emission via GatewayClient over WebSocket
    let (tx_msg, mut rx_msg) = mpsc::channel::<String>(16);
    let router = Router::new().route(
        "/ws",
        get(move |ws: WebSocketUpgrade| {
            let tx_msg = tx_msg.clone();
            async move {
                ws.on_upgrade(move |mut socket: WebSocket| async move {
                    while let Some(Ok(axum::extract::ws::Message::Text(text))) = socket.recv().await
                    {
                        if text.contains(MSG_TYPE_HELLO) {
                            let ack = json!({ "type": MSG_TYPE_HELLO_ACK, "status": "ok" });
                            let _ = socket
                                .send(axum::extract::ws::Message::Text(ack.to_string().into()))
                                .await;
                        } else if text.contains(MSG_TYPE_REMOTE_TREE_UPDATE) {
                            let _ = tx_msg.send(text.to_string()).await;
                            break;
                        }
                    }
                })
            }
        }),
    );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server_task = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });

    files::set_workspace_path(Some(&temp_dir.to_string_lossy()));

    let client_config = GatewayConfig {
        enabled: true,
        server_url: format!("http://127.0.0.1:{port}"),
        gateway_id: "test-gw-tree-emit".to_string(),
        host_label: "AQVN-Test-Host".to_string(),
        session_cookie: None,
        scope_files: true,
        scope_exec: false,
        scope_browser: false,
        scope_desktop: false,
        ping_interval_secs: 25,
        reconnect_delay_ms: 200,
        workspace_path: temp_dir.to_string_lossy().to_string(),
    };

    let client = GatewayClient::new(client_config);
    let (handle, join_handle) = client.start();

    handle
        .wait_for_handshake(Duration::from_secs(5))
        .await
        .expect("handshake completed");

    let received_tree_text = tokio::time::timeout(Duration::from_secs(5), rx_msg.recv())
        .await
        .expect("tree update received within timeout")
        .expect("channel message");

    assert!(
        received_tree_text.contains(MSG_TYPE_REMOTE_TREE_UPDATE),
        "received message must be connector_remote_tree_update"
    );
    assert!(
        received_tree_text.contains("file_d1.txt"),
        "received snapshot must contain file_d1.txt"
    );
    assert!(
        !received_tree_text.contains("file_d4.txt"),
        "received snapshot must NOT contain file_d4.txt"
    );

    handle.stop();
    let _ = join_handle.await;
    server_task.abort();
    files::set_workspace_path(None);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_streamed_file_reading_with_seek() {
    let _lock = TEST_SANDBOX_LOCK.lock().await;
    let (sandbox_dir, sandbox) = create_temp_sandbox();
    set_test_sandbox(Some(sandbox.clone()));

    // Create a 256KB structured file directly on disk inside sandbox
    let file_size: usize = 256 * 1024;
    let mut file_data = vec![b'.'; file_size];

    // Seed distinctive markers at specific byte boundaries
    let header = b"START_HEADER_SEEK_000\n";
    file_data[0..header.len()].copy_from_slice(header);

    let middle_offset: usize = 65536; // 64KB offset
    let middle_slice = b"MIDDLE_SEEK_WINDOW_64K";
    file_data[middle_offset..middle_offset + middle_slice.len()].copy_from_slice(middle_slice);

    let second_offset: usize = 131072; // 128KB offset
    let second_slice = b"SECOND_SEEK_WINDOW_128K\n";
    file_data[second_offset..second_offset + second_slice.len()].copy_from_slice(second_slice);

    let bin_offset: usize = 196608; // 192KB offset
    let bin_slice: [u8; 6] = [0x00, 0xDE, 0xAD, 0xBE, 0xEF, 0x00];
    file_data[bin_offset..bin_offset + bin_slice.len()].copy_from_slice(&bin_slice);

    let tail_offset: usize = file_size - 32;
    let tail_slice = b"END_OF_FILE_SEEK_BUFFER_32_BYTES";
    file_data[tail_offset..tail_offset + tail_slice.len()].copy_from_slice(tail_slice);

    let target_filename = "streamed_large_file.dat";
    std::fs::write(sandbox_dir.join(target_filename), &file_data).expect("write large test file");

    // 1. Stream middle slice with Seek offset 64KB
    let read_mid = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "req-seek-mid",
        "action": "read",
        "path": target_filename,
        "offset": middle_offset,
        "length": middle_slice.len(),
        "encoding": "utf-8"
    });
    let reply_str = files::handle_file_op_with_sandbox(&read_mid, &sandbox).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse reply");

    assert_eq!(reply["success"], true);
    assert_eq!(reply["status"], "ok");
    assert_eq!(reply["content"], "MIDDLE_SEEK_WINDOW_64K");
    assert_eq!(reply["offset"], middle_offset);
    assert_eq!(reply["size"], middle_slice.len());
    assert_eq!(reply["total_size"], file_size);
    assert_eq!(reply["transfer_protocol"], TRANSFER_PROTOCOL_V1);
    assert_eq!(reply["protocol_version"], "v1");

    // Chunk checksum must match the streamed window slice (not entire file)
    let expected_chunk_checksum = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(middle_slice);
        format!("{:x}", hasher.finalize())
    };
    assert_eq!(reply["checksum"], expected_chunk_checksum);

    // 2. Stream second slice with Seek offset 128KB
    let read_second = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "req-seek-second",
        "action": "read",
        "path": target_filename,
        "offset": second_offset,
        "length": second_slice.len(),
        "encoding": "utf-8"
    });
    let reply_str = files::handle_file_op_with_sandbox(&read_second, &sandbox).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse reply");

    assert_eq!(reply["success"], true);
    assert_eq!(reply["content"], "SECOND_SEEK_WINDOW_128K\n");
    assert_eq!(reply["size"], second_slice.len());
    assert_eq!(reply["total_size"], file_size);
    assert_eq!(reply["offset"], second_offset);

    // 3. Binary slice with base64 encoding using seek
    let read_bin = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "req-seek-bin",
        "action": "read",
        "path": target_filename,
        "offset": bin_offset,
        "length": bin_slice.len(),
        "encoding": "base64"
    });
    let reply_str = files::handle_file_op_with_sandbox(&read_bin, &sandbox).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse reply");

    assert_eq!(reply["success"], true);
    assert_eq!(reply["encoding"], "base64");
    assert_eq!(reply["size"], bin_slice.len());
    assert_eq!(reply["total_size"], file_size);
    assert_eq!(reply["content"], "AN6tvu8A");

    // 4. Seek to start (offset 0, length of header)
    let read_start = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "req-seek-start",
        "action": "read",
        "path": target_filename,
        "offset": 0,
        "length": header.len(),
        "encoding": "utf-8"
    });
    let reply_str = files::handle_file_op_with_sandbox(&read_start, &sandbox).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse reply");

    assert_eq!(reply["success"], true);
    assert_eq!(reply["content"], "START_HEADER_SEEK_000\n");
    assert_eq!(reply["size"], header.len());
    assert_eq!(reply["offset"], 0);

    // 5. Seek to tail
    let read_tail = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "req-seek-tail",
        "action": "read",
        "path": target_filename,
        "offset": tail_offset,
        "length": tail_slice.len(),
        "encoding": "utf-8"
    });
    let reply_str = files::handle_file_op_with_sandbox(&read_tail, &sandbox).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse reply");

    assert_eq!(reply["success"], true);
    assert_eq!(reply["content"], "END_OF_FILE_SEEK_BUFFER_32_BYTES");
    assert_eq!(reply["size"], tail_slice.len());

    // 6. Seek beyond EOF returns empty bounded buffer
    let read_eof = json!({
        "type": MSG_TYPE_FILE_OP,
        "id": "req-seek-eof",
        "action": "read",
        "path": target_filename,
        "offset": file_size + 1024,
        "length": 64
    });
    let reply_str = files::handle_file_op_with_sandbox(&read_eof, &sandbox).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse reply");

    assert_eq!(reply["success"], true);
    assert_eq!(reply["size"], 0);
    assert_eq!(reply["content"], "");

    set_test_sandbox(None);
    let _ = std::fs::remove_dir_all(&sandbox_dir);
}
