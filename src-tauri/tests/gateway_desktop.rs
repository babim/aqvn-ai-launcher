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

static TEST_GATEWAY_MUTEX: Mutex<()> = Mutex::const_new(());

#[tokio::test]
async fn test_gateway_computer_use() {
    let _lock = TEST_GATEWAY_MUTEX.lock().await;
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
    let _lock = TEST_GATEWAY_MUTEX.lock().await;
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

#[tokio::test]
async fn test_windows_sendinput_compiles_or_simulates() {
    let _lock = TEST_GATEWAY_MUTEX.lock().await;
    use aqvn_workspace_lib::connector::handlers::computer_use::{
        clear_injected_events, clear_last_windows_inputs, get_last_windows_inputs,
        handle_computer_use_op, normalize_screen_coordinates, simulate_windows_sendinput,
        WIN_KEYEVENTF_KEYUP, WIN_KEYEVENTF_UNICODE, WIN_MOUSEEVENTF_ABSOLUTE,
        WIN_MOUSEEVENTF_HWHEEL, WIN_MOUSEEVENTF_LEFTDOWN, WIN_MOUSEEVENTF_LEFTUP,
        WIN_MOUSEEVENTF_MOVE, WIN_MOUSEEVENTF_RIGHTDOWN, WIN_MOUSEEVENTF_RIGHTUP,
        WIN_MOUSEEVENTF_WHEEL, WIN_VK_ESCAPE, WIN_VK_RETURN,
    };

    clear_injected_events();
    clear_last_windows_inputs();

    // 1. Coordinate normalization (screen -> 0..65535)
    let (nx, ny) = normalize_screen_coordinates(960.0, 540.0, 1920.0, 1080.0);
    assert_eq!(nx, 32768);
    assert_eq!(ny, 32768);

    let (nx_zero, ny_zero) = normalize_screen_coordinates(0.0, 0.0, 1920.0, 1080.0);
    assert_eq!(nx_zero, 0);
    assert_eq!(ny_zero, 0);

    let (nx_max, ny_max) = normalize_screen_coordinates(1920.0, 1080.0, 1920.0, 1080.0);
    assert_eq!(nx_max, 65535);
    assert_eq!(ny_max, 65535);

    // 2. Mouse move action dispatch & Windows SendInput generation
    let move_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "test-win-move",
        "action": "mouse_move",
        "x": 960.0,
        "y": 540.0
    });
    let reply_str = handle_computer_use_op(&move_op).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("valid json reply");
    assert_eq!(reply["success"], true);

    let win_inputs = get_last_windows_inputs();
    assert!(!win_inputs.is_empty(), "must record windows inputs");
    let last_move = win_inputs.last().unwrap();
    assert_eq!(last_move.input_type, "mouse");
    assert_eq!(
        last_move.flags & (WIN_MOUSEEVENTF_MOVE | WIN_MOUSEEVENTF_ABSOLUTE),
        WIN_MOUSEEVENTF_MOVE | WIN_MOUSEEVENTF_ABSOLUTE
    );
    assert_eq!(last_move.dx, 32768);
    assert_eq!(last_move.dy, 32768);

    // 3. Mouse click (left and right)
    clear_last_windows_inputs();
    let click_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "test-win-click",
        "action": "click",
        "button": "left"
    });
    let reply_str = handle_computer_use_op(&click_op).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("valid reply");
    assert_eq!(reply["success"], true);

    let win_clicks = get_last_windows_inputs();
    assert_eq!(win_clicks.len(), 2, "left click produces down + up");
    assert_eq!(win_clicks[0].flags, WIN_MOUSEEVENTF_LEFTDOWN);
    assert_eq!(win_clicks[1].flags, WIN_MOUSEEVENTF_LEFTUP);

    // Right double click
    clear_last_windows_inputs();
    let rclick_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "test-win-rclick",
        "action": "double_click",
        "button": "right"
    });
    let reply_str = handle_computer_use_op(&rclick_op).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("valid reply");
    assert_eq!(reply["success"], true);

    let win_rclicks = get_last_windows_inputs();
    assert_eq!(win_rclicks.len(), 4, "right double click produces 4 events");
    assert_eq!(win_rclicks[0].flags, WIN_MOUSEEVENTF_RIGHTDOWN);
    assert_eq!(win_rclicks[1].flags, WIN_MOUSEEVENTF_RIGHTUP);
    assert_eq!(win_rclicks[2].flags, WIN_MOUSEEVENTF_RIGHTDOWN);
    assert_eq!(win_rclicks[3].flags, WIN_MOUSEEVENTF_RIGHTUP);

    // 4. Mouse wheel scroll (vertical & horizontal)
    clear_last_windows_inputs();
    let scroll_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "test-win-scroll",
        "action": "scroll",
        "delta_x": 1.0,
        "delta_y": -2.0
    });
    let reply_str = handle_computer_use_op(&scroll_op).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("valid reply");
    assert_eq!(reply["success"], true);

    let win_scrolls = get_last_windows_inputs();
    assert_eq!(win_scrolls.len(), 2, "vertical + horizontal scroll events");
    let vert_scroll = &win_scrolls[0];
    assert_eq!(vert_scroll.flags, WIN_MOUSEEVENTF_WHEEL);
    assert_eq!(vert_scroll.mouse_data as i32, -240); // -2.0 * 120
    let horiz_scroll = &win_scrolls[1];
    assert_eq!(horiz_scroll.flags, WIN_MOUSEEVENTF_HWHEEL);
    assert_eq!(horiz_scroll.mouse_data as i32, 120); // 1.0 * 120

    // 5. Text typing via SendInput KEYEVENTF_UNICODE
    clear_last_windows_inputs();
    let type_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "test-win-type",
        "action": "type",
        "text": "AQVN"
    });
    let reply_str = handle_computer_use_op(&type_op).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("valid reply");
    assert_eq!(reply["success"], true);

    let win_types = get_last_windows_inputs();
    assert_eq!(win_types.len(), 8, "4 chars * 2 (down + up)");
    assert_eq!(win_types[0].scan, 'A' as u16);
    assert_eq!(win_types[0].flags, WIN_KEYEVENTF_UNICODE);
    assert_eq!(win_types[1].scan, 'A' as u16);
    assert_eq!(
        win_types[1].flags,
        WIN_KEYEVENTF_UNICODE | WIN_KEYEVENTF_KEYUP
    );

    // 6. Key press (Virtual Key mapping: Return, Escape)
    clear_last_windows_inputs();
    let key_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "test-win-key",
        "action": "key",
        "key": "Return"
    });
    let reply_str = handle_computer_use_op(&key_op).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("valid reply");
    assert_eq!(reply["success"], true);

    let win_keys = get_last_windows_inputs();
    assert_eq!(win_keys.len(), 2);
    assert_eq!(win_keys[0].vk, WIN_VK_RETURN);
    assert_eq!(win_keys[0].flags, 0);
    assert_eq!(win_keys[1].vk, WIN_VK_RETURN);
    assert_eq!(win_keys[1].flags, WIN_KEYEVENTF_KEYUP);

    // 7. Direct simulate_windows_sendinput verification
    let sim_esc = simulate_windows_sendinput("key", &json!({"key": "Escape"})).expect("sim esc");
    assert_eq!(sim_esc.len(), 2);
    assert_eq!(sim_esc[0].vk, WIN_VK_ESCAPE);

    let sim_hk =
        simulate_windows_sendinput("hotkey", &json!({"keys": ["Ctrl", "C"]})).expect("sim hotkey");
    assert_eq!(sim_hk.len(), 4, "Ctrl down, C down, C up, Ctrl up");
}

