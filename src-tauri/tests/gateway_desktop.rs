//! Integration tests for host computer use via connector_computer_use_op.
//!
//! Fulfills VAL-GATEWAY-005:
//! - Handles connector_computer_use_op actions (capture, mouse movement, clicks, keystrokes)
//! - Captures host desktop screen and active window
//! - Injects mouse movement, clicks, double clicks, drags, and scrolls
//! - Injects text typing, key presses, and hotkey combinations
//! - Replies with connector_computer_use_op_result over WebSocket

use aqvn_workspace_lib::connector::client::{GatewayClient, GatewayConfig};
use aqvn_workspace_lib::connector::handlers::computer_use::{self, set_test_screen_capture};
use aqvn_workspace_lib::connector::handlers::dispatch_message;
use aqvn_workspace_lib::connector::protocol::{
    MSG_TYPE_COMPUTER_USE_OP, MSG_TYPE_COMPUTER_USE_OP_RESULT, MSG_TYPE_HELLO, MSG_TYPE_HELLO_ACK,
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
async fn test_gateway_computer_use() {
    // ------------------------------------------------------------- 1. Screen capture action
    let capture_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "op": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-capture-1",
        "action": "capture",
        "format": "png"
    });

    let reply_str = computer_use::handle_computer_use_op(&capture_op).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse capture reply");

    assert_eq!(reply["type"], MSG_TYPE_COMPUTER_USE_OP_RESULT);
    assert_eq!(reply["op"], MSG_TYPE_COMPUTER_USE_OP_RESULT);
    assert_eq!(reply["id"], "req-cu-capture-1");
    assert_eq!(reply["success"], true);
    assert_eq!(reply["status"], "ok");
    assert_eq!(reply["action"], "capture");
    assert!(reply["data"]["captured"].as_bool().unwrap_or(false));
    assert_eq!(reply["data"]["format"], "png");
    let screenshot_b64 = reply["screenshot"]
        .as_str()
        .or_else(|| reply["data"]["data"].as_str())
        .expect("screenshot base64 string present");
    assert!(
        !screenshot_b64.is_empty(),
        "screenshot base64 must not be empty"
    );

    // Test override screen capture
    set_test_screen_capture(Some(json!({
        "data": "aGVsbG8tdGVzdC1zY3JlZW5zaG90",
        "width": 1280,
        "height": 720
    })));
    let override_reply_str = computer_use::handle_computer_use_op(&capture_op).await;
    let override_reply: Value =
        serde_json::from_str(&override_reply_str).expect("parse override reply");
    assert_eq!(override_reply["screenshot"], "aGVsbG8tdGVzdC1zY3JlZW5zaG90");
    assert_eq!(override_reply["data"]["width"], 1280);
    assert_eq!(override_reply["data"]["height"], 720);
    set_test_screen_capture(None);

    // ------------------------------------------------------------- 2. Active window capture
    let active_win_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-capture-win",
        "action": "active_window"
    });

    let reply_win_str = computer_use::handle_computer_use_op(&active_win_op).await;
    let reply_win: Value =
        serde_json::from_str(&reply_win_str).expect("parse active window capture reply");
    assert_eq!(reply_win["success"], true);
    assert_eq!(reply_win["action"], "active_window");

    // ------------------------------------------------------------- 3. Mouse move action
    let move_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-move-1",
        "action": "mouse_move",
        "x": 350.0,
        "y": 450.0
    });

    let reply_move_str = computer_use::handle_computer_use_op(&move_op).await;
    let reply_move: Value = serde_json::from_str(&reply_move_str).expect("parse move reply");
    assert_eq!(reply_move["type"], MSG_TYPE_COMPUTER_USE_OP_RESULT);
    assert_eq!(reply_move["id"], "req-cu-move-1");
    assert_eq!(reply_move["success"], true);
    assert_eq!(reply_move["action"], "mouse_move");
    assert_eq!(reply_move["data"]["moved"], true);
    assert_eq!(reply_move["data"]["x"], 350.0);
    assert_eq!(reply_move["data"]["y"], 450.0);

    // ------------------------------------------------------------- 4. Cursor position query
    let cursor_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-cursor-1",
        "action": "cursor_position"
    });

    let reply_cursor_str = computer_use::handle_computer_use_op(&cursor_op).await;
    let reply_cursor: Value = serde_json::from_str(&reply_cursor_str).expect("parse cursor reply");
    assert_eq!(reply_cursor["success"], true);
    assert_eq!(reply_cursor["data"]["x"], 350.0);
    assert_eq!(reply_cursor["data"]["y"], 450.0);

    // ------------------------------------------------------------- 5. Mouse click action
    let click_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-click-1",
        "action": "click",
        "button": "left",
        "x": 200.0,
        "y": 300.0
    });

    let reply_click_str = computer_use::handle_computer_use_op(&click_op).await;
    let reply_click: Value = serde_json::from_str(&reply_click_str).expect("parse click reply");
    assert_eq!(reply_click["success"], true);
    assert_eq!(reply_click["action"], "click");
    assert_eq!(reply_click["data"]["clicked"], true);
    assert_eq!(reply_click["data"]["button"], "left");
    assert_eq!(reply_click["data"]["x"], 200.0);
    assert_eq!(reply_click["data"]["y"], 300.0);

    // ------------------------------------------------------------- 6. Double click
    let dbl_click_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-dblclick-1",
        "action": "double_click",
        "button": "left"
    });

    let reply_dbl_str = computer_use::handle_computer_use_op(&dbl_click_op).await;
    let reply_dbl: Value = serde_json::from_str(&reply_dbl_str).expect("parse dbl click reply");
    assert_eq!(reply_dbl["success"], true);
    assert_eq!(reply_dbl["data"]["clicked"], true);
    assert_eq!(reply_dbl["data"]["count"], 2);

    // ------------------------------------------------------------- 7. Mouse drag
    let drag_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-drag-1",
        "action": "mouse_drag",
        "start_x": 100.0,
        "start_y": 100.0,
        "end_x": 300.0,
        "end_y": 300.0
    });

    let reply_drag_str = computer_use::handle_computer_use_op(&drag_op).await;
    let reply_drag: Value = serde_json::from_str(&reply_drag_str).expect("parse drag reply");
    assert_eq!(reply_drag["success"], true);
    assert_eq!(reply_drag["data"]["dragged"], true);

    // ------------------------------------------------------------- 8. Mouse scroll
    let scroll_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-scroll-1",
        "action": "mouse_scroll",
        "delta_x": 0.0,
        "delta_y": -120.0
    });

    let reply_scroll_str = computer_use::handle_computer_use_op(&scroll_op).await;
    let reply_scroll: Value = serde_json::from_str(&reply_scroll_str).expect("parse scroll reply");
    assert_eq!(reply_scroll["success"], true);
    assert_eq!(reply_scroll["data"]["scrolled"], true);

    // ------------------------------------------------------------- 9. Text typing
    let type_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-type-1",
        "action": "type",
        "text": "AQVN Workspace Native"
    });

    let reply_type_str = computer_use::handle_computer_use_op(&type_op).await;
    let reply_type: Value = serde_json::from_str(&reply_type_str).expect("parse type reply");
    assert_eq!(reply_type["success"], true);
    assert_eq!(reply_type["action"], "type");
    assert_eq!(reply_type["data"]["typed"], true);
    assert_eq!(reply_type["data"]["length"], 21);

    // ------------------------------------------------------------- 10. Key press
    let key_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-key-1",
        "action": "key",
        "key": "Return"
    });

    let reply_key_str = computer_use::handle_computer_use_op(&key_op).await;
    let reply_key: Value = serde_json::from_str(&reply_key_str).expect("parse key reply");
    assert_eq!(reply_key["success"], true);
    assert_eq!(reply_key["action"], "key");
    assert_eq!(reply_key["data"]["pressed"], true);
    assert_eq!(reply_key["data"]["key"], "Return");

    // ------------------------------------------------------------- 11. Hotkey combination
    let hotkey_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-hotkey-1",
        "action": "hotkey",
        "keys": ["Control", "Shift", "P"]
    });

    let reply_hotkey_str = computer_use::handle_computer_use_op(&hotkey_op).await;
    let reply_hotkey: Value = serde_json::from_str(&reply_hotkey_str).expect("parse hotkey reply");
    assert_eq!(reply_hotkey["success"], true);
    assert_eq!(reply_hotkey["data"]["hotkey"], true);

    // ------------------------------------------------------------- 12. Displays/screens list
    let screens_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-screens-1",
        "action": "screens"
    });

    let reply_screens_str = computer_use::handle_computer_use_op(&screens_op).await;
    let reply_screens: Value =
        serde_json::from_str(&reply_screens_str).expect("parse screens reply");
    assert_eq!(reply_screens["success"], true);
    assert!(reply_screens["data"]["displays"].is_array());

    // ------------------------------------------------------------- 13. Unknown action error handling
    let unknown_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-unknown-1",
        "action": "nonexistent_action"
    });

    let reply_unknown_str = computer_use::handle_computer_use_op(&unknown_op).await;
    let reply_unknown: Value =
        serde_json::from_str(&reply_unknown_str).expect("parse unknown reply");
    assert_eq!(reply_unknown["success"], false);
    assert_eq!(reply_unknown["status"], "error");

    // ------------------------------------------------------------- 14. Dispatch via dispatch_message
    let dispatch_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-dispatch-1",
        "action": "mouse_move",
        "x": 100.0,
        "y": 150.0
    });

    let dispatch_reply_str = dispatch_message(&dispatch_op.to_string())
        .await
        .expect("dispatch_message must handle connector_computer_use_op");
    let dispatch_reply: Value = serde_json::from_str(&dispatch_reply_str).unwrap();
    assert_eq!(dispatch_reply["type"], MSG_TYPE_COMPUTER_USE_OP_RESULT);
    assert_eq!(dispatch_reply["id"], "req-cu-dispatch-1");
    assert_eq!(dispatch_reply["success"], true);

    // ------------------------------------------------------------- 15. WebSocket end-to-end integration
    let (tx_request, rx_request) = mpsc::channel::<String>(16);
    let rx_request_mutex = Arc::new(Mutex::new(rx_request));
    let (tx_reply, rx_reply) = mpsc::channel::<String>(16);
    let rx_reply_mutex = Arc::new(Mutex::new(rx_reply));

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
        gateway_id: "test-gw-desktop-1".to_string(),
        host_label: "test-host".to_string(),
        session_cookie: None,
        scope_files: false,
        scope_exec: false,
        scope_browser: false,
        scope_desktop: true,
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

    // Send capture operation over WebSocket
    let ws_capture_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "ws-req-cu-capture",
        "action": "capture",
        "format": "png"
    });
    tx_request.send(ws_capture_op.to_string()).await.unwrap();

    let ws_reply_text = rx_reply_mutex
        .lock()
        .await
        .recv()
        .await
        .expect("receive ws reply for capture");
    let ws_reply: Value = serde_json::from_str(&ws_reply_text).unwrap();
    assert_eq!(ws_reply["type"], MSG_TYPE_COMPUTER_USE_OP_RESULT);
    assert_eq!(ws_reply["id"], "ws-req-cu-capture");
    assert_eq!(ws_reply["success"], true);
    assert_eq!(ws_reply["action"], "capture");

    // Send mouse click operation over WebSocket
    let ws_click_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "ws-req-cu-click",
        "action": "click",
        "button": "left",
        "x": 500.0,
        "y": 600.0
    });
    tx_request.send(ws_click_op.to_string()).await.unwrap();

    let ws_click_reply_text = rx_reply_mutex
        .lock()
        .await
        .recv()
        .await
        .expect("receive ws reply for click");
    let ws_click_reply: Value = serde_json::from_str(&ws_click_reply_text).unwrap();
    assert_eq!(ws_click_reply["type"], MSG_TYPE_COMPUTER_USE_OP_RESULT);
    assert_eq!(ws_click_reply["id"], "ws-req-cu-click");
    assert_eq!(ws_click_reply["success"], true);
    assert_eq!(ws_click_reply["action"], "click");
    assert_eq!(ws_click_reply["data"]["x"], 500.0);
    assert_eq!(ws_click_reply["data"]["y"], 600.0);

    handle.stop();
    let _ = join_handle.await;
    server_task.abort();
}

