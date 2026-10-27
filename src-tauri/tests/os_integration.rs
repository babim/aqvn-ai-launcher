//! OS integration surface: navigation policy, tray icon states, hotkey binding.
//!
//! The GUI-only behaviour (menu events, X11 grabs) is exercised by
//! `tools/tray_rig.sh`; the pure decisions live here so regressions fail in CI.

use aqvn_workspace_lib::app::{hotkey, links, tray};
use url::Url;

#[test]
fn app_pages_navigate_inside_the_webview() {
    for raw in [
        "tauri://localhost/index.html",
        "tauri://localhost/settings.html",
        "http://tauri.localhost/index.html",
        "about:blank",
    ] {
        let url = Url::parse(raw).expect("test url parses");
        assert_eq!(
            links::classify(&url),
            links::Nav::App,
            "{raw} must stay in the webview"
        );
    }
}

#[test]
fn external_https_is_handed_to_the_system_browser() {
    for raw in ["https://example.com/docs", "http://localhost:3000/x"] {
        let url = Url::parse(raw).expect("test url parses");
        assert_eq!(links::classify(&url), links::Nav::External, "{raw}");
    }
}

#[test]
fn unsupported_schemes_are_blocked() {
    for raw in ["file:///etc/passwd", "javascript:alert(1)"] {
        let url = Url::parse(raw).expect("test url parses");
        assert_eq!(links::classify(&url), links::Nav::Blocked, "{raw}");
    }
}

#[test]
fn tray_icons_are_distinct_rgba_assets() {
    let idle = tray::icon_bytes(false);
    let busy = tray::icon_bytes(true);
    assert_eq!(idle.len(), 32 * 32 * 4, "idle glyph is 32x32 RGBA");
    assert_eq!(busy.len(), 32 * 32 * 4, "busy glyph is 32x32 RGBA");
    assert_ne!(idle, busy, "busy and idle icons must be visually distinct");
    assert_ne!(tray::tooltip(false), tray::tooltip(true));
    // Both states must build a Tauri image at runtime.
    assert_eq!(tray::image(false).width(), 32);
    assert_eq!(tray::image(true).height(), 32);
}

#[test]
fn hotkey_is_alt_space_and_parses() {
    assert_eq!(hotkey::HOTKEY, "Alt+Space");
    let shortcut: tauri_plugin_global_shortcut::Shortcut = hotkey::HOTKEY
        .parse()
        .expect("Alt+Space must parse as a shortcut");
    assert_eq!(shortcut.to_string().to_lowercase(), "alt+space");
}