#[tokio::test]
async fn test_computer_use_multi_display_and_cursor_overlay() {
    let _lock = TEST_GATEWAY_MUTEX.lock().await;
    use aqvn_workspace_lib::connector::handlers::computer_use::{
        self, clear_test_display_captures, list_displays, set_cursor_position,
        set_test_display_capture, set_test_displays, set_test_screen_capture, DisplayInfo,
    };
    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
    use base64::Engine;
    use std::io::Cursor;

    // Reset test state at start
    set_test_displays(None);
    set_test_screen_capture(None);
    clear_test_display_captures();

    // 1. Default list_displays returns primary display
    let default_displays = list_displays();
    assert!(
        !default_displays.is_empty(),
        "must have at least one display"
    );
    assert_eq!(default_displays[0].id, 0);
    assert!(default_displays[0].is_primary);

    // Also via JSON-RPC / envelope action "list_displays"
    let list_disp_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-list-disp-1",
        "action": "list_displays",
    });
    let reply_str = computer_use::handle_computer_use_op(&list_disp_op).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("parse list_displays reply");
    assert_eq!(reply["success"], true);
    assert_eq!(reply["action"], "list_displays");
    assert!(reply["data"]["displays"].is_array());
    assert!(!reply["data"]["displays"].as_array().unwrap().is_empty());

    // 2. Multi-display configuration override
    let mock_monitors = vec![
        DisplayInfo {
            id: 0,
            name: "Primary Monitor (4K)".to_string(),
            width: 3840,
            height: 2160,
            scale_factor: 2.0,
            is_primary: true,
        },
        DisplayInfo {
            id: 1,
            name: "Secondary Monitor (1080p)".to_string(),
            width: 1920,
            height: 1080,
            scale_factor: 1.0,
            is_primary: false,
        },
    ];
    set_test_displays(Some(mock_monitors.clone()));

    let updated_displays = list_displays();
    assert_eq!(updated_displays.len(), 2);
    assert_eq!(updated_displays[0].id, 0);
    assert_eq!(updated_displays[0].width, 3840);
    assert_eq!(updated_displays[1].id, 1);
    assert_eq!(updated_displays[1].width, 1920);

    // Verify op dispatch returns both monitors
    let op_displays_reply_str = computer_use::handle_computer_use_op(&list_disp_op).await;
    let op_displays_reply: Value = serde_json::from_str(&op_displays_reply_str).unwrap();
    assert_eq!(
        op_displays_reply["data"]["displays"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    // 3. Screen capture targeting display_id
    // Set per-display test capture image buffers
    let mut img_disp0 = image::RgbaImage::new(400, 300);
    for pixel in img_disp0.pixels_mut() {
        *pixel = image::Rgba([10, 20, 30, 255]); // dark blue
    }
    let mut disp0_png = Vec::new();
    img_disp0
        .write_to(&mut Cursor::new(&mut disp0_png), image::ImageFormat::Png)
        .unwrap();
    let disp0_b64 = BASE64_STANDARD.encode(&disp0_png);

    let mut img_disp1 = image::RgbaImage::new(600, 450);
    for pixel in img_disp1.pixels_mut() {
        *pixel = image::Rgba([40, 50, 60, 255]); // greenish blue
    }
    let mut disp1_png = Vec::new();
    img_disp1
        .write_to(&mut Cursor::new(&mut disp1_png), image::ImageFormat::Png)
        .unwrap();
    let disp1_b64 = BASE64_STANDARD.encode(&disp1_png);

    set_test_display_capture(
        0,
        Some(json!({
            "data": disp0_b64,
            "width": 400,
            "height": 300,
        })),
    );
    set_test_display_capture(
        1,
        Some(json!({
            "data": disp1_b64,
            "width": 600,
            "height": 450,
        })),
    );

    // Capture Display 0
    let cap_disp0_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-cap-disp-0",
        "action": "capture",
        "display_id": 0,
    });
    let reply_disp0_str = computer_use::handle_computer_use_op(&cap_disp0_op).await;
    let reply_disp0: Value = serde_json::from_str(&reply_disp0_str).unwrap();
    assert_eq!(reply_disp0["success"], true);
    assert_eq!(reply_disp0["data"]["display_id"], 0);
    assert_eq!(reply_disp0["data"]["width"], 400);
    assert_eq!(reply_disp0["data"]["height"], 300);

    // Capture Display 1
    let cap_disp1_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-cap-disp-1",
        "action": "capture",
        "display_id": 1,
    });
    let reply_disp1_str = computer_use::handle_computer_use_op(&cap_disp1_op).await;
    let reply_disp1: Value = serde_json::from_str(&reply_disp1_str).unwrap();
    assert_eq!(reply_disp1["success"], true);
    assert_eq!(reply_disp1["data"]["display_id"], 1);
    assert_eq!(reply_disp1["data"]["width"], 600);
    assert_eq!(reply_disp1["data"]["height"], 450);

    // Capture invalid display_id: 99 returns error
    let cap_invalid_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-cap-invalid",
        "action": "capture",
        "display_id": 99,
    });
    let reply_invalid_str = computer_use::handle_computer_use_op(&cap_invalid_op).await;
    let reply_invalid: Value = serde_json::from_str(&reply_invalid_str).unwrap();
    assert_eq!(reply_invalid["success"], false);
    assert!(reply_invalid["error"].as_str().unwrap().contains("99"));

    // 4. Visual crosshair cursor overlay rendering when show_cursor is true
    clear_test_display_captures();
    // Solid white 800x600 image
    let mut white_img = image::RgbaImage::new(800, 600);
    for pixel in white_img.pixels_mut() {
        *pixel = image::Rgba([255, 255, 255, 255]);
    }
    let mut white_png = Vec::new();
    white_img
        .write_to(&mut Cursor::new(&mut white_png), image::ImageFormat::Png)
        .unwrap();
    let white_b64 = BASE64_STANDARD.encode(&white_png);

    set_test_screen_capture(Some(json!({
        "data": white_b64,
        "width": 800,
        "height": 600,
    })));

    // Set simulated cursor position at (400.0, 300.0)
    set_cursor_position(400.0, 300.0);

    // 4a. Capture without show_cursor (show_cursor: false)
    let no_cursor_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-no-cursor",
        "action": "capture",
        "show_cursor": false,
    });
    let no_cursor_reply_str = computer_use::handle_computer_use_op(&no_cursor_op).await;
    let no_cursor_reply: Value = serde_json::from_str(&no_cursor_reply_str).unwrap();
    assert_eq!(no_cursor_reply["success"], true);
    assert_eq!(no_cursor_reply["data"]["show_cursor"], false);

    let no_cursor_b64 = no_cursor_reply["screenshot"].as_str().unwrap();
    let no_cursor_bytes = BASE64_STANDARD.decode(no_cursor_b64).unwrap();
    let no_cursor_img = image::load_from_memory(&no_cursor_bytes)
        .unwrap()
        .to_rgba8();
    // Center pixel at (400, 300) remains untouched white
    assert_eq!(
        no_cursor_img.get_pixel(400, 300),
        &image::Rgba([255, 255, 255, 255])
    );

    // 4b. Capture with show_cursor: true
    let with_cursor_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-with-cursor",
        "action": "capture",
        "show_cursor": true,
    });
    let with_cursor_reply_str = computer_use::handle_computer_use_op(&with_cursor_op).await;
    let with_cursor_reply: Value = serde_json::from_str(&with_cursor_reply_str).unwrap();
    assert_eq!(with_cursor_reply["success"], true);
    assert_eq!(with_cursor_reply["data"]["show_cursor"], true);

    let with_cursor_b64 = with_cursor_reply["screenshot"].as_str().unwrap();
    let with_cursor_bytes = BASE64_STANDARD.decode(with_cursor_b64).unwrap();
    let with_cursor_img = image::load_from_memory(&with_cursor_bytes)
        .unwrap()
        .to_rgba8();

    // Center pixel at cursor position (400, 300) has crosshair red color
    let center_pixel = with_cursor_img.get_pixel(400, 300);
    assert_eq!(
        center_pixel,
        &image::Rgba([255, 0, 0, 255]),
        "Center cursor must be red"
    );

    // Horizontal arm pixel at (406, 300) is red
    assert_eq!(
        with_cursor_img.get_pixel(406, 300),
        &image::Rgba([255, 0, 0, 255]),
        "Horizontal arm must be red"
    );

    // Vertical arm pixel at (400, 306) is red
    assert_eq!(
        with_cursor_img.get_pixel(400, 306),
        &image::Rgba([255, 0, 0, 255]),
        "Vertical arm must be red"
    );

    // Distant pixel at (50, 50) is still white
    assert_eq!(
        with_cursor_img.get_pixel(50, 50),
        &image::Rgba([255, 255, 255, 255])
    );

    // 4c. Scaled capture with show_cursor: true (scale = 0.5)
    let scaled_cursor_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-scaled-cursor",
        "action": "capture",
        "show_cursor": true,
        "scale": 0.5,
    });
    let scaled_reply_str = computer_use::handle_computer_use_op(&scaled_cursor_op).await;
    let scaled_reply: Value = serde_json::from_str(&scaled_reply_str).unwrap();
    assert_eq!(scaled_reply["success"], true);
    assert_eq!(scaled_reply["data"]["width"], 400);
    assert_eq!(scaled_reply["data"]["height"], 300);

    let scaled_b64 = scaled_reply["screenshot"].as_str().unwrap();
    let scaled_bytes = BASE64_STANDARD.decode(scaled_b64).unwrap();
    let scaled_img = image::load_from_memory(&scaled_bytes).unwrap().to_rgba8();
    // (400, 300) scaled by 0.5 -> (200, 150)
    assert_eq!(
        scaled_img.get_pixel(200, 150),
        &image::Rgba([255, 0, 0, 255]),
        "Scaled cursor at (200, 150) must be red"
    );

    // Cleanup
    set_test_displays(None);
    set_test_screen_capture(None);
    clear_test_display_captures();
}

