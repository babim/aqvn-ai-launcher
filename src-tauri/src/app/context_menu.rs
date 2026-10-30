//! Native OS context menu and action dispatching (VAL-UI-002 and VAL-UI-003).

use std::sync::atomic::{AtomicU32, Ordering};
use tauri::menu::{Menu, MenuItemBuilder, PredefinedMenuItem};
use tauri::{AppHandle, Manager};

pub const ZOOM_MIN: f64 = 0.7;
pub const ZOOM_MAX: f64 = 2.0;
pub const ZOOM_STEP: f64 = 0.1;
pub const ZOOM_DEFAULT: f64 = 1.0;

static CURRENT_ZOOM_PERCENT: AtomicU32 = AtomicU32::new(100);

pub fn get_zoom_scale() -> f64 {
    (CURRENT_ZOOM_PERCENT.load(Ordering::SeqCst) as f64) / 100.0
}

pub fn set_zoom_scale(scale: f64) -> f64 {
    let clamped = scale.clamp(ZOOM_MIN, ZOOM_MAX);
    let stepped = (clamped * 10.0).round() / 10.0;
    let percent = (stepped * 100.0).round() as u32;
    CURRENT_ZOOM_PERCENT.store(percent, Ordering::SeqCst);
    (percent as f64) / 100.0
}

pub fn get_zoom_percent() -> u32 {
    CURRENT_ZOOM_PERCENT.load(Ordering::SeqCst)
}

pub fn format_zoom_label(percent: u32) -> String {
    format!("{LABEL_ZOOM_LEVEL} ({percent}%)")
}

pub fn zoom_in_scale() -> f64 {
    let current = get_zoom_scale();
    set_zoom_scale(current + ZOOM_STEP)
}

pub fn zoom_out_scale() -> f64 {
    let current = get_zoom_scale();
    set_zoom_scale(current - ZOOM_STEP)
}

pub fn reset_zoom_scale() -> f64 {
    set_zoom_scale(ZOOM_DEFAULT)
}

pub const ID_SETTINGS: &str = "settings";
pub const ID_ZOOM_LEVEL: &str = "zoom_level";
pub const ID_ZOOM_IN: &str = "zoom_in";
pub const ID_ZOOM_OUT: &str = "zoom_out";
pub const ID_RESET_ZOOM: &str = "reset_zoom";
pub const ID_RELOAD: &str = "reload";
pub const ID_HARD_REFRESH: &str = "hard_refresh";
pub const ID_HELP: &str = "help";
pub const ID_ABOUT: &str = "about";

pub const LABEL_SETTINGS: &str = "Settings";
pub const LABEL_ZOOM_LEVEL: &str = "Zoom";
pub const LABEL_ZOOM_IN: &str = "Zoom In";
pub const LABEL_ZOOM_OUT: &str = "Zoom Out";
pub const LABEL_RESET_ZOOM: &str = "Reset Zoom";
pub const LABEL_RELOAD: &str = "Reload";
pub const LABEL_HARD_REFRESH: &str = "Hard Refresh";
pub const LABEL_HELP: &str = "Help & Shortcuts";
pub const LABEL_ABOUT: &str = "About AQVN Workspace";

/// Builds the native OS context menu.
pub fn build_context_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let settings = MenuItemBuilder::with_id(ID_SETTINGS, LABEL_SETTINGS)
        .accelerator("CmdOrCtrl+,")
        .build(app)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let zoom_level_label = format_zoom_label(get_zoom_percent());
    let zoom_level = MenuItemBuilder::with_id(ID_ZOOM_LEVEL, zoom_level_label)
        .enabled(false)
        .build(app)?;
    let zoom_in = MenuItemBuilder::with_id(ID_ZOOM_IN, LABEL_ZOOM_IN)
        .accelerator("CmdOrCtrl+=")
        .build(app)?;
    let zoom_out = MenuItemBuilder::with_id(ID_ZOOM_OUT, LABEL_ZOOM_OUT)
        .accelerator("CmdOrCtrl+-")
        .build(app)?;
    let reset_zoom = MenuItemBuilder::with_id(ID_RESET_ZOOM, LABEL_RESET_ZOOM)
        .accelerator("CmdOrCtrl+0")
        .build(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let reload = MenuItemBuilder::with_id(ID_RELOAD, LABEL_RELOAD)
        .accelerator("F5")
        .build(app)?;
    let hard_refresh = MenuItemBuilder::with_id(ID_HARD_REFRESH, LABEL_HARD_REFRESH)
        .accelerator("CmdOrCtrl+Shift+R")
        .build(app)?;
    let sep3 = PredefinedMenuItem::separator(app)?;
    let help = MenuItemBuilder::with_id(ID_HELP, LABEL_HELP)
        .accelerator("F1")
        .build(app)?;
    let sep4 = PredefinedMenuItem::separator(app)?;
    let about = MenuItemBuilder::with_id(ID_ABOUT, LABEL_ABOUT).build(app)?;

    Menu::with_items(
        app,
        &[
            &settings,
            &sep1,
            &zoom_level,
            &zoom_in,
            &zoom_out,
            &reset_zoom,
            &sep2,
            &reload,
            &hard_refresh,
            &sep3,
            &help,
            &sep4,
            &about,
        ],
    )
}

