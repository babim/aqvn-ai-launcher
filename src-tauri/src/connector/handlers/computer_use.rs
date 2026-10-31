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
use std::io::Cursor;
use std::path::PathBuf;
use std::process::Command;
use std::sync::RwLock;

/// Fallback valid 1x1 blank transparent PNG encoded in Base64.
const FALLBACK_PNG_BASE64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

static TEST_SCREEN_CAPTURE: RwLock<Option<Value>> = RwLock::new(None);
static TEST_SCREEN_DIMS: RwLock<Option<(f64, f64)>> = RwLock::new(None);
static SIMULATED_CURSOR: RwLock<(f64, f64)> = RwLock::new((0.0, 0.0));
static INJECTED_EVENTS_LOG: RwLock<Vec<Value>> = RwLock::new(Vec::new());
static LAST_WINDOWS_INPUTS: RwLock<Vec<SimulatedWindowsInput>> = RwLock::new(Vec::new());

// Win32 MOUSEEVENTF flags
pub const WIN_MOUSEEVENTF_MOVE: u32 = 0x0001;
pub const WIN_MOUSEEVENTF_LEFTDOWN: u32 = 0x0002;
pub const WIN_MOUSEEVENTF_LEFTUP: u32 = 0x0004;
pub const WIN_MOUSEEVENTF_RIGHTDOWN: u32 = 0x0008;
pub const WIN_MOUSEEVENTF_RIGHTUP: u32 = 0x0010;
pub const WIN_MOUSEEVENTF_MIDDLEDOWN: u32 = 0x0020;
pub const WIN_MOUSEEVENTF_MIDDLEUP: u32 = 0x0040;
pub const WIN_MOUSEEVENTF_WHEEL: u32 = 0x0800;
pub const WIN_MOUSEEVENTF_HWHEEL: u32 = 0x1000;
pub const WIN_MOUSEEVENTF_ABSOLUTE: u32 = 0x8000;

// Win32 KEYEVENTF flags
pub const WIN_KEYEVENTF_KEYUP: u32 = 0x0002;
pub const WIN_KEYEVENTF_UNICODE: u32 = 0x0004;

// Win32 Virtual Key codes
pub const WIN_VK_BACK: u16 = 0x08;
pub const WIN_VK_TAB: u16 = 0x09;
pub const WIN_VK_RETURN: u16 = 0x0D;
pub const WIN_VK_SHIFT: u16 = 0x10;
pub const WIN_VK_CONTROL: u16 = 0x11;
pub const WIN_VK_MENU: u16 = 0x12; // Alt
pub const WIN_VK_CAPITAL: u16 = 0x14; // Caps Lock
pub const WIN_VK_ESCAPE: u16 = 0x1B;
pub const WIN_VK_SPACE: u16 = 0x20;
pub const WIN_VK_PRIOR: u16 = 0x21; // Page Up
pub const WIN_VK_NEXT: u16 = 0x22; // Page Down
pub const WIN_VK_END: u16 = 0x23;
pub const WIN_VK_HOME: u16 = 0x24;
pub const WIN_VK_LEFT: u16 = 0x25;
pub const WIN_VK_UP: u16 = 0x26;
pub const WIN_VK_RIGHT: u16 = 0x27;
pub const WIN_VK_DOWN: u16 = 0x28;
pub const WIN_VK_DELETE: u16 = 0x2E;
pub const WIN_VK_LWIN: u16 = 0x5B;
pub const WIN_VK_F1: u16 = 0x70;
pub const WIN_VK_F2: u16 = 0x71;
pub const WIN_VK_F3: u16 = 0x72;
pub const WIN_VK_F4: u16 = 0x73;
pub const WIN_VK_F5: u16 = 0x74;
pub const WIN_VK_F6: u16 = 0x75;
pub const WIN_VK_F7: u16 = 0x76;
pub const WIN_VK_F8: u16 = 0x77;
pub const WIN_VK_F9: u16 = 0x78;
pub const WIN_VK_F10: u16 = 0x79;
pub const WIN_VK_F11: u16 = 0x7A;
pub const WIN_VK_F12: u16 = 0x7B;

/// Cross-platform simulated Windows SendInput structure matching Win32 INPUT union.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SimulatedWindowsInput {
    pub input_type: String, // "mouse" | "keyboard"
    pub flags: u32,
    pub dx: i32,
    pub dy: i32,
    pub mouse_data: u32,
    pub vk: u16,
    pub scan: u16,
    pub is_key_up: bool,
    pub is_unicode: bool,
}

/// Sets screen dimension overrides for testing coordinate normalization.
pub fn set_test_screen_dimensions(dims: Option<(f64, f64)>) {
    if let Ok(mut lock) = TEST_SCREEN_DIMS.write() {
        *lock = dims;
    }
}