#[tokio::test]
async fn test_screenshot_scaling_and_compression() {
    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
    use base64::Engine;
    use std::io::Cursor;

    // Create an 800x600 PNG image with gradient pattern to give realistic non-trivial bytes
    let orig_w = 800u32;
    let orig_h = 600u32;
    let mut img_buf = image::RgbaImage::new(orig_w, orig_h);
    for (x, y, pixel) in img_buf.enumerate_pixels_mut() {
        *pixel = image::Rgba([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8, 255]);
    }

    let mut orig_png_bytes = Vec::new();
    img_buf
        .write_to(
            &mut Cursor::new(&mut orig_png_bytes),
            image::ImageFormat::Png,
        )
        .expect("write test png");
    let orig_b64 = BASE64_STANDARD.encode(&orig_png_bytes);

    set_test_screen_capture(Some(json!({
        "data": orig_b64,
        "width": orig_w,
        "height": orig_h,
    })));

    // 1. Direct call to capture_screen_native with scale = 0.5
    let (scaled_b64, w_s, h_s) =
        computer_use::capture_screen_native("png", false, None, Some(0.5), None);
    assert_eq!(w_s, 400);
    assert_eq!(h_s, 300);

    let decoded_bytes = BASE64_STANDARD.decode(&scaled_b64).expect("valid base64");
    let decoded_img = image::load_from_memory(&decoded_bytes).expect("valid png image");
    assert_eq!(decoded_img.width(), 400);
    assert_eq!(decoded_img.height(), 300);
    assert!(
        scaled_b64.len() < orig_b64.len(),
        "Scaled screenshot base64 (len {}) must be smaller than original (len {}) to reduce token payload",
        scaled_b64.len(),
        orig_b64.len()
    );

    // 2. Direct call to capture_screen_native with max_width = 200
    let (mw_b64, w_mw, h_mw) =
        computer_use::capture_screen_native("png", false, None, None, Some(200));
    assert_eq!(w_mw, 200);
    assert_eq!(h_mw, 150);

    let mw_bytes = BASE64_STANDARD.decode(&mw_b64).expect("valid base64");
    let mw_img = image::load_from_memory(&mw_bytes).expect("valid png image");
    assert_eq!(mw_img.width(), 200);
    assert_eq!(mw_img.height(), 150);
    assert!(mw_b64.len() < scaled_b64.len());

    // 3. Direct call with both scale = 0.5 and max_width = 200
    let (_both_b64, w_both, h_both) =
        computer_use::capture_screen_native("png", false, None, Some(0.5), Some(200));
    assert_eq!(w_both, 200);
    assert_eq!(h_both, 150);

    // 4. Test format conversion to JPEG
    let (jpeg_b64, w_jpeg, h_jpeg) =
        computer_use::capture_screen_native("jpeg", false, None, Some(0.5), None);
    assert_eq!(w_jpeg, 400);
    assert_eq!(h_jpeg, 300);
    let jpeg_bytes = BASE64_STANDARD.decode(&jpeg_b64).expect("valid base64");
    let jpeg_img = image::load_from_memory(&jpeg_bytes).expect("valid jpeg image");
    assert_eq!(jpeg_img.width(), 400);
    assert_eq!(jpeg_img.height(), 300);

    // 5. Op dispatch via handle_computer_use_op with scale
    let scale_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-scale-half",
        "action": "capture",
        "scale": 0.5,
    });
    let reply_str = computer_use::handle_computer_use_op(&scale_op).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse scale reply");
    assert_eq!(reply["success"], true);
    assert_eq!(reply["data"]["width"], 400);
    assert_eq!(reply["data"]["height"], 300);
    let op_screenshot = reply["screenshot"].as_str().expect("screenshot present");
    let op_bytes = BASE64_STANDARD
        .decode(op_screenshot)
        .expect("decode op screenshot");
    let op_img = image::load_from_memory(&op_bytes).expect("load op screenshot");
    assert_eq!(op_img.width(), 400);
    assert_eq!(op_img.height(), 300);

    // 6. Op dispatch via handle_computer_use_op with max_width
    let mw_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-mw",
        "action": "capture",
        "max_width": 200,
    });
    let mw_reply_str = computer_use::handle_computer_use_op(&mw_op).await;
    let mw_reply: Value = serde_json::from_str(&mw_reply_str).expect("parse mw reply");
    assert_eq!(mw_reply["success"], true);
    assert_eq!(mw_reply["data"]["width"], 200);
    assert_eq!(mw_reply["data"]["height"], 150);

    // 7. Op dispatch active_window with scale inside params
    let act_win_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-act-scale",
        "action": "active_window",
        "params": {
            "scale": 0.25
        }
    });
    let act_reply_str = computer_use::handle_computer_use_op(&act_win_op).await;
    let act_reply: Value = serde_json::from_str(&act_reply_str).expect("parse act win reply");
    assert_eq!(act_reply["success"], true);
    assert_eq!(act_reply["data"]["width"], 200);
    assert_eq!(act_reply["data"]["height"], 150);

    set_test_screen_capture(None);
}