/// Enumeration of all context menu actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextMenuAction {
    Settings,
    ZoomIn,
    ZoomOut,
    ResetZoom,
    Reload,
    HardRefresh,
    Help,
    About,
}

impl ContextMenuAction {
    /// Maps a menu identifier string to a ContextMenuAction.
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            ID_SETTINGS => Some(Self::Settings),
            ID_ZOOM_IN => Some(Self::ZoomIn),
            ID_ZOOM_OUT => Some(Self::ZoomOut),
            ID_RESET_ZOOM | "zoom_reset" => Some(Self::ResetZoom),
            ID_RELOAD => Some(Self::Reload),
            ID_HARD_REFRESH => Some(Self::HardRefresh),
            ID_HELP => Some(Self::Help),
            ID_ABOUT => Some(Self::About),
            _ => None,
        }
    }

    /// Returns the JavaScript code evaluated in the webview for webview actions.
    pub fn js_action(&self) -> Option<&'static str> {
        match self {
            Self::ZoomIn => {
                Some("if (window.__AQVN_CONTAINER__) window.__AQVN_CONTAINER__.zoomIn();")
            }
            Self::ZoomOut => {
                Some("if (window.__AQVN_CONTAINER__) window.__AQVN_CONTAINER__.zoomOut();")
            }
            Self::ResetZoom => {
                Some("if (window.__AQVN_CONTAINER__) window.__AQVN_CONTAINER__.resetZoom();")
            }
            Self::Reload => Some("window.location.reload();"),
            Self::HardRefresh => Some(
                "if (window.__AQVN_CONTAINER__) window.__AQVN_CONTAINER__.hardRefresh(); else window.location.reload();",
            ),
            _ => None,
        }
    }
}

/// Dispatches an action to either native window management or webview script evaluation.
pub fn dispatch_action(
    app: &AppHandle,
    window: Option<&tauri::WebviewWindow>,
    action: ContextMenuAction,
) -> Result<(), String> {
    match action {
        ContextMenuAction::Settings => {
            super::open_settings_window(app);
            Ok(())
        }
        ContextMenuAction::Help => {
            super::open_help_window(app);
            Ok(())
        }
        ContextMenuAction::About => {
            super::open_about_window(app);
            Ok(())
        }
        ContextMenuAction::ZoomIn | ContextMenuAction::ZoomOut | ContextMenuAction::ResetZoom => {
            let native_scale = match action {
                ContextMenuAction::ZoomIn => Some(zoom_in_scale()),
                ContextMenuAction::ZoomOut => Some(zoom_out_scale()),
                ContextMenuAction::ResetZoom => Some(reset_zoom_scale()),
                _ => None,
            };

            let target_window = window.cloned().or_else(|| app.get_webview_window("main"));
            if let (Some(w), Some(scale)) = (&target_window, native_scale) {
                let _ = w.set_zoom(scale);
            }

            if let Some(js) = action.js_action() {
                if let Some(w) = &target_window {
                    w.eval(js).map_err(|e| e.to_string())?;
                }
            }
            Ok(())
        }
        ContextMenuAction::Reload | ContextMenuAction::HardRefresh => {
            if let Some(js) = action.js_action() {
                if let Some(w) = window {
                    w.eval(js).map_err(|e| e.to_string())?;
                } else if let Some(w) = app.get_webview_window("main") {
                    w.eval(js).map_err(|e| e.to_string())?;
                }
            }
            Ok(())
        }
    }
}

/// Resolves a menu ID and dispatches its corresponding action.
pub fn handle_menu_id(
    app: &AppHandle,
    window: Option<&tauri::WebviewWindow>,
    id: &str,
) -> Result<Option<ContextMenuAction>, String> {
    if let Some(action) = ContextMenuAction::from_id(id) {
        dispatch_action(app, window, action)?;
        Ok(Some(action))
    } else {
        Ok(None)
    }
}

/// Tauri command to trigger native OS context menu popup on the target window.
#[tauri::command]
pub fn show_context_menu(window: tauri::WebviewWindow) -> Result<(), String> {
    let menu = build_context_menu(window.app_handle()).map_err(|e| e.to_string())?;
    window.popup_menu(&menu).map_err(|e| e.to_string())?;
    Ok(())
}