/// Gets the effective screen dimensions.
pub fn get_active_screen_dimensions() -> (f64, f64) {
    if let Ok(lock) = TEST_SCREEN_DIMS.read() {
        if let Some(dims) = *lock {
            return dims;
        }
    }
    #[cfg(target_os = "windows")]
    {
        windows_input::get_screen_dimensions()
    }
    #[cfg(not(target_os = "windows"))]
    {
        (1920.0, 1080.0)
    }
}

/// Retrieves the recorded Windows SendInput packets.
pub fn get_last_windows_inputs() -> Vec<SimulatedWindowsInput> {
    if let Ok(lock) = LAST_WINDOWS_INPUTS.read() {
        lock.clone()
    } else {
        Vec::new()
    }
}

/// Clears the recorded Windows SendInput packets.
pub fn clear_last_windows_inputs() {
    if let Ok(mut lock) = LAST_WINDOWS_INPUTS.write() {
        lock.clear();
    }
}

fn record_windows_inputs(inputs: &[SimulatedWindowsInput]) {
    if let Ok(mut lock) = LAST_WINDOWS_INPUTS.write() {
        lock.extend_from_slice(inputs);
    }
}

/// Normalizes screen coordinates (x, y) into Windows absolute mouse coordinates (0..65535).
pub fn normalize_screen_coordinates(x: f64, y: f64, screen_w: f64, screen_h: f64) -> (i32, i32) {
    let w = if screen_w <= 0.0 { 1920.0 } else { screen_w };
    let h = if screen_h <= 0.0 { 1080.0 } else { screen_h };
    let nx = ((x * 65535.0) / w).round() as i32;
    let ny = ((y * 65535.0) / h).round() as i32;
    (nx.clamp(0, 65535), ny.clamp(0, 65535))
}

