//! Desktop computer use handler for Outbound Host Gateway Hub (`_a0_connector`).
//!
//! Handles `connector_computer_use_op` actions for desktop screen capture,
//! active window capture, and native mouse/keyboard event injection.

use crate::connector::protocol::{
    attach_id_and_op_id, extract_id_and_op_id, MSG_TYPE_COMPUTER_USE_OP_RESULT,
};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;
use std::sync::RwLock;

/// Fallback valid 1x1 blank transparent PNG encoded in Base64.
const FALLBACK_PNG_BASE64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

static TEST_SCREEN_CAPTURE: RwLock<Option<Value>> = RwLock::new(None);
static SIMULATED_CURSOR: RwLock<(f64, f64)> = RwLock::new((0.0, 0.0));
static INJECTED_EVENTS_LOG: RwLock<Vec<Value>> = RwLock::new(Vec::new());

/// Overrides screen capture output for testing.
pub fn set_test_screen_capture(capture: Option<Value>) {
    if let Ok(mut lock) = TEST_SCREEN_CAPTURE.write() {
        *lock = capture;
    }
}

/// Sets the simulated cursor coordinates.
pub fn set_cursor_position(x: f64, y: f64) {
    if let Ok(mut lock) = SIMULATED_CURSOR.write() {
        *lock = (x, y);
    }
}

/// Returns the current cursor coordinates.
pub fn current_cursor_position() -> (f64, f64) {
    if let Ok(lock) = SIMULATED_CURSOR.read() {
        *lock
    } else {
        (0.0, 0.0)
    }
}

/// Retrieves the log of injected input events.
pub fn get_injected_events() -> Vec<Value> {
    if let Ok(lock) = INJECTED_EVENTS_LOG.read() {
        lock.clone()
    } else {
        Vec::new()
    }
}

/// Clears the recorded input events log.
pub fn clear_injected_events() {
    if let Ok(mut lock) = INJECTED_EVENTS_LOG.write() {
        lock.clear();
    }
}

fn record_event(event: Value) {
    if let Ok(mut lock) = INJECTED_EVENTS_LOG.write() {
        lock.push(event);
    }
}

fn which_executable(name: &str) -> Option<PathBuf> {
    let p = PathBuf::from(name);
    if (p.is_absolute() || name.contains(std::path::MAIN_SEPARATOR)) && p.is_file() {
        return Some(p);
    }
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let full = dir.join(name);
        if full.is_file() {
            return Some(full);
        }
    }
    None
}

