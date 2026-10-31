//!
//! Fulfills VAL-GATEWAY-004:
//! - Handles connector_browser_op actions (open, navigate, click, type, screenshot, evaluate)
//! - Connects to local Chrome CDP at http://127.0.0.1:9222
//! - Handles open_remote_debugging by launching browser with remote debugging

use aqvn_workspace_lib::connector::client::{GatewayClient, GatewayConfig};
use aqvn_workspace_lib::connector::handlers::browser::{
    self, set_test_browser_binary, set_test_cdp_endpoint, DEFAULT_CDP_PORT, DEFAULT_CDP_URL,
};
use aqvn_workspace_lib::connector::protocol::{
    MSG_TYPE_BROWSER_OP, MSG_TYPE_BROWSER_OP_RESULT, MSG_TYPE_HELLO, MSG_TYPE_HELLO_ACK,
};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::Path as AxumPath;
use axum::routing::{get, put};
use axum::Router;
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};

/// Runs a mock CDP server supporting `/json/version`, `/json/list`, `/json/new`, and `/devtools/page/:id`.
async fn run_mock_cdp_server() -> (u16, tokio::task::JoinHandle<()>) {
    // Attempt port 9222 first if available, otherwise ephemeral port
    let (listener, port) = match TcpListener::bind((Ipv4Addr::LOCALHOST, 9222)).await {
        Ok(l) => (l, 9222),
        Err(_) => {
            let l = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
                .await
                .expect("bind ephemeral port for mock CDP");
            let p = l.local_addr().unwrap().port();
            (l, p)
        }
    };

    let p_copy = port;
    let ip = Ipv4Addr::LOCALHOST;
    let app = Router::new()
        .route(
            "/json/version",
            get(move || async move {
                axum::Json(json!({
                    "Browser": "Chrome/120.0.0.0",
                    "Protocol-Version": "1.3",
                    "User-Agent": "Mozilla/5.0 AQVN-Workspace",
                    "V8-Version": "12.0.0.0",
                    "webSocketDebuggerUrl": format!("ws://{ip}:{p_copy}/devtools/browser")
                }))
            }),
        )
        .route(
            "/json/list",
            get(move || async move {
                axum::Json(json!([
                    {
                        "id": "target-page-1",
                        "type": "page",
                        "title": "AQVN Test Page",
                        "url": format!("http://{ip}:{p_copy}/test"),
                        "webSocketDebuggerUrl": format!("ws://{ip}:{p_copy}/devtools/page/target-page-1")
                    }
                ]))
            }),
        )
        .route(
            "/json",
            get(move || async move {
                axum::Json(json!([
                    {
                        "id": "target-page-1",
                        "type": "page",
                        "title": "AQVN Test Page",
                        "url": format!("http://{ip}:{p_copy}/test"),
                        "webSocketDebuggerUrl": format!("ws://{ip}:{p_copy}/devtools/page/target-page-1")
                    }
                ]))
            }),
        )
        .route(
            "/json/new",
            put(move || async move {
                axum::Json(json!({
                    "id": "target-page-new",
                    "type": "page",
                    "title": "New Tab",
                    "url": "http://example.com/opened",
                    "webSocketDebuggerUrl": format!("ws://{ip}:{p_copy}/devtools/page/target-page-new")
                }))
            }),
        )
        .route(
            "/devtools/page/{target_id}",
            get(handle_cdp_ws_upgrade),
        );

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (port, handle)
}

async fn handle_cdp_ws_upgrade(
    AxumPath(_target_id): AxumPath<String>,
    ws: WebSocketUpgrade,
) -> axum::response::Response {
    ws.on_upgrade(handle_cdp_socket)
}