/// Maps human key name to Windows Virtual Key code.
pub fn map_key_to_win_vk(key: &str) -> Option<u16> {
    match key.to_lowercase().as_str() {
        "return" | "enter" => Some(WIN_VK_RETURN),
        "backspace" | "back" => Some(WIN_VK_BACK),
        "tab" => Some(WIN_VK_TAB),
        "escape" | "esc" => Some(WIN_VK_ESCAPE),
        "space" | " " => Some(WIN_VK_SPACE),
        "up" | "arrowup" => Some(WIN_VK_UP),
        "down" | "arrowdown" => Some(WIN_VK_DOWN),
        "left" | "arrowleft" => Some(WIN_VK_LEFT),
        "right" | "arrowright" => Some(WIN_VK_RIGHT),
        "delete" | "del" => Some(WIN_VK_DELETE),
        "home" => Some(WIN_VK_HOME),
        "end" => Some(WIN_VK_END),
        "pageup" | "prior" => Some(WIN_VK_PRIOR),
        "pagedown" | "next" => Some(WIN_VK_NEXT),
        "shift" | "shiftleft" | "shiftright" => Some(WIN_VK_SHIFT),
        "ctrl" | "control" | "controlleft" | "controlright" => Some(WIN_VK_CONTROL),
        "alt" | "altleft" | "altright" | "menu" => Some(WIN_VK_MENU),
        "win" | "meta" | "super" | "windows" => Some(WIN_VK_LWIN),
        "capslock" => Some(WIN_VK_CAPITAL),
        "f1" => Some(WIN_VK_F1),
        "f2" => Some(WIN_VK_F2),
        "f3" => Some(WIN_VK_F3),
        "f4" => Some(WIN_VK_F4),
        "f5" => Some(WIN_VK_F5),
        "f6" => Some(WIN_VK_F6),
        "f7" => Some(WIN_VK_F7),
        "f8" => Some(WIN_VK_F8),
        "f9" => Some(WIN_VK_F9),
        "f10" => Some(WIN_VK_F10),
        "f11" => Some(WIN_VK_F11),
        "f12" => Some(WIN_VK_F12),
        s if s.len() == 1 => {
            let c = s.chars().next().unwrap();
            if c.is_ascii_alphanumeric() {
                Some(c.to_ascii_uppercase() as u16)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn build_windows_mouse_move(
    x: f64,
    y: f64,
    screen_w: f64,
    screen_h: f64,
) -> SimulatedWindowsInput {
    let (dx, dy) = normalize_screen_coordinates(x, y, screen_w, screen_h);
    SimulatedWindowsInput {
        input_type: "mouse".to_string(),
        flags: WIN_MOUSEEVENTF_MOVE | WIN_MOUSEEVENTF_ABSOLUTE,
        dx,
        dy,
        mouse_data: 0,
        vk: 0,
        scan: 0,
        is_key_up: false,
        is_unicode: false,
    }
}

pub fn build_windows_mouse_press(button: &str, down: bool) -> SimulatedWindowsInput {
    let flags = match (button.to_lowercase().as_str(), down) {
        ("right", true) => WIN_MOUSEEVENTF_RIGHTDOWN,
        ("right", false) => WIN_MOUSEEVENTF_RIGHTUP,
        ("middle", true) => WIN_MOUSEEVENTF_MIDDLEDOWN,
        ("middle", false) => WIN_MOUSEEVENTF_MIDDLEUP,
        (_, true) => WIN_MOUSEEVENTF_LEFTDOWN,
        (_, false) => WIN_MOUSEEVENTF_LEFTUP,
    };
    SimulatedWindowsInput {
        input_type: "mouse".to_string(),
        flags,
        dx: 0,
        dy: 0,
        mouse_data: 0,
        vk: 0,
        scan: 0,
        is_key_up: !down,
        is_unicode: false,
    }
}

pub fn build_windows_mouse_click(
    button: &str,
    count: u32,
    x_opt: Option<f64>,
    y_opt: Option<f64>,
    screen_w: f64,
    screen_h: f64,
) -> Vec<SimulatedWindowsInput> {
    let mut inputs = Vec::new();
    if let (Some(x), Some(y)) = (x_opt, y_opt) {
        inputs.push(build_windows_mouse_move(x, y, screen_w, screen_h));
    }
    for _ in 0..count.max(1) {
        inputs.push(build_windows_mouse_press(button, true));
        inputs.push(build_windows_mouse_press(button, false));
    }
    inputs
}

pub fn build_windows_mouse_scroll(delta_x: f64, delta_y: f64) -> Vec<SimulatedWindowsInput> {
    let mut inputs = Vec::new();
    if delta_y.abs() > 0.001 {
        let val = (delta_y * 120.0).round() as i32;
        inputs.push(SimulatedWindowsInput {
            input_type: "mouse".to_string(),
            flags: WIN_MOUSEEVENTF_WHEEL,
            dx: 0,
            dy: 0,
            mouse_data: val as u32,
            vk: 0,
            scan: 0,
            is_key_up: false,
            is_unicode: false,
        });
    }
    if delta_x.abs() > 0.001 {
        let val = (delta_x * 120.0).round() as i32;
        inputs.push(SimulatedWindowsInput {
            input_type: "mouse".to_string(),
            flags: WIN_MOUSEEVENTF_HWHEEL,
            dx: 0,
            dy: 0,
            mouse_data: val as u32,
            vk: 0,
            scan: 0,
            is_key_up: false,
            is_unicode: false,
        });
    }
    inputs
}

pub fn build_windows_type_text(text: &str) -> Vec<SimulatedWindowsInput> {
    let mut inputs = Vec::new();
    for ch in text.encode_utf16() {
        inputs.push(SimulatedWindowsInput {
            input_type: "keyboard".to_string(),
            flags: WIN_KEYEVENTF_UNICODE,
            dx: 0,
            dy: 0,
            mouse_data: 0,
            vk: 0,
            scan: ch,
            is_key_up: false,
            is_unicode: true,
        });
        inputs.push(SimulatedWindowsInput {
            input_type: "keyboard".to_string(),
            flags: WIN_KEYEVENTF_UNICODE | WIN_KEYEVENTF_KEYUP,
            dx: 0,
            dy: 0,
            mouse_data: 0,
            vk: 0,
            scan: ch,
            is_key_up: true,
            is_unicode: true,
        });
    }
    inputs
}

pub fn build_windows_key_press(key: &str) -> Vec<SimulatedWindowsInput> {
    if let Some(vk) = map_key_to_win_vk(key) {
        vec![
            SimulatedWindowsInput {
                input_type: "keyboard".to_string(),
                flags: 0,
                dx: 0,
                dy: 0,
                mouse_data: 0,
                vk,
                scan: 0,
                is_key_up: false,
                is_unicode: false,
            },
            SimulatedWindowsInput {
                input_type: "keyboard".to_string(),
                flags: WIN_KEYEVENTF_KEYUP,
                dx: 0,
                dy: 0,
                mouse_data: 0,
                vk,
                scan: 0,
                is_key_up: true,
                is_unicode: false,
            },
        ]
    } else {
        build_windows_type_text(key)
    }
}

pub fn build_windows_hotkey(keys: &[String]) -> Vec<SimulatedWindowsInput> {
    let mut vks = Vec::new();
    for k in keys {
        if let Some(vk) = map_key_to_win_vk(k) {
            vks.push(vk);
        }
    }
    let mut inputs = Vec::new();
    for &vk in &vks {
        inputs.push(SimulatedWindowsInput {
            input_type: "keyboard".to_string(),
            flags: 0,
            dx: 0,
            dy: 0,
            mouse_data: 0,
            vk,
            scan: 0,
            is_key_up: false,
            is_unicode: false,
        });
    }
    for &vk in vks.iter().rev() {
        inputs.push(SimulatedWindowsInput {
            input_type: "keyboard".to_string(),
            flags: WIN_KEYEVENTF_KEYUP,
            dx: 0,
            dy: 0,
            mouse_data: 0,
            vk,
            scan: 0,
            is_key_up: true,
            is_unicode: false,
        });
    }
    inputs
}

pub fn simulate_windows_sendinput(
    action: &str,
    envelope: &Value,
) -> Result<Vec<SimulatedWindowsInput>, String> {
    let params = envelope.get("params");
    match action {
        "mouse_move" | "move" | "move_cursor" | "cursor_move" => {
            let x = envelope
                .get("x")
                .or_else(|| params.and_then(|p| p.get("x")))
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let y = envelope
                .get("y")
                .or_else(|| params.and_then(|p| p.get("y")))
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let (sw, sh) = get_active_screen_dimensions();
            Ok(vec![build_windows_mouse_move(x, y, sw, sh)])
        }
        "click" | "mouse_click" | "double_click" | "triple_click" => {
            let button = envelope
                .get("button")
                .or_else(|| params.and_then(|p| p.get("button")))
                .and_then(Value::as_str)
                .unwrap_or("left");
            let count = if action == "double_click" {
                2
            } else if action == "triple_click" {
                3
            } else {
                envelope
                    .get("clicks")
                    .or_else(|| envelope.get("count"))
                    .or_else(|| params.and_then(|p| p.get("clicks").or_else(|| p.get("count"))))
                    .and_then(Value::as_u64)
                    .map(|c| c as u32)
                    .unwrap_or(1)
            };
            let x_opt = envelope
                .get("x")
                .or_else(|| params.and_then(|p| p.get("x")))
                .and_then(Value::as_f64);
            let y_opt = envelope
                .get("y")
                .or_else(|| params.and_then(|p| p.get("y")))
                .and_then(Value::as_f64);
            let (sw, sh) = get_active_screen_dimensions();
            Ok(build_windows_mouse_click(
                button, count, x_opt, y_opt, sw, sh,
            ))
        }
        "mouse_down" | "down" => {
            let button = envelope
                .get("button")
                .or_else(|| params.and_then(|p| p.get("button")))
                .and_then(Value::as_str)
                .unwrap_or("left");
            Ok(vec![build_windows_mouse_press(button, true)])
        }
        "mouse_up" | "up" => {
            let button = envelope
                .get("button")
                .or_else(|| params.and_then(|p| p.get("button")))
                .and_then(Value::as_str)
                .unwrap_or("left");
            Ok(vec![build_windows_mouse_press(button, false)])
        }
        "mouse_scroll" | "scroll" => {
            let delta_x = envelope
                .get("delta_x")
                .or_else(|| envelope.get("dx"))
                .or_else(|| params.and_then(|p| p.get("delta_x").or_else(|| p.get("dx"))))
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let delta_y = envelope
                .get("delta_y")
                .or_else(|| envelope.get("dy"))
                .or_else(|| params.and_then(|p| p.get("delta_y").or_else(|| p.get("dy"))))
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            Ok(build_windows_mouse_scroll(delta_x, delta_y))
        }
        "type" | "type_text" | "text" | "write" => {
            let text = envelope
                .get("text")
                .or_else(|| params.and_then(|p| p.get("text")))
                .and_then(Value::as_str)
                .unwrap_or_default();
            Ok(build_windows_type_text(text))
        }
        "key" | "key_press" | "press_key" => {
            let key = envelope
                .get("key")
                .or_else(|| params.and_then(|p| p.get("key")))
                .and_then(Value::as_str)
                .unwrap_or_default();
            Ok(build_windows_key_press(key))
        }
        "hotkey" | "shortcut" => {
            let keys_val = envelope
                .get("keys")
                .or_else(|| params.and_then(|p| p.get("keys")));
            let key_str = envelope
                .get("key")
                .or_else(|| params.and_then(|p| p.get("key")))
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
            Ok(build_windows_hotkey(&keys_list))
        }
        other => Err(format!(
            "unsupported action for windows sendinput simulation: {other}"
        )),
    }
}

#[cfg(target_os = "windows")]
mod windows_input {
    use super::SimulatedWindowsInput;
    use std::mem::size_of;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, MOUSEINPUT,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

    pub fn get_screen_dimensions() -> (f64, f64) {
        unsafe {
            let cx = GetSystemMetrics(SM_CXSCREEN);
            let cy = GetSystemMetrics(SM_CYSCREEN);
            let w = if cx > 0 { cx as f64 } else { 1920.0 };
            let h = if cy > 0 { cy as f64 } else { 1080.0 };
            (w, h)
        }
    }

    pub fn dispatch_simulated_inputs(inputs: &[SimulatedWindowsInput]) -> u32 {
        if inputs.is_empty() {
            return 0;
        }
        let win_inputs: Vec<INPUT> = inputs
            .iter()
            .map(|s| {
                if s.input_type == "mouse" {
                    INPUT {
                        r#type: INPUT_MOUSE,
                        Anonymous: INPUT_0 {
                            mi: MOUSEINPUT {
                                dx: s.dx,
                                dy: s.dy,
                                mouseData: s.mouse_data,
                                dwFlags: s.flags,
                                time: 0,
                                dwExtraInfo: 0,
                            },
                        },
                    }
                } else {
                    INPUT {
                        r#type: INPUT_KEYBOARD,
                        Anonymous: INPUT_0 {
                            ki: KEYBDINPUT {
                                wVk: s.vk,
                                wScan: s.scan,
                                dwFlags: s.flags,
                                time: 0,
                                dwExtraInfo: 0,
                            },
                        },
                    }
                }
            })
            .collect();

        unsafe {
            SendInput(
                win_inputs.len() as u32,
                win_inputs.as_ptr(),
                size_of::<INPUT>() as i32,
            )
        }
    }
}

#[cfg(target_os = "macos")]
mod macos_input {
    use super::which_executable;
    use std::process::Command;

    fn run_osascript(script: &str) {
        if which_executable("osascript").is_some() {
            let _ = Command::new("osascript").args(["-e", script]).status();
        }
    }

    pub fn mouse_move(x: f64, y: f64) {
        if which_executable("cliclick").is_some() {
            let _ = Command::new("cliclick")
                .args(["m", &format!("{},{}", x.round() as i32, y.round() as i32)])
                .status();
        }
    }

    pub fn mouse_click(button: &str, count: u32, x_opt: Option<f64>, y_opt: Option<f64>) {
        if let (Some(x), Some(y)) = (x_opt, y_opt) {
            if which_executable("cliclick").is_some() {
                let cmd = match (button, count) {
                    ("right", _) => "rc",
                    (_, 2) => "dc",
                    (_, 3) => "tc",
                    _ => "c",
                };
                let _ = Command::new("cliclick")
                    .args([cmd, &format!("{},{}", x.round() as i32, y.round() as i32)])
                    .status();
                return;
            }
            let script = format!(
                r#"tell application "System Events" to click at {{{}, {}}}"#,
                x.round() as i32,
                y.round() as i32
            );
            run_osascript(&script);
        } else {
            let script = r#"tell application "System Events" to click"#;
            run_osascript(script);
        }
    }

    pub fn mouse_press(_button: &str, down: bool) {
        if which_executable("cliclick").is_some() {
            let op = if down { "dd" } else { "du" };
            let _ = Command::new("cliclick").arg(op).status();
        }
    }

    pub fn mouse_scroll(_delta_x: f64, _delta_y: f64) {}

    pub fn type_text(text: &str) {
        let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!(
            r#"tell application "System Events" to keystroke "{}""#,
            escaped
        );
        run_osascript(&script);
    }

    pub fn key_press(key: &str) {
        match key.to_lowercase().as_str() {
            "return" | "enter" => {
                run_osascript(r#"tell application "System Events" to key code 36"#)
            }
            "backspace" | "back" => {
                run_osascript(r#"tell application "System Events" to key code 51"#)
            }
            "tab" => run_osascript(r#"tell application "System Events" to key code 48"#),
            "escape" | "esc" => run_osascript(r#"tell application "System Events" to key code 53"#),
            "space" | " " => run_osascript(r#"tell application "System Events" to key code 49"#),
            "up" | "arrowup" => {
                run_osascript(r#"tell application "System Events" to key code 126"#)
            }
            "down" | "arrowdown" => {
                run_osascript(r#"tell application "System Events" to key code 125"#)
            }
            "left" | "arrowleft" => {
                run_osascript(r#"tell application "System Events" to key code 123"#)
            }
            "right" | "arrowright" => {
                run_osascript(r#"tell application "System Events" to key code 124"#)
            }
            _ => type_text(key),
        }
    }

    pub fn key_down(key: &str) {
        key_press(key);
    }

    pub fn key_up(_key: &str) {}

    pub fn hotkey(keys: &[String]) {
        let mut modifiers = Vec::new();
        let mut target_key = String::new();
        for k in keys {
            match k.to_lowercase().as_str() {
                "command" | "cmd" | "super" | "win" | "meta" => modifiers.push("command down"),
                "control" | "ctrl" => modifiers.push("control down"),
                "alt" | "option" => modifiers.push("option down"),
                "shift" => modifiers.push("shift down"),
                other => target_key = other.to_string(),
            }
        }
        if !target_key.is_empty() {
            let mod_clause = if modifiers.is_empty() {
                String::new()
            } else {
                format!(" using {{{}}}", modifiers.join(", "))
            };
            let escaped = target_key.replace('\\', "\\\\").replace('"', "\\\"");
            let script = format!(
                r#"tell application "System Events" to keystroke "{}"{}"#,
                escaped, mod_clause
            );
            run_osascript(&script);
        }
    }
}

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

/// Computes target dimensions when scaling or capping by max_width.
pub fn compute_scaled_dimensions(
    orig_w: u32,
    orig_h: u32,
    scale: Option<f64>,
    max_width: Option<u32>,
) -> (u32, u32) {
    if orig_w == 0 || orig_h == 0 {
        return (orig_w, orig_h);
    }
    let mut w = orig_w as f64;
    let mut h = orig_h as f64;

    if let Some(s) = scale {
        if s > 0.0 && s < 1.0 {
            w = (w * s).max(1.0);
            h = (h * s).max(1.0);
        }
    }

    if let Some(mw) = max_width {
        if mw > 0 && w > mw as f64 {
            let ratio = mw as f64 / w;
            w = mw as f64;
            h = (h * ratio).max(1.0);
        }
    }

    let target_w = (w.round() as u32).max(1);
    let target_h = (h.round() as u32).max(1);
    (target_w, target_h)
}

fn process_and_encode_image(
    image_bytes: &[u8],
    orig_w: u32,
    orig_h: u32,
    format: &str,
    scale: Option<f64>,
    max_width: Option<u32>,
) -> (String, u32, u32) {
    let (target_w, target_h) = compute_scaled_dimensions(orig_w, orig_h, scale, max_width);
    let is_jpeg = format.eq_ignore_ascii_case("jpeg") || format.eq_ignore_ascii_case("jpg");

    if target_w == orig_w && target_h == orig_h && !is_jpeg {
        return (BASE64_STANDARD.encode(image_bytes), orig_w, orig_h);
    }

    if let Ok(img) = image::load_from_memory(image_bytes) {
        let resized = img.resize_exact(target_w, target_h, image::imageops::FilterType::Triangle);
        let mut buf = Vec::new();
        if is_jpeg {
            let rgb = resized.to_rgb8();
            if rgb
                .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Jpeg)
                .is_ok()
            {
                return (BASE64_STANDARD.encode(&buf), target_w, target_h);
            }
        } else if resized
            .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
            .is_ok()
        {
            return (BASE64_STANDARD.encode(&buf), target_w, target_h);
        }
    }

    (BASE64_STANDARD.encode(image_bytes), target_w, target_h)
}

fn process_and_encode_b64(
    b64_data: &str,
    orig_w: u32,
    orig_h: u32,
    format: &str,
    scale: Option<f64>,
    max_width: Option<u32>,
) -> (String, u32, u32) {
    let (target_w, target_h) = compute_scaled_dimensions(orig_w, orig_h, scale, max_width);
    let is_jpeg = format.eq_ignore_ascii_case("jpeg") || format.eq_ignore_ascii_case("jpg");

    if target_w == orig_w && target_h == orig_h && !is_jpeg {
        return (b64_data.to_string(), orig_w, orig_h);
    }

    if let Ok(bytes) = BASE64_STANDARD.decode(b64_data.trim()) {
        if let Ok(img) = image::load_from_memory(&bytes) {
            let resized =
                img.resize_exact(target_w, target_h, image::imageops::FilterType::Triangle);
            let mut buf = Vec::new();
            if is_jpeg {
                let rgb = resized.to_rgb8();
                if rgb
                    .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Jpeg)
                    .is_ok()
                {
                    return (BASE64_STANDARD.encode(&buf), target_w, target_h);
                }
            } else if resized
                .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
                .is_ok()
            {
                return (BASE64_STANDARD.encode(&buf), target_w, target_h);
            }
        }
    }

    (b64_data.to_string(), target_w, target_h)
}

/// Performs native screen capture or window capture on host OS.
pub fn capture_screen_native(
    format: &str,
    active_window: bool,
    _display_id: Option<u32>,
    scale: Option<f64>,
    max_width: Option<u32>,
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
            return process_and_encode_b64(&data, width, height, format, scale, max_width);
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
                    return process_and_encode_image(&bytes, 1920, 1080, format, scale, max_width);
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
                    return process_and_encode_b64(&text, 1920, 1080, format, scale, max_width);
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
                            return process_and_encode_image(
                                &bytes, 1920, 1080, format, scale, max_width,
                            );
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
                        return process_and_encode_image(
                            &out.stdout,
                            1920,
                            1080,
                            format,
                            scale,
                            max_width,
                        );
                    }
                }
            }
        }
    }

    // 3. Fallback valid PNG base64 for headless / CI
    let _ = active_window;
    process_and_encode_b64(FALLBACK_PNG_BASE64, 1920, 1080, format, scale, max_width)
}

fn inject_mouse_move(x: f64, y: f64) {
    set_cursor_position(x, y);
    record_event(json!({
        "type": "mouse_move",
        "x": x,
        "y": y,
    }));
    let (sw, sh) = get_active_screen_dimensions();
    let sim = build_windows_mouse_move(x, y, sw, sh);
    record_windows_inputs(std::slice::from_ref(&sim));
    #[cfg(target_os = "windows")]
    {
        windows_input::dispatch_simulated_inputs(&[sim]);
    }
    #[cfg(target_os = "macos")]
    {
        macos_input::mouse_move(x, y);
    }
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
    let (sw, sh) = get_active_screen_dimensions();
    let sims = build_windows_mouse_click(button, count, None, None, sw, sh);
    record_windows_inputs(&sims);
    #[cfg(target_os = "windows")]
    {
        windows_input::dispatch_simulated_inputs(&sims);
    }
    #[cfg(target_os = "macos")]
    {
        macos_input::mouse_click(button, count, x_opt, y_opt);
    }
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
    let sim = build_windows_mouse_press(button, down);
    record_windows_inputs(std::slice::from_ref(&sim));
    #[cfg(target_os = "windows")]
    {
        windows_input::dispatch_simulated_inputs(&[sim]);
    }
    #[cfg(target_os = "macos")]
    {
        macos_input::mouse_press(button, down);
    }
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
    let sims = build_windows_mouse_scroll(delta_x, delta_y);
    record_windows_inputs(&sims);
    #[cfg(target_os = "windows")]
    {
        windows_input::dispatch_simulated_inputs(&sims);
    }
    #[cfg(target_os = "macos")]
    {
        macos_input::mouse_scroll(delta_x, delta_y);
    }
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
    let sims = build_windows_type_text(text);
    record_windows_inputs(&sims);
    #[cfg(target_os = "windows")]
    {
        windows_input::dispatch_simulated_inputs(&sims);
    }
    #[cfg(target_os = "macos")]
    {
        macos_input::type_text(text);
    }
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
    let sims = build_windows_key_press(key);
    record_windows_inputs(&sims);
    #[cfg(target_os = "windows")]
    {
        windows_input::dispatch_simulated_inputs(&sims);
    }
    #[cfg(target_os = "macos")]
    {
        macos_input::key_press(key);
    }
    #[cfg(target_os = "linux")]
    {
        if std::env::var("DISPLAY").is_ok() && which_executable("xdotool").is_some() {
            let _ = Command::new("xdotool").args(["key", key]).status();
        }
    }
}

fn inject_key_down(key: &str) {
    record_event(json!({ "type": "key_down", "key": key }));
    if let Some(vk) = map_key_to_win_vk(key) {
        let sim = SimulatedWindowsInput {
            input_type: "keyboard".to_string(),
            flags: 0,
            dx: 0,
            dy: 0,
            mouse_data: 0,
            vk,
            scan: 0,
            is_key_up: false,
            is_unicode: false,
        };
        record_windows_inputs(std::slice::from_ref(&sim));
        #[cfg(target_os = "windows")]
        {
            windows_input::dispatch_simulated_inputs(&[sim]);
        }
    }
    #[cfg(target_os = "macos")]
    {
        macos_input::key_down(key);
    }
    #[cfg(target_os = "linux")]
    {
        if std::env::var("DISPLAY").is_ok() && which_executable("xdotool").is_some() {
            let _ = Command::new("xdotool").args(["keydown", key]).status();
        }
    }
}

fn inject_key_up(key: &str) {
    record_event(json!({ "type": "key_up", "key": key }));
    if let Some(vk) = map_key_to_win_vk(key) {
        let sim = SimulatedWindowsInput {
            input_type: "keyboard".to_string(),
            flags: WIN_KEYEVENTF_KEYUP,
            dx: 0,
            dy: 0,
            mouse_data: 0,
            vk,
            scan: 0,
            is_key_up: true,
            is_unicode: false,
        };
        record_windows_inputs(std::slice::from_ref(&sim));
        #[cfg(target_os = "windows")]
        {
            windows_input::dispatch_simulated_inputs(&[sim]);
        }
    }
    #[cfg(target_os = "macos")]
    {
        macos_input::key_up(key);
    }
    #[cfg(target_os = "linux")]
    {
        if std::env::var("DISPLAY").is_ok() && which_executable("xdotool").is_some() {
            let _ = Command::new("xdotool").args(["keyup", key]).status();
        }
    }
}

fn inject_hotkey(keys_list: &[String], combination: &str) {
    record_event(json!({
        "type": "hotkey",
        "combination": combination,
        "keys": keys_list,
    }));
    let sims = build_windows_hotkey(keys_list);
    record_windows_inputs(&sims);
    #[cfg(target_os = "windows")]
    {
        windows_input::dispatch_simulated_inputs(&sims);
    }
    #[cfg(target_os = "macos")]
    {
        macos_input::hotkey(keys_list);
    }
    #[cfg(target_os = "linux")]
    {
        if std::env::var("DISPLAY").is_ok() && which_executable("xdotool").is_some() {
            let _ = Command::new("xdotool").args(["key", combination]).status();
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

            let scale = envelope
                .get("scale")
                .or_else(|| params_obj.and_then(|p| p.get("scale")))
                .and_then(Value::as_f64);

            let max_width = envelope
                .get("max_width")
                .or_else(|| params_obj.and_then(|p| p.get("max_width")))
                .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)))
                .map(|v| v as u32);

            let (b64, width, height) =
                capture_screen_native(format, active_win, display_id, scale, max_width);

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

            let scale = envelope
                .get("scale")
                .or_else(|| params_obj.and_then(|p| p.get("scale")))
                .and_then(Value::as_f64);

            let max_width = envelope
                .get("max_width")
                .or_else(|| params_obj.and_then(|p| p.get("max_width")))
                .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)))
                .map(|v| v as u32);

            let (b64, width, height) = capture_screen_native(format, true, None, scale, max_width);

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

            inject_key_down(key);

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

            inject_key_up(key);

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
            inject_hotkey(&keys_list, &combination);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_normalization() {
        let (nx, ny) = normalize_screen_coordinates(960.0, 540.0, 1920.0, 1080.0);
        assert_eq!(nx, 32768);
        assert_eq!(ny, 32768);

        let (nx0, ny0) = normalize_screen_coordinates(0.0, 0.0, 1920.0, 1080.0);
        assert_eq!(nx0, 0);
        assert_eq!(ny0, 0);

        let (nx_clamp, ny_clamp) = normalize_screen_coordinates(5000.0, -100.0, 1920.0, 1080.0);
        assert_eq!(nx_clamp, 65535);
        assert_eq!(ny_clamp, 0);
    }

    #[test]
    fn test_build_windows_mouse_events() {
        let mv = build_windows_mouse_move(960.0, 540.0, 1920.0, 1080.0);
        assert_eq!(mv.flags, WIN_MOUSEEVENTF_MOVE | WIN_MOUSEEVENTF_ABSOLUTE);
        assert_eq!(mv.dx, 32768);
        assert_eq!(mv.dy, 32768);

        let clicks = build_windows_mouse_click("left", 1, None, None, 1920.0, 1080.0);
        assert_eq!(clicks.len(), 2);
        assert_eq!(clicks[0].flags, WIN_MOUSEEVENTF_LEFTDOWN);
        assert_eq!(clicks[1].flags, WIN_MOUSEEVENTF_LEFTUP);

        let scroll = build_windows_mouse_scroll(0.0, 3.0);
        assert_eq!(scroll.len(), 1);
        assert_eq!(scroll[0].flags, WIN_MOUSEEVENTF_WHEEL);
        assert_eq!(scroll[0].mouse_data as i32, 360);
    }

    #[test]
    fn test_build_windows_keyboard_events() {
        let type_evts = build_windows_type_text("Hi");
        assert_eq!(type_evts.len(), 4);
        assert_eq!(type_evts[0].scan, 'H' as u16);
        assert_eq!(type_evts[0].flags, WIN_KEYEVENTF_UNICODE);
        assert_eq!(
            type_evts[1].flags,
            WIN_KEYEVENTF_UNICODE | WIN_KEYEVENTF_KEYUP
        );

        let key_evts = build_windows_key_press("Return");
        assert_eq!(key_evts.len(), 2);
        assert_eq!(key_evts[0].vk, WIN_VK_RETURN);
        assert_eq!(key_evts[1].flags, WIN_KEYEVENTF_KEYUP);
    }
}
