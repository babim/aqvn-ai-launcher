//! OS integration: tray icon, global hotkey, notifications, and
//! external-link policy. Tray and hotkey are compiled out of mobile builds.

#[cfg(not(target_os = "android"))]
pub mod hotkey;
pub mod links;
pub mod notify;
#[cfg(not(target_os = "android"))]
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

/// Opens or focuses the Launcher Settings window.
pub fn open_settings_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        if let Err(err) = window.show() {
            eprintln!("aqvn: settings window show failed: {err}");
        }
        if let Err(err) = window.set_focus() {
            eprintln!("aqvn: settings window focus failed: {err}");
        }
    } else {
        match tauri::WebviewWindowBuilder::new(
            app,
            "settings",
            tauri::WebviewUrl::App("settings.html".into()),
        )
        .title("AQVN Workspace — Settings")
        .inner_size(800.0, 720.0)
        .resizable(true)
        .build()
        {
            Ok(_) => eprintln!("aqvn: settings window created"),
            Err(err) => eprintln!("aqvn: failed to open settings window: {err}"),
        }
    }
}

/// Tauri IPC command to open the Launcher Settings window.
#[tauri::command]
pub fn open_settings(app: AppHandle) -> Result<(), String> {
    open_settings_window(&app);
    Ok(())
}

/// Opens or focuses the Help & Shortcuts window.
pub fn open_help_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("help") {
        if let Err(err) = window.show() {
            eprintln!("aqvn: help window show failed: {err}");
        }
        if let Err(err) = window.set_focus() {
            eprintln!("aqvn: help window focus failed: {err}");
        }
    } else {
        match tauri::WebviewWindowBuilder::new(
            app,
            "help",
            tauri::WebviewUrl::App("help.html".into()),
        )
        .title("AQVN Workspace — Help & Shortcuts")
        .inner_size(840.0, 700.0)
        .resizable(true)
        .build()
        {
            Ok(_) => eprintln!("aqvn: help window created"),
            Err(err) => eprintln!("aqvn: failed to open help window: {err}"),
        }
    }
}

/// Tauri IPC command to open the Help & Shortcuts window.
#[tauri::command]
pub fn open_help(app: AppHandle) -> Result<(), String> {
    open_help_window(&app);
    Ok(())
}

/// Busy/idle tray icon, driven by chat activity reported from the UI.
#[tauri::command]
pub fn set_tray_busy(app: AppHandle, busy: bool) -> Result<(), String> {
    #[cfg(not(target_os = "android"))]
    {
        tray::set_busy(&app, busy).map_err(|err| err.to_string())
    }
    #[cfg(target_os = "android")]
    {
        let _ = (&app, busy);
        Ok(())
    }
}