#[tokio::test]
async fn test_wayland_capture_and_input_support() {
    let _lock = TEST_GATEWAY_MUTEX.lock().await;
    use aqvn_workspace_lib::connector::handlers::computer_use::{
        clear_injected_events, clear_last_wayland_inputs, clear_test_wayland_tools,
        get_last_wayland_inputs, handle_computer_use_op, is_wayland, set_test_screen_capture,
        set_test_wayland_cmd_handler, set_test_wayland_display, set_test_wayland_tool,
    };
    use std::path::PathBuf;
    use std::sync::Arc;

    // Reset initial test state
    set_test_screen_capture(None);
    set_test_wayland_display(None);
    set_test_wayland_cmd_handler(None);
    clear_test_wayland_tools();
    clear_last_wayland_inputs();
    clear_injected_events();

    // 1. Wayland detection
    assert!(!is_wayland(), "Initially not wayland");
    set_test_wayland_display(Some("wayland-0"));
    assert!(
        is_wayland(),
        "is_wayland() returns true under WAYLAND_DISPLAY"
    );

    // 2. Capture under Wayland when grim is NOT found -> explicit descriptive error
    let capture_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-wayland-cap-missing",
        "action": "capture",
        "format": "png",
    });
    let reply_str = handle_computer_use_op(&capture_op).await;
    let reply: Value = serde_json::from_str(&reply_str).expect("valid json reply");
    assert_eq!(reply["success"], false);
    assert_eq!(reply["status"], "error");
    let err_msg = reply["error"].as_str().unwrap_or_default();
    assert!(
        err_msg.to_lowercase().contains("grim"),
        "error must explicitly mention grim: {err_msg}"
    );
    assert!(
        reply.get("screenshot").is_none() || reply["screenshot"].is_null(),
        "must not return silent 1x1 screenshot fallback on failure"
    );

    // 3. Capture under Wayland with grim present (simulated)
    let mut test_png_buf = Vec::new();
    let img_rgba = image::RgbaImage::from_pixel(100, 100, image::Rgba([200, 100, 50, 255]));
    img_rgba
        .write_to(
            &mut std::io::Cursor::new(&mut test_png_buf),
            image::ImageFormat::Png,
        )
        .expect("write test png");
    let test_png_bytes = test_png_buf.clone();

    set_test_wayland_tool("grim", Some(PathBuf::from("/usr/bin/grim")));
    set_test_wayland_cmd_handler(Some(Arc::new(move |tool, _args| {
        if tool == "grim" {
            Ok(test_png_bytes.clone())
        } else {
            Ok(Vec::new())
        }
    })));

    let cap_op_success = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-wayland-cap-ok",
        "action": "capture",
        "format": "png",
    });
    let reply_ok_str = handle_computer_use_op(&cap_op_success).await;
    let reply_ok: Value = serde_json::from_str(&reply_ok_str).expect("valid json reply");
    assert_eq!(reply_ok["success"], true);
    assert_eq!(reply_ok["status"], "ok");
    assert_eq!(reply_ok["data"]["width"], 100);
    assert_eq!(reply_ok["data"]["height"], 100);
    let screenshot_b64 = reply_ok["screenshot"].as_str().unwrap();
    assert!(!screenshot_b64.is_empty());

    let wayland_inputs = get_last_wayland_inputs();
    let grim_call = wayland_inputs.iter().find(|i| i.tool == "grim");
    assert!(grim_call.is_some(), "must record grim command execution");
    let grim_args = &grim_call.unwrap().args;
    assert!(grim_args.contains(&"-t".to_string()));
    assert!(grim_args.contains(&"png".to_string()));
    assert!(grim_args.contains(&"-".to_string()));

    // 4. Capture under Wayland when grim fails -> explicit error
    set_test_wayland_cmd_handler(Some(Arc::new(|tool, _args| {
        if tool == "grim" {
            Err("failed to connect to Wayland display socket".to_string())
        } else {
            Ok(Vec::new())
        }
    })));
    let reply_fail_str = handle_computer_use_op(&capture_op).await;
    let reply_fail: Value = serde_json::from_str(&reply_fail_str).expect("valid json reply");
    assert_eq!(reply_fail["success"], false);
    assert_eq!(reply_fail["status"], "error");
    assert!(reply_fail["error"]
        .as_str()
        .unwrap()
        .contains("failed to connect to Wayland"));

    // 5. Wayland Input injection when tools missing -> explicit error
    clear_test_wayland_tools();
    set_test_wayland_cmd_handler(None);
    clear_last_wayland_inputs();

    let move_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-wayland-move-fail",
        "action": "mouse_move",
        "x": 250.0,
        "y": 350.0,
    });
    let move_fail_str = handle_computer_use_op(&move_op).await;
    let move_fail: Value = serde_json::from_str(&move_fail_str).expect("valid reply");
    assert_eq!(move_fail["success"], false);
    assert_eq!(move_fail["status"], "error");
    assert!(move_fail["error"]
        .as_str()
        .unwrap()
        .to_lowercase()
        .contains("ydotool"));

    let type_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-wayland-type-fail",
        "action": "type",
        "text": "test",
    });
    let type_fail_str = handle_computer_use_op(&type_op).await;
    let type_fail: Value = serde_json::from_str(&type_fail_str).expect("valid reply");
    assert_eq!(type_fail["success"], false);
    assert_eq!(type_fail["status"], "error");
    assert!(
        type_fail["error"].as_str().unwrap().contains("wtype")
            || type_fail["error"].as_str().unwrap().contains("ydotool")
    );

    // 6. Wayland Input injection when tools present -> successful execution
    set_test_wayland_tool("ydotool", Some(PathBuf::from("/usr/bin/ydotool")));
    set_test_wayland_tool("wtype", Some(PathBuf::from("/usr/bin/wtype")));
    set_test_wayland_cmd_handler(Some(Arc::new(|_tool, _args| Ok(Vec::new()))));
    clear_last_wayland_inputs();

    // 6a. Mouse move via ydotool
    let move_ok_str = handle_computer_use_op(&move_op).await;
    let move_ok: Value = serde_json::from_str(&move_ok_str).expect("valid reply");
    assert_eq!(move_ok["success"], true);
    assert_eq!(move_ok["status"], "ok");

    let wayland_inputs = get_last_wayland_inputs();
    let move_call = wayland_inputs
        .iter()
        .find(|i| i.tool == "ydotool" && i.action == "mousemove");
    assert!(move_call.is_some(), "must invoke ydotool mousemove");
    assert!(move_call.unwrap().args.contains(&"--absolute".to_string()));
    assert!(move_call.unwrap().args.contains(&"250".to_string()));
    assert!(move_call.unwrap().args.contains(&"350".to_string()));

    // 6b. Mouse click via ydotool
    clear_last_wayland_inputs();
    let click_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-wayland-click",
        "action": "click",
        "button": "left",
    });
    let click_ok_str = handle_computer_use_op(&click_op).await;
    let click_ok: Value = serde_json::from_str(&click_ok_str).expect("valid reply");
    assert_eq!(click_ok["success"], true);
    assert_eq!(click_ok["status"], "ok");
    let click_call = get_last_wayland_inputs()
        .into_iter()
        .find(|i| i.tool == "ydotool" && i.action == "click");
    assert!(click_call.is_some(), "must invoke ydotool click");

    // 6c. Type text via wtype
    clear_last_wayland_inputs();
    let type_ok_str = handle_computer_use_op(&type_op).await;
    let type_ok: Value = serde_json::from_str(&type_ok_str).expect("valid reply");
    assert_eq!(type_ok["success"], true);
    assert_eq!(type_ok["status"], "ok");
    let type_call = get_last_wayland_inputs()
        .into_iter()
        .find(|i| i.tool == "wtype" && i.action == "type");
    assert!(type_call.is_some(), "must invoke wtype type");
    assert!(type_call.unwrap().args.contains(&"test".to_string()));

    // 6d. Key press via wtype
    clear_last_wayland_inputs();
    let key_op = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP,
        "id": "req-cu-wayland-key",
        "action": "key",
        "key": "Return",
    });
    let key_ok_str = handle_computer_use_op(&key_op).await;
    let key_ok: Value = serde_json::from_str(&key_ok_str).expect("valid reply");
    assert_eq!(key_ok["success"], true);
    assert_eq!(key_ok["status"], "ok");
    let key_call = get_last_wayland_inputs()
        .into_iter()
        .find(|i| i.tool == "wtype" && i.action == "key");
    assert!(key_call.is_some(), "must invoke wtype key");
    assert!(key_call.unwrap().args.contains(&"Return".to_string()));

    // Cleanup
    set_test_wayland_display(None);
    set_test_wayland_cmd_handler(None);
    clear_test_wayland_tools();
    clear_last_wayland_inputs();
    clear_injected_events();
}
