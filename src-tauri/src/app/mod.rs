//! Desktop-only OS integration: tray icon, global hotkey, notifications, and
//! external-link policy. Compiled out of mobile builds.

pub mod hotkey;
pub mod links;
pub mod notify;
pub mod tray;

use tauri::{AppHandle, Manager};

/// Shows or hides the main window (tray menu item and global hotkey share it).
pub fn toggle_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        eprintln!("aqvn: toggle ignored, main window is not available");
        return;
    };
    let visible = window.is_visible().unwrap_or(false);
    if visible {
        match window.hide() {
            Ok(()) => eprintln!("aqvn: window hidden"),
            Err(err) => eprintln!("aqvn: window hide failed: {err}"),
        }
    } else {
        if let Err(err) = window.show() {
            eprintln!("aqvn: window show failed: {err}");
        }
        if let Err(err) = window.set_focus() {
            eprintln!("aqvn: window focus failed: {err}");
        }
        eprintln!("aqvn: window shown");
    }
}

/// Busy/idle tray icon, driven by chat activity reported from the UI.
#[tauri::command]
pub fn set_tray_busy(app: AppHandle, busy: bool) -> Result<(), String> {
    tray::set_busy(&app, busy).map_err(|err| err.to_string())
}
