//! System tray.
//!
//! Linux never emits tray click events, so show/hide is a menu item by design.
//! The icon doubles as a status indicator: official logo while idle, amber badge dot
//! while a chat response is streaming.

use tauri::image::Image;
use tauri::menu::{Menu, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::AppHandle;

pub const TRAY_ID: &str = "aqvn-tray";

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

/// Registers the tray icon with its menu. Must run on the main thread.
pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItemBuilder::with_id("toggle", "Show / Hide window").build(app)?;
    let settings = MenuItemBuilder::with_id("settings", "Launcher Settings...").build(app)?;
    let help = MenuItemBuilder::with_id("help", "Help & Shortcuts (F1)...").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit AQVN Workspace").build(app)?;
    let menu = Menu::with_items(app, &[&toggle, &settings, &help, &quit])?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(image(false))
        .tooltip(tooltip(false))
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle" => super::toggle_main_window(app),
            "settings" => super::open_settings_window(app),
            "help" => super::open_help_window(app),
            "quit" => {
                // Process exit closes the proxy and the RFC/SSH listeners.
                eprintln!("aqvn: quitting from tray menu");
                app.exit(0);
            }
            other => eprintln!("aqvn: unknown tray menu id {other:?}"),
        })
        .build(app)?;

    eprintln!(
        "aqvn: tray registered (menu: Show / Hide window, Launcher Settings..., Help & Shortcuts (F1)..., Quit)"
    );
    Ok(())
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
