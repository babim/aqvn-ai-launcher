//! Dynamic global hotkey registration and hot-reload watcher.
//!
//! `global-hotkey 0.8` grabs the key through X11 only: on Wayland the grab is
//! unavailable upstream, so the tray menu item stays the guaranteed path. The
//! registration result is always logged — success or failure with the reason.

use crate::config::ConfigWatcher;
use std::sync::Mutex;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

pub const DEFAULT_HOTKEY: &str = "Alt+Space";
pub const HOTKEY: &str = DEFAULT_HOTKEY;

static CURRENT_HOTKEY: Mutex<Option<String>> = Mutex::new(None);

pub fn register(app: &AppHandle, hotkey_str: &str) {
    let hotkey = hotkey_str.trim();
    if hotkey.is_empty() {
        return;
    }
    let app_handle = app.clone();
    let registered = app
        .global_shortcut()
        .on_shortcut(hotkey, move |_app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                super::toggle_main_window(&app_handle);
            }
        });
    match registered {
        Ok(()) => {
            let mut guard = CURRENT_HOTKEY.lock().unwrap();
            *guard = Some(hotkey.to_string());
            eprintln!("aqvn: global hotkey {hotkey} registered successfully");
        }
        Err(err) => eprintln!("aqvn: global hotkey {hotkey} registration FAILED: {err}"),
    }
}

pub fn update_hotkey(app: &AppHandle, new_hotkey: &str) {
    let mut guard = CURRENT_HOTKEY.lock().unwrap();
    if let Some(old) = guard.as_deref() {
        if old == new_hotkey {
            return;
        }
        let _ = app.global_shortcut().unregister(old);
    }
    *guard = None;
    drop(guard);
    register(app, new_hotkey);
}

pub fn watch(app: AppHandle, watcher: ConfigWatcher) {
    tauri::async_runtime::spawn(async move {
        let mut rx = watcher.subscribe();
        let mut current = watcher.current().global_hotkey;
        while rx.changed().await.is_ok() {
            let next = rx.borrow().global_hotkey.clone();
            if next != current {
                update_hotkey(&app, &next);
                current = next;
            }
        }
    });
}