async fn handle_cdp_socket(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.next().await {
        if let Message::Text(text) = msg {
            if let Ok(req) = serde_json::from_str::<Value>(text.as_str()) {
                let req_id = req.get("id").and_then(|v| v.as_u64()).unwrap_or(1);
                let method = req
                    .get("method")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                let resp_payload = match method {
                    "Page.navigate" => json!({
                        "id": req_id,
                        "result": {
                            "frameId": "mock-frame-001",
                            "loaderId": "mock-loader-001"
                        }
                    }),

                    "Runtime.evaluate" => {
                        let expr = req
                            .get("params")
                            .and_then(|p| p.get("expression"))
                            .and_then(|e| e.as_str())
                            .unwrap_or_default();

                        let result_value = if expr.contains("2 + 2") {
                            json!({ "type": "number", "value": 4 })
                        } else if expr.contains("throw") {
                            json!({
                                "id": req_id,
                                "result": Value::Null,
                                "exceptionDetails": {
                                    "text": "Simulated JS Exception"
                                }
                            })
                        } else if expr.contains("querySelector") {
                            json!({ "type": "boolean", "value": true })
                        } else {
                            json!({ "type": "string", "value": "evaluated_ok" })
                        };

                        if expr.contains("throw") {
                            result_value
                        } else {
                            json!({
                                "id": req_id,
                                "result": {
                                    "result": result_value
                                }
                            })
                        }
                    }

                    "Page.captureScreenshot" => json!({
                        "id": req_id,
                        "result": {
                            "data": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII="
                        }
                    }),

                    "Input.dispatchMouseEvent" => json!({
                        "id": req_id,
                        "result": {}
                    }),

                    "Input.insertText" => json!({
                        "id": req_id,
                        "result": {}
                    }),

                    _ => json!({
                        "id": req_id,
                        "result": {}
                    }),
                };

                let _ = socket
                    .send(Message::Text(resp_payload.to_string().into()))
                    .await;
            }
        }
    }
}

/// Creates an executable mock browser script in a temporary directory.
fn create_mock_browser_binary() -> std::path::PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp_dir = std::env::temp_dir();
    let bin_path = temp_dir.join(format!("mock-browser-{unique}.sh"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(&bin_path, "#!/bin/sh\nexit 0\n").expect("write mock browser script");
        let mut perms = std::fs::metadata(&bin_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&bin_path, perms).expect("set executable perms");
    }

    #[cfg(not(unix))]
    {
        std::fs::write(&bin_path, "@echo off\nexit 0\n").expect("write mock browser script");
    }

    bin_path
}

static TEST_LOCK: Mutex<()> = Mutex::const_new(());