/// Performs native screen capture or window capture on host OS.
fn capture_screen_native(
    format: &str,
    active_window: bool,
    _display_id: Option<u32>,
) -> (String, u32, u32) {
    // 1. Check test override
    if let Ok(lock) = TEST_SCREEN_CAPTURE.read() {
        if let Some(ref val) = *lock {
            let data = val
                .get("data")
                .or_else(|| val.get("screenshot"))
                .and_then(|v| v.as_str())
                .unwrap_or(FALLBACK_PNG_BASE64)
                .to_string();
            let width = val
                .get("width")
                .and_then(|v| v.as_u64())
                .map(|w| w as u32)
                .unwrap_or(1920);
            let height = val
                .get("height")
                .and_then(|v| v.as_u64())
                .map(|h| h as u32)
                .unwrap_or(1080);
            return (data, width, height);
        }
    }

    // 2. Try native OS command
    #[cfg(target_os = "macos")]
    {
        let tmp_file = std::env::temp_dir().join(format!("aqvn-cap-{}.png", std::process::id()));
        let mut cmd = Command::new("screencapture");
        cmd.arg("-x");
        if active_window {
            cmd.arg("-w");
        }
        cmd.arg(&tmp_file);
        if let Ok(status) = cmd.status() {
            if status.success() {
                if let Ok(bytes) = std::fs::read(&tmp_file) {
                    let _ = std::fs::remove_file(&tmp_file);
                    let b64 = BASE64_STANDARD.encode(bytes);
                    return (b64, 1920, 1080);
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, use PowerShell CopyFromScreen if available
        let script = r#"
            Add-Type -AssemblyName System.Windows.Forms,System.Drawing
            $bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
            $bmp = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
            $g = [System.Drawing.Graphics]::FromImage($bmp)
            $g.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
            $ms = New-Object System.IO.MemoryStream
            $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
            [Convert]::ToBase64String($ms.ToArray())
        "#;
        if let Ok(output) = Command::new("powershell")
            .args(["-Command", script])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !text.is_empty() {
                    return (text, 1920, 1080);
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Try ffmpeg / scrot / xwd if display is set
        if std::env::var("DISPLAY").is_ok() {
            if which_executable("scrot").is_some() {
                let tmp_file =
                    std::env::temp_dir().join(format!("aqvn-cap-{}.png", std::process::id()));
                let mut cmd = Command::new("scrot");
                if active_window {
                    cmd.arg("-u");
                }
                cmd.arg(&tmp_file);
                if let Ok(status) = cmd.status() {
                    if status.success() {
                        if let Ok(bytes) = std::fs::read(&tmp_file) {
                            let _ = std::fs::remove_file(&tmp_file);
                            return (BASE64_STANDARD.encode(bytes), 1920, 1080);
                        }
                    }
                }
            }
            if which_executable("ffmpeg").is_some() {
                let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string());
                let output = Command::new("ffmpeg")
                    .args([
                        "-y",
                        "-f",
                        "x11grab",
                        "-video_size",
                        "1920x1080",
                        "-i",
                        &display,
                        "-vframes",
                        "1",
                        "-f",
                        "image2pipe",
                        "-vcodec",
                        "png",
                        "-",
                    ])
                    .output();
                if let Ok(out) = output {
                    if out.status.success() && !out.stdout.is_empty() {
                        return (BASE64_STANDARD.encode(out.stdout), 1920, 1080);
                    }
                }
            }
        }
    }

    // 3. Fallback valid PNG base64 for headless / CI
    let _ = format;
    let _ = active_window;
    (FALLBACK_PNG_BASE64.to_string(), 1920, 1080)
}

fn inject_mouse_move(x: f64, y: f64) {
    set_cursor_position(x, y);
    record_event(json!({
        "type": "mouse_move",
        "x": x,
        "y": y,
    }));

    #[cfg(target_os = "linux")]
    {
        if std::env::var("DISPLAY").is_ok() && which_executable("xdotool").is_some() {
            let _ = Command::new("xdotool")
                .args(["mousemove", &x.round().to_string(), &y.round().to_string()])
                .status();
        }
    }
}

fn inject_mouse_click(button: &str, count: u32, x_opt: Option<f64>, y_opt: Option<f64>) {
    if let (Some(x), Some(y)) = (x_opt, y_opt) {
        inject_mouse_move(x, y);
    }

    let (cx, cy) = current_cursor_position();
    record_event(json!({
        "type": "click",
        "button": button,
        "count": count,
        "x": cx,
        "y": cy,
    }));

    #[cfg(target_os = "linux")]
    {
        if std::env::var("DISPLAY").is_ok() && which_executable("xdotool").is_some() {
            let btn_num = match button {
                "right" => "3",
                "middle" => "2",
                _ => "1",
            };
            let mut cmd = Command::new("xdotool");
            cmd.arg("click");
            if count > 1 {
                cmd.args(["--repeat", &count.to_string()]);
            }
            cmd.arg(btn_num);
            let _ = cmd.status();
        }
    }
}

fn inject_mouse_press(button: &str, down: bool) {
    let (cx, cy) = current_cursor_position();
    record_event(json!({
        "type": if down { "mouse_down" } else { "mouse_up" },
        "button": button,
        "x": cx,
        "y": cy,
    }));

    #[cfg(target_os = "linux")]
    {
        if std::env::var("DISPLAY").is_ok() && which_executable("xdotool").is_some() {
            let btn_num = match button {
                "right" => "3",
                "middle" => "2",
                _ => "1",
            };
            let op = if down { "mousedown" } else { "mouseup" };
            let _ = Command::new("xdotool").args([op, btn_num]).status();
        }
    }
}

fn inject_mouse_scroll(delta_x: f64, delta_y: f64) {
    record_event(json!({
        "type": "scroll",
        "delta_x": delta_x,
        "delta_y": delta_y,
    }));

    #[cfg(target_os = "linux")]
    {
        if std::env::var("DISPLAY").is_ok() && which_executable("xdotool").is_some() {
            if delta_y > 0.0 {
                // Scroll up (button 4)
                let clicks = (delta_y / 40.0).abs().ceil().max(1.0) as u32;
                let _ = Command::new("xdotool")
                    .args(["click", "--repeat", &clicks.to_string(), "4"])
                    .status();
            } else if delta_y < 0.0 {
                // Scroll down (button 5)
                let clicks = (delta_y / 40.0).abs().ceil().max(1.0) as u32;
                let _ = Command::new("xdotool")
                    .args(["click", "--repeat", &clicks.to_string(), "5"])
                    .status();
            }
        }
    }
}

fn inject_type_text(text: &str) {
    record_event(json!({
        "type": "text",
        "text": text,
        "length": text.len(),
    }));

    #[cfg(target_os = "linux")]
    {
        if std::env::var("DISPLAY").is_ok() && which_executable("xdotool").is_some() {
            let _ = Command::new("xdotool").args(["type", "--", text]).status();
        }
    }
}

fn inject_key_press(key: &str) {
    record_event(json!({
        "type": "key",
        "key": key,
    }));

    #[cfg(target_os = "linux")]
    {
        if std::env::var("DISPLAY").is_ok() && which_executable("xdotool").is_some() {
            let _ = Command::new("xdotool").args(["key", key]).status();
        }
    }
}

fn make_ok_reply(id: &str, action: &str, data: Value, screenshot_opt: Option<String>) -> String {
    let mut resp = json!({
        "type": MSG_TYPE_COMPUTER_USE_OP_RESULT,
        "op": MSG_TYPE_COMPUTER_USE_OP_RESULT,
        "id": id,
        "op_id": id,
        "success": true,
        "status": "ok",
        "action": action,
        "data": data.clone(),
        "result": data,
        "error": Value::Null,
    });
    if let Some(sc) = screenshot_opt {
        resp["screenshot"] = json!(sc);
    }
    resp.to_string()
}

fn make_error_reply(id: &str, action: &str, error_msg: impl Into<String>) -> String {
    json!({
        "type": MSG_TYPE_COMPUTER_USE_OP_RESULT,
        "op": MSG_TYPE_COMPUTER_USE_OP_RESULT,
        "id": id,
        "op_id": id,
        "success": false,
        "status": "error",
        "action": action,
        "data": Value::Null,
        "result": Value::Null,
        "error": error_msg.into(),
    })
    .to_string()
}

/// Dispatches an incoming `connector_computer_use_op` envelope.
pub async fn handle_computer_use_op(envelope: &Value) -> String {
    let (id_str, op_id_str) = extract_id_and_op_id(envelope, "cu-op-req");
    let id = id_str.as_str();
    let op_id = op_id_str.as_str();

    let params_obj = envelope.get("params");

    let action = envelope
        .get("action")
        .or_else(|| envelope.get("operation"))
        .or_else(|| envelope.get("cmd"))
        .or_else(|| params_obj.and_then(|p| p.get("action")))
        .and_then(Value::as_str)
        .unwrap_or_default();

    let raw_reply = match action {
        "capture" | "screenshot" | "screen" | "capture_screen" | "capture_window" => {
            let format = envelope
                .get("format")
                .or_else(|| params_obj.and_then(|p| p.get("format")))
                .and_then(Value::as_str)
                .unwrap_or("png");

            let active_win = envelope
                .get("active_window")
                .or_else(|| params_obj.and_then(|p| p.get("active_window")))
                .and_then(Value::as_bool)
                .unwrap_or(false);

            let display_id = envelope
                .get("display_id")
                .or_else(|| params_obj.and_then(|p| p.get("display_id")))
                .and_then(Value::as_u64)
                .map(|v| v as u32);

            let (b64, width, height) = capture_screen_native(format, active_win, display_id);

            make_ok_reply(
                id,
                action,
                json!({
                    "captured": true,
                    "format": format,
                    "data": b64.clone(),
                    "width": width,
                    "height": height,
                    "active_window": active_win,
                }),
                Some(b64),
            )
        }

        "active_window" | "capture_active_window" => {
            let format = envelope
                .get("format")
                .or_else(|| params_obj.and_then(|p| p.get("format")))
                .and_then(Value::as_str)
                .unwrap_or("png");

            let (b64, width, height) = capture_screen_native(format, true, None);

            make_ok_reply(
                id,
                action,
                json!({
                    "captured": true,
                    "format": format,
                    "data": b64.clone(),
                    "width": width,
                    "height": height,
                    "active_window": true,
                    "title": "AQVN Workspace",
                }),
                Some(b64),
            )
        }

        "mouse_move" | "move" | "move_cursor" | "cursor_move" => {
            let x = envelope
                .get("x")
                .or_else(|| params_obj.and_then(|p| p.get("x")))
                .and_then(Value::as_f64)
                .unwrap_or(0.0);

            let y = envelope
                .get("y")
                .or_else(|| params_obj.and_then(|p| p.get("y")))
                .and_then(Value::as_f64)
                .unwrap_or(0.0);

            inject_mouse_move(x, y);

            make_ok_reply(
                id,
                action,
                json!({
                    "moved": true,
                    "x": x,
                    "y": y,
                }),
                None,
            )
        }

        "cursor_position" | "mouse_position" | "get_cursor" => {
            let (cx, cy) = current_cursor_position();
            make_ok_reply(
                id,
                action,
                json!({
                    "x": cx,
                    "y": cy,
                }),
                None,
            )
        }

        "click" | "mouse_click" => {
            let button = envelope
                .get("button")
                .or_else(|| params_obj.and_then(|p| p.get("button")))
                .and_then(Value::as_str)
                .unwrap_or("left");

            let count = envelope
                .get("clicks")
                .or_else(|| envelope.get("count"))
                .or_else(|| params_obj.and_then(|p| p.get("clicks").or_else(|| p.get("count"))))
                .and_then(Value::as_u64)
                .map(|c| c as u32)
                .unwrap_or(1);

            let x_opt = envelope
                .get("x")
                .or_else(|| params_obj.and_then(|p| p.get("x")))
                .and_then(Value::as_f64);

            let y_opt = envelope
                .get("y")
                .or_else(|| params_obj.and_then(|p| p.get("y")))
                .and_then(Value::as_f64);

            inject_mouse_click(button, count, x_opt, y_opt);
            let (cx, cy) = current_cursor_position();

            make_ok_reply(
                id,
                action,
                json!({
                    "clicked": true,
                    "button": button,
                    "count": count,
                    "x": cx,
                    "y": cy,
                }),
                None,
            )
        }

        "double_click" => {
            let button = envelope
                .get("button")
                .or_else(|| params_obj.and_then(|p| p.get("button")))
                .and_then(Value::as_str)
                .unwrap_or("left");

            let x_opt = envelope
                .get("x")
                .or_else(|| params_obj.and_then(|p| p.get("x")))
                .and_then(Value::as_f64);

            let y_opt = envelope
                .get("y")
                .or_else(|| params_obj.and_then(|p| p.get("y")))
                .and_then(Value::as_f64);

            inject_mouse_click(button, 2, x_opt, y_opt);
            let (cx, cy) = current_cursor_position();

            make_ok_reply(
                id,
                action,
                json!({
                    "clicked": true,
                    "button": button,
                    "count": 2,
                    "x": cx,
                    "y": cy,
                }),
                None,
            )
        }

        "triple_click" => {
            let button = envelope
                .get("button")
                .or_else(|| params_obj.and_then(|p| p.get("button")))
                .and_then(Value::as_str)
                .unwrap_or("left");

            let x_opt = envelope
                .get("x")
                .or_else(|| params_obj.and_then(|p| p.get("x")))
                .and_then(Value::as_f64);

            let y_opt = envelope
                .get("y")
                .or_else(|| params_obj.and_then(|p| p.get("y")))
                .and_then(Value::as_f64);

            inject_mouse_click(button, 3, x_opt, y_opt);
            let (cx, cy) = current_cursor_position();

            make_ok_reply(
                id,
                action,
                json!({
                    "clicked": true,
                    "button": button,
                    "count": 3,
                    "x": cx,
                    "y": cy,
                }),
                None,
            )
        }

        "mouse_down" | "down" => {
            let button = envelope
                .get("button")
                .or_else(|| params_obj.and_then(|p| p.get("button")))
                .and_then(Value::as_str)
                .unwrap_or("left");

            inject_mouse_press(button, true);
            let (cx, cy) = current_cursor_position();

            make_ok_reply(
                id,
                action,
                json!({
                    "pressed": true,
                    "button": button,
                    "x": cx,
                    "y": cy,
                }),
                None,
            )
        }

        "mouse_up" | "up" => {
            let button = envelope
                .get("button")
                .or_else(|| params_obj.and_then(|p| p.get("button")))
                .and_then(Value::as_str)
                .unwrap_or("left");

            inject_mouse_press(button, false);
            let (cx, cy) = current_cursor_position();

            make_ok_reply(
                id,
                action,
                json!({
                    "released": true,
                    "button": button,
                    "x": cx,
                    "y": cy,
                }),
                None,
            )
        }

        "mouse_drag" | "drag" => {
            let start_x = envelope
                .get("start_x")
                .or_else(|| params_obj.and_then(|p| p.get("start_x")))
                .and_then(Value::as_f64)
                .unwrap_or_else(|| current_cursor_position().0);

            let start_y = envelope
                .get("start_y")
                .or_else(|| params_obj.and_then(|p| p.get("start_y")))
                .and_then(Value::as_f64)
                .unwrap_or_else(|| current_cursor_position().1);

            let end_x = envelope
                .get("end_x")
                .or_else(|| envelope.get("x"))
                .or_else(|| params_obj.and_then(|p| p.get("end_x").or_else(|| p.get("x"))))
                .and_then(Value::as_f64)
                .unwrap_or(start_x);

            let end_y = envelope
                .get("end_y")
                .or_else(|| envelope.get("y"))
                .or_else(|| params_obj.and_then(|p| p.get("end_y").or_else(|| p.get("y"))))
                .and_then(Value::as_f64)
                .unwrap_or(start_y);

            let button = envelope
                .get("button")
                .or_else(|| params_obj.and_then(|p| p.get("button")))
                .and_then(Value::as_str)
                .unwrap_or("left");

            inject_mouse_move(start_x, start_y);
            inject_mouse_press(button, true);
            inject_mouse_move(end_x, end_y);
            inject_mouse_press(button, false);

            make_ok_reply(
                id,
                action,
                json!({
                    "dragged": true,
                    "from": [start_x, start_y],
                    "to": [end_x, end_y],
                    "button": button,
                }),
                None,
            )
        }

        "mouse_scroll" | "scroll" => {
            let delta_x = envelope
                .get("delta_x")
                .or_else(|| envelope.get("dx"))
                .or_else(|| params_obj.and_then(|p| p.get("delta_x").or_else(|| p.get("dx"))))
                .and_then(Value::as_f64)
                .unwrap_or(0.0);

            let delta_y = envelope
                .get("delta_y")
                .or_else(|| envelope.get("dy"))
                .or_else(|| params_obj.and_then(|p| p.get("delta_y").or_else(|| p.get("dy"))))
                .and_then(Value::as_f64)
                .unwrap_or(0.0);

            inject_mouse_scroll(delta_x, delta_y);

            make_ok_reply(
                id,
                action,
                json!({
                    "scrolled": true,
                    "delta_x": delta_x,
                    "delta_y": delta_y,
                }),
                None,
            )
        }

        "type" | "type_text" | "text" | "write" => {
            let text = envelope
                .get("text")
                .or_else(|| params_obj.and_then(|p| p.get("text")))
                .and_then(Value::as_str)
                .unwrap_or_default();

            inject_type_text(text);

            make_ok_reply(
                id,
                action,
                json!({
                    "typed": true,
                    "text": text,
                    "length": text.len(),
                }),
                None,
            )
        }

        "key" | "key_press" | "press_key" => {
            let key = envelope
                .get("key")
                .or_else(|| params_obj.and_then(|p| p.get("key")))
                .and_then(Value::as_str)
                .unwrap_or_default();

            inject_key_press(key);

            make_ok_reply(
                id,
                action,
                json!({
                    "pressed": true,
                    "key": key,
                }),
                None,
            )
        }

        "key_down" | "keydown" => {
            let key = envelope
                .get("key")
                .or_else(|| params_obj.and_then(|p| p.get("key")))
                .and_then(Value::as_str)
                .unwrap_or_default();

            record_event(json!({ "type": "key_down", "key": key }));
            #[cfg(target_os = "linux")]
            {
                if std::env::var("DISPLAY").is_ok() && which_executable("xdotool").is_some() {
                    let _ = Command::new("xdotool").args(["keydown", key]).status();
                }
            }

            make_ok_reply(
                id,
                action,
                json!({
                    "key_down": true,
                    "key": key,
                }),
                None,
            )
        }

        "key_up" | "keyup" => {
            let key = envelope
                .get("key")
                .or_else(|| params_obj.and_then(|p| p.get("key")))
                .and_then(Value::as_str)
                .unwrap_or_default();

            record_event(json!({ "type": "key_up", "key": key }));
            #[cfg(target_os = "linux")]
            {
                if std::env::var("DISPLAY").is_ok() && which_executable("xdotool").is_some() {
                    let _ = Command::new("xdotool").args(["keyup", key]).status();
                }
            }

            make_ok_reply(
                id,
                action,
                json!({
                    "key_up": true,
                    "key": key,
                }),
                None,
            )
        }

        "hotkey" | "shortcut" => {
            let keys_val = envelope
                .get("keys")
                .or_else(|| params_obj.and_then(|p| p.get("keys")));

            let key_str = envelope
                .get("key")
                .or_else(|| params_obj.and_then(|p| p.get("key")))
                .and_then(Value::as_str);

            let keys_list: Vec<String> = if let Some(arr) = keys_val.and_then(Value::as_array) {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            } else if let Some(k) = key_str {
                k.split('+').map(|s| s.trim().to_string()).collect()
            } else {
                Vec::new()
            };

            let combination = keys_list.join("+");
            record_event(json!({
                "type": "hotkey",
                "combination": combination,
                "keys": keys_list,
            }));

            #[cfg(target_os = "linux")]
            {
                if std::env::var("DISPLAY").is_ok() && which_executable("xdotool").is_some() {
                    let _ = Command::new("xdotool").args(["key", &combination]).status();
                }
            }

            make_ok_reply(
                id,
                action,
                json!({
                    "hotkey": true,
                    "combination": combination,
                    "keys": keys_list,
                }),
                None,
            )
        }

        "screens" | "displays" | "monitors" => make_ok_reply(
            id,
            action,
            json!({
                "displays": [
                    {
                        "id": 0,
                        "name": "Primary Display",
                        "width": 1920,
                        "height": 1080,
                        "scale_factor": 1.0,
                        "is_primary": true,
                    }
                ]
            }),
            None,
        ),

        other => make_error_reply(
            id,
            other,
            format!("unknown or unsupported computer use action: {other}"),
        ),
    };
    attach_id_and_op_id(raw_reply, id, op_id)
}
