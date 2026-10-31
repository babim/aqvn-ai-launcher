//! System tray.
//!
//! Linux never emits tray click events, so show/hide is a menu item by design.
//! The icon doubles as a status indicator: official logo while idle, amber badge dot
//! while a chat response is streaming.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

pub const TRAY_ID: &str = "aqvn-tray";

pub const ID_SERVER_STATUS: &str = "server_status";
pub const ID_SERVER_URL: &str = "server_status";
pub const ID_GATEWAY_STATUS: &str = "gateway_status";
pub const ID_TOGGLE: &str = "toggle";
pub const ID_SETTINGS: &str = "settings";
pub const ID_HELP: &str = "help";
pub const ID_ABOUT: &str = "about";
pub const ID_QUIT: &str = "quit";

static CURRENT_SERVER_URL: RwLock<Option<String>> = RwLock::new(None);
static CURRENT_GATEWAY_CONNECTED: AtomicBool = AtomicBool::new(false);

const TRAY_SIZE: u32 = 32;
// Raw RGBA (32x32) shipped instead of PNG so the binary does not need a
// decoder feature just for two 4 KiB glyphs.
const IDLE_RGBA: &[u8] = include_bytes!("../../icons/tray-idle.rgba");
const BUSY_RGBA: &[u8] = include_bytes!("../../icons/tray-busy.rgba");

/// Raw RGBA bytes for the requested activity state.
pub fn icon_bytes(busy: bool) -> &'static [u8] {
    if busy {
        BUSY_RGBA
    } else {
        IDLE_RGBA
    }
}

/// Tauri image for the requested activity state.
pub fn image(busy: bool) -> Image<'static> {
    Image::new(icon_bytes(busy), TRAY_SIZE, TRAY_SIZE)
}

/// Tooltip text for the requested activity state.
pub fn tooltip(busy: bool) -> &'static str {
    if busy {
        "AQVN Workspace — working"
    } else {
        "AQVN Workspace — idle"
    }
}

/// Formats the active server URL status label.
pub fn format_server_url_label(server_url: &str) -> String {
    if server_url.trim().is_empty() {
        "Server: None".to_string()
    } else {
        format!("Server: {server_url}")
    }
}

/// Alias for `format_server_url_label`.
pub fn format_server_label(server_url: &str) -> String {
    format_server_url_label(server_url)
}

/// Alias for `format_server_url_label`.
pub fn format_server_status(server_url: &str) -> String {
    format_server_url_label(server_url)
}

/// Formats the Gateway connection status label.
pub fn format_gateway_status_label(connected: bool) -> String {
    format!(
        "Gateway: {}",
        if connected {
            "Connected"
        } else {
            "Disconnected"
        }
    )
}

/// Alias for `format_gateway_status_label`.
pub fn format_gateway_status(connected: bool) -> String {
    format_gateway_status_label(connected)
}

/// Alias for `format_gateway_status_label`.
pub fn format_gateway_label(connected: bool) -> String {
    format_gateway_status_label(connected)
}

/// Gets the currently recorded active server URL.
pub fn get_server_url() -> String {
    CURRENT_SERVER_URL
        .read()
        .ok()
        .and_then(|lock| lock.clone())
        .unwrap_or_else(|| "https://a0.matmagoc.com".to_string())
}

/// Checks whether Gateway connection is currently active.
pub fn is_gateway_connected() -> bool {
    CURRENT_GATEWAY_CONNECTED.load(Ordering::SeqCst)
}

/// Sets the Gateway connection state.
pub fn set_gateway_connected(connected: bool) {
    CURRENT_GATEWAY_CONNECTED.store(connected, Ordering::SeqCst);
}