#[tokio::test]
async fn test_gateway_cdp_browser_control() {
    let _guard = TEST_LOCK.lock().await;
    let (cdp_port, _server_handle) = run_mock_cdp_server().await;
    let cdp_url = format!("http://{}:{cdp_port}", Ipv4Addr::LOCALHOST);
    set_test_cdp_endpoint(Some(cdp_url.clone()));

    // Verify default constants
    assert_eq!(DEFAULT_CDP_PORT, 9222);
    assert_eq!(
        DEFAULT_CDP_URL,
        format!("http://{}:9222", Ipv4Addr::LOCALHOST)
    );

    // ------------------------------------------------------------- 1. Open action
    let open_op = json!({
        "type": MSG_TYPE_BROWSER_OP,
        "op": MSG_TYPE_BROWSER_OP,
        "id": "req-browser-open-1",
        "action": "open",
        "url": "http://example.com/test-open"
    });

    let reply_str = browser::handle_browser_op(&open_op).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse open reply");

    assert_eq!(reply["type"], MSG_TYPE_BROWSER_OP_RESULT);
    assert_eq!(reply["id"], "req-browser-open-1");
    assert_eq!(reply["success"], true);
    assert_eq!(reply["status"], "ok");
    assert_eq!(reply["action"], "open");
    assert_eq!(reply["data"]["opened"], true);

    // ------------------------------------------------------------- 2. Navigate action
    let nav_op = json!({
        "type": MSG_TYPE_BROWSER_OP,
        "id": "req-browser-nav-1",
        "action": "navigate",
        "url": "http://example.com/dashboard"
    });

    let reply_str = browser::handle_browser_op(&nav_op).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse navigate reply");

    assert_eq!(reply["type"], MSG_TYPE_BROWSER_OP_RESULT);
    assert_eq!(reply["id"], "req-browser-nav-1");
    assert_eq!(reply["success"], true);
    assert_eq!(reply["status"], "ok");
    assert_eq!(reply["action"], "navigate");
    assert_eq!(reply["data"]["navigated"], true);
    assert_eq!(reply["data"]["url"], "http://example.com/dashboard");

    // ------------------------------------------------------------- 3. Click action (selector)
    let click_sel_op = json!({
        "type": MSG_TYPE_BROWSER_OP,
        "id": "req-browser-click-1",
        "action": "click",
        "selector": "#action-btn"
    });

    let reply_str = browser::handle_browser_op(&click_sel_op).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse click selector reply");

    assert_eq!(reply["success"], true);
    assert_eq!(reply["action"], "click");
    assert_eq!(reply["data"]["clicked"], true);
    assert_eq!(reply["data"]["selector"], "#action-btn");

    // ------------------------------------------------------------- 4. Click action (coordinates)
    let click_coord_op = json!({
        "type": MSG_TYPE_BROWSER_OP,
        "id": "req-browser-click-coord",
        "action": "click",
        "x": 150.0,
        "y": 250.0
    });

    let reply_str = browser::handle_browser_op(&click_coord_op).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse click coord reply");

    assert_eq!(reply["success"], true);
    assert_eq!(reply["data"]["clicked"], true);
    assert_eq!(reply["data"]["x"], 150.0);
    assert_eq!(reply["data"]["y"], 250.0);

    // ------------------------------------------------------------- 5. Type action
    let type_op = json!({
        "type": MSG_TYPE_BROWSER_OP,
        "id": "req-browser-type-1",
        "action": "type",
        "selector": "#search-field",
        "text": "AQVN AI Workspace"
    });

    let reply_str = browser::handle_browser_op(&type_op).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse type reply");

    assert_eq!(reply["success"], true);
    assert_eq!(reply["action"], "type");
    assert_eq!(reply["data"]["typed"], true);
    assert_eq!(reply["data"]["selector"], "#search-field");

    // ------------------------------------------------------------- 6. Screenshot action
    let screenshot_op = json!({
        "type": MSG_TYPE_BROWSER_OP,
        "id": "req-browser-shot-1",
        "action": "screenshot",
        "format": "png"
    });

    let reply_str = browser::handle_browser_op(&screenshot_op).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse screenshot reply");

    assert_eq!(reply["success"], true);
    assert_eq!(reply["action"], "screenshot");
    assert_eq!(reply["data"]["format"], "png");
    assert!(reply["data"]["data"].as_str().unwrap().len() > 10);

    // ------------------------------------------------------------- 7. Evaluate action
    let eval_op = json!({
        "type": MSG_TYPE_BROWSER_OP,
        "id": "req-browser-eval-1",
        "action": "evaluate",
        "expression": "2 + 2"
    });

    let reply_str = browser::handle_browser_op(&eval_op).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse eval reply");

    assert_eq!(reply["success"], true);
    assert_eq!(reply["action"], "evaluate");
    assert_eq!(reply["data"]["expression"], "2 + 2");
    assert_eq!(reply["data"]["value"], 4);

    // ------------------------------------------------------------- 8. open_remote_debugging
    let mock_bin = create_mock_browser_binary();
    set_test_browser_binary(Some(mock_bin.clone()));

    let debug_op = json!({
        "type": MSG_TYPE_BROWSER_OP,
        "id": "req-browser-debug-1",
        "action": "open_remote_debugging",
        "port": 9222,
        "url": "http://example.com/debug"
    });

    let reply_str = browser::handle_browser_op(&debug_op).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse remote debug reply");

    assert_eq!(reply["success"], true);
    assert_eq!(reply["action"], "open_remote_debugging");
    assert_eq!(reply["data"]["launched"], true);
    assert_eq!(reply["data"]["port"], 9222);
    assert!(reply["data"]["pid"].as_u64().unwrap_or(0) > 0);

    let _ = std::fs::remove_file(mock_bin);

    // ------------------------------------------------------------- 9. Error cases
    // Missing URL in navigate
    let bad_nav = json!({
        "type": MSG_TYPE_BROWSER_OP,
        "id": "req-bad-nav",
        "action": "navigate"
    });
    let bad_reply: Value =
        serde_json::from_str(&browser::handle_browser_op(&bad_nav).await).unwrap();
    assert_eq!(bad_reply["success"], false);
    assert_eq!(bad_reply["status"], "error");

    // Missing expression in evaluate
    let bad_eval = json!({
        "type": MSG_TYPE_BROWSER_OP,
        "id": "req-bad-eval",
        "action": "evaluate"
    });
    let bad_reply: Value =
        serde_json::from_str(&browser::handle_browser_op(&bad_eval).await).unwrap();
    assert_eq!(bad_reply["success"], false);

    // Missing text in type
    let bad_type = json!({
        "type": MSG_TYPE_BROWSER_OP,
        "id": "req-bad-type",
        "action": "type"
    });
    let bad_reply: Value =
        serde_json::from_str(&browser::handle_browser_op(&bad_type).await).unwrap();
    assert_eq!(bad_reply["success"], false);

    // Unknown action
    let unknown_op = json!({
        "type": MSG_TYPE_BROWSER_OP,
        "id": "req-unknown",
        "action": "fly_to_moon"
    });
    let bad_reply: Value =
        serde_json::from_str(&browser::handle_browser_op(&unknown_op).await).unwrap();
    assert_eq!(bad_reply["success"], false);

    // Clean up test endpoints
    set_test_cdp_endpoint(None);
    set_test_browser_binary(None);
}

