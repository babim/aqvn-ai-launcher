//! `Alt+Space` global hotkey.
//!
//! `global-hotkey 0.8` grabs the key through X11 only: on Wayland the grab is
//! unavailable upstream, so the tray menu item stays the guaranteed path. The
//! registration result is always logged — success or failure with the reason.

use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

pub const HOTKEY: &str = "Alt+Space";

/// Registers the toggle hotkey and logs the outcome.
pub fn register(app: &AppHandle) {
    let registered = app
        .global_shortcut()
        .on_shortcut(HOTKEY, |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                super::toggle_main_window(app);
            }
        });

    match registered {
        Ok(()) => eprintln!(
            "aqvn: global hotkey {HOTKEY} registered (X11 XGrabKey ok, is_registered={}); Wayland unsupported — tray menu fallback",
            app.global_shortcut().is_registered(HOTKEY)
        ),
        Err(err) => eprintln!(
            "aqvn: global hotkey {HOTKEY} registration FAILED: {err}; falling back to the tray menu Show / Hide item (Wayland has no global hotkey support)"
        ),
    }
}