/// Builds the system tray menu displaying dynamic server and gateway status.
pub fn build_tray_menu(
    app: &AppHandle,
    server_url: &str,
    gateway_connected: bool,
) -> tauri::Result<Menu<tauri::Wry>> {
    let server_item =
        MenuItemBuilder::with_id(ID_SERVER_STATUS, format_server_url_label(server_url))
            .enabled(false)
            .build(app)?;
    let gateway_item = MenuItemBuilder::with_id(
        ID_GATEWAY_STATUS,
        format_gateway_status_label(gateway_connected),
    )
    .enabled(false)
    .build(app)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let toggle = MenuItemBuilder::with_id(ID_TOGGLE, "Show / Hide window").build(app)?;
    let settings = MenuItemBuilder::with_id(ID_SETTINGS, "Launcher Settings...").build(app)?;
    let help = MenuItemBuilder::with_id(ID_HELP, "Help & Shortcuts (F1)...").build(app)?;
    let about = MenuItemBuilder::with_id(ID_ABOUT, "About AQVN Workspace...").build(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItemBuilder::with_id(ID_QUIT, "Quit AQVN Workspace").build(app)?;

    Menu::with_items(
        app,
        &[
            &server_item,
            &gateway_item,
            &sep1,
            &toggle,
            &settings,
            &help,
            &about,
            &sep2,
            &quit,
        ],
    )
}

/// Registers the tray icon with its menu. Must run on the main thread.
pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let server_url = app
        .try_state::<crate::config::ConfigWatcher>()
        .map(|w| w.current().server_url)
        .unwrap_or_else(|| "https://a0.matmagoc.com".to_string());
    let gateway_connected = is_gateway_connected();

    if let Ok(mut lock) = CURRENT_SERVER_URL.write() {
        *lock = Some(server_url.clone());
    }

    let menu = build_tray_menu(app, &server_url, gateway_connected)?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(image(false))
        .tooltip(tooltip(false))
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            ID_TOGGLE => super::toggle_main_window(app),
            ID_SETTINGS => super::open_settings_window(app),
            ID_HELP => super::open_help_window(app),
            ID_ABOUT => super::open_about_window(app),
            ID_QUIT => {
                // Process exit closes the proxy and the RFC/SSH listeners.
                eprintln!("aqvn: quitting from tray menu");
                app.exit(0);
            }
            ID_SERVER_STATUS | ID_GATEWAY_STATUS => {}
            other => eprintln!("aqvn: unknown tray menu id {other:?}"),
        })
        .build(app)?;

    eprintln!(
        "aqvn: tray registered (menu: Show / Hide window, Launcher Settings..., Help & Shortcuts (F1)..., About AQVN Workspace..., Quit)"
    );
    Ok(())
}

/// Updates the dynamic tray menu status indicators with active server URL and Gateway status.
pub fn update_tray_status(
    app: &AppHandle,
    server_url: &str,
    gateway_connected: bool,
) -> tauri::Result<()> {
    if let Ok(mut lock) = CURRENT_SERVER_URL.write() {
        *lock = Some(server_url.to_string());
    }
    CURRENT_GATEWAY_CONNECTED.store(gateway_connected, Ordering::SeqCst);

    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        eprintln!(
            "aqvn: tray not registered, stored status server={server_url}, gateway={gateway_connected}"
        );
        return Ok(());
    };

    let menu = build_tray_menu(app, server_url, gateway_connected)?;
    tray.set_menu(Some(menu))?;
    eprintln!(
        "aqvn: tray menu updated (server: {server_url}, gateway: {})",
        if gateway_connected {
            "connected"
        } else {
            "disconnected"
        }
    );
    Ok(())
}

/// Alias for `update_tray_status`.
pub fn set_tray_status(
    app: &AppHandle,
    server_url: &str,
    gateway_connected: bool,
) -> tauri::Result<()> {
    update_tray_status(app, server_url, gateway_connected)
}

/// Swaps the tray icon between busy and idle. Logged so headless runs can
/// prove the transition even when the glyph is hard to see in a capture.
pub fn set_busy(app: &AppHandle, busy: bool) -> tauri::Result<()> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        eprintln!("aqvn: tray icon not registered, ignoring busy={busy}");
        return Ok(());
    };
    tray.set_icon(Some(image(busy)))?;
    tray.set_tooltip(Some(tooltip(busy)))?;
    eprintln!("aqvn: tray icon -> {}", if busy { "busy" } else { "idle" });
    Ok(())
}