#[tokio::test]
async fn test_gateway_client_browser_op_dispatch() {
    let _guard = TEST_LOCK.lock().await;
    let (cdp_port, _cdp_handle) = run_mock_cdp_server().await;
    let cdp_url = format!("http://{}:{cdp_port}", Ipv4Addr::LOCALHOST);
    set_test_cdp_endpoint(Some(cdp_url));

    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .expect("bind ephemeral port for mock server");
    let ws_port = listener.local_addr().unwrap().port();

    let (received_tx, mut received_rx) = mpsc::channel::<Value>(16);
    let received_tx = Arc::new(Mutex::new(received_tx));

    let app = Router::new().route(
        "/ws",
        get(move |ws: WebSocketUpgrade| {
            let tx_clone = received_tx.clone();
            async move {
                ws.on_upgrade(move |mut socket: WebSocket| async move {
                    while let Some(Ok(msg)) = socket.next().await {
                        if let Message::Text(text) = msg {
                            if let Ok(val) = serde_json::from_str::<Value>(text.as_str()) {
                                let op_type = val
                                    .get("type")
                                    .or_else(|| val.get("op"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or_default();

                                if op_type == MSG_TYPE_HELLO {
                                    let ack = json!({
                                        "type": MSG_TYPE_HELLO_ACK,
                                        "status": "ok",
                                    });
                                    let _ =
                                        socket.send(Message::Text(ack.to_string().into())).await;

                                    // Send a connector_browser_op to the client
                                    let browser_req = json!({
                                        "type": MSG_TYPE_BROWSER_OP,
                                        "op": MSG_TYPE_BROWSER_OP,
                                        "id": "ws-req-browser-1",
                                        "action": "evaluate",
                                        "expression": "2 + 2"
                                    });
                                    let _ = socket
                                        .send(Message::Text(browser_req.to_string().into()))
                                        .await;
                                } else if op_type == MSG_TYPE_BROWSER_OP_RESULT {
                                    let tx = tx_clone.lock().await;
                                    let _ = tx.send(val).await;
                                }
                            }
                        }
                    }
                })
            }
        }),
    );

    let _server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let config = GatewayConfig {
        enabled: true,
        server_url: format!("http://{}:{ws_port}", Ipv4Addr::LOCALHOST),
        gateway_id: "test-gw-browser-ws".to_string(),
        host_label: "test-host".to_string(),
        session_cookie: None,
        scope_files: false,
        scope_exec: false,
        scope_browser: true,
        scope_desktop: false,
        ping_interval_secs: 10,
        reconnect_delay_ms: 50,
        ..Default::default()
    };

    let client = GatewayClient::new(config);
    let (handle, _join) = client.start();

    // Wait for the result to come back over WebSocket
    let res = tokio::time::timeout(Duration::from_secs(5), received_rx.recv()).await;
    assert!(
        res.is_ok(),
        "timed out waiting for connector_browser_op_result over WS"
    );
    let result_msg = res.unwrap().expect("received browser op result");

    assert_eq!(result_msg["type"], MSG_TYPE_BROWSER_OP_RESULT);
    assert_eq!(result_msg["id"], "ws-req-browser-1");
    assert_eq!(result_msg["success"], true);
    assert_eq!(result_msg["action"], "evaluate");
    assert_eq!(result_msg["data"]["value"], 4);

    handle.stop();
    set_test_cdp_endpoint(None);
}
