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
fn proxy_url_navigates_inside_the_webview() {
    let url = Url::parse("http://127.0.0.1:9041/plugins/aqvn_intel_suite/webui/canvas.html")
        .expect("test url parses");
    assert_eq!(
        links::classify_with_proxy(&url, Some(9041)),
        links::Nav::App,
        "proxy url on matching port must stay in webview"
    );
    assert_eq!(
        links::classify_with_proxy(&url, Some(8080)),
        links::Nav::External,
        "proxy url on non-matching port must be external"
    );
}

#[test]
fn external_https_is_handed_to_the_system_browser() {
    for raw in [
        "https://example.com/docs",
        "http://localhost:3000/x",
        "mailto:support@aqvn.net",
    ] {
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

#[test]
fn container_window_config_points_to_accelerated_proxy() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let raw = std::fs::read_to_string(manifest_dir.join("tauri.conf.json")).unwrap();
    let conf: tauri::utils::config::Config = serde_json::from_str(&raw).unwrap();
    let main_window = conf.app.windows.iter().find(|w| w.label == "main").unwrap();
    let configured = links::configure_container_window(main_window, 9041);
    assert_eq!(
        configured.url,
        tauri::WebviewUrl::External(links::container_url(9041))
    );
}

#[test]
fn test_container_navigation_policy() {
    let port = 9041;
    let container_root = links::container_url(port);
    assert_eq!(container_root.as_str(), "http://127.0.0.1:9041/");
    assert_eq!(
        links::classify_with_proxy(&container_root, Some(port)),
        links::Nav::App,
        "container root must stay in webview"
    );

    // Internal app routes and frames stay inside the webview
    let internal_routes = [
        "http://127.0.0.1:9041/",
        "http://127.0.0.1:9041/index.html",
        "http://127.0.0.1:9041/login",
        "http://127.0.0.1:9041/settings.html",
        "http://127.0.0.1:9041/plugins/aqvn_intel_suite/webui/canvas.html",
        "http://127.0.0.1:9041/extensions/my_extension/index.html",
        "http://127.0.0.1:9041/public/asset.png",
        "http://127.0.0.1:9041/vendor/marked.min.js",
        "http://127.0.0.1:9041/css/app.css",
        "http://127.0.0.1:9041/js/bundle.js",
        "http://localhost:9041/plugins/aqvn_intel_suite/webui/canvas.html",
        "tauri://localhost/index.html",
        "http://tauri.localhost/index.html",
        "about:blank",
    ];
    for raw in internal_routes {
        let url = Url::parse(raw).expect("valid url");
        assert_eq!(
            links::classify_with_proxy(&url, Some(port)),
            links::Nav::App,
            "Internal route {raw} must stay in webview"
        );
    }

    // External links open in system default browser
    let external_links = [
        "https://example.com/docs",
        "https://github.com/aqvn/workspace",
        "http://example.com/login",
        "http://127.0.0.1:8080/plugins/canvas.html",
        "http://localhost:3000/api",
        "https://a0.matmagoc.com/chat",
    ];
    for raw in external_links {
        let url = Url::parse(raw).expect("valid url");
        assert_eq!(
            links::classify_with_proxy(&url, Some(port)),
            links::Nav::External,
            "External link {raw} must open in system browser"
        );
    }

    // Blocked schemes
    let blocked_schemes = [
        "file:///etc/passwd",
        "javascript:alert(1)",
        "ftp://mirror.example.com",
    ];
    for raw in blocked_schemes {
        let url = Url::parse(raw).expect("valid url");
        assert_eq!(
            links::classify_with_proxy(&url, Some(port)),
            links::Nav::Blocked,
            "Unsupported scheme {raw} must be blocked"
        );
    }
}

#[test]
fn main_window_uses_container_configuration() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib_rs = std::fs::read_to_string(manifest_dir.join("src/lib.rs")).unwrap();
    assert!(
        lib_rs.contains("configure_container_window"),
        "src/lib.rs must use configure_container_window to point webview window at accelerated proxy"
    );
}

#[test]
fn init_script_injects_notification_and_tray_bridge() {
    let script = aqvn_workspace_lib::proxy::server::init_script(9041);
    assert!(script.contains("notify_task_complete"));
    assert!(script.contains("set_tray_busy"));
    assert!(script.contains("__AQVN_CONTAINER__"));
}

#[test]
fn test_settings_window_helper_exists() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let app_mod = std::fs::read_to_string(manifest_dir.join("src/app/mod.rs")).unwrap();
    assert!(
        app_mod.contains("pub fn open_settings_window"),
        "app/mod.rs must define open_settings_window"
    );
    assert!(
        app_mod.contains("pub fn open_settings("),
        "app/mod.rs must define open_settings IPC command"
    );
    assert!(
        app_mod.contains("settings.html"),
        "open_settings_window must load settings.html"
    );
    assert!(
        app_mod.contains("AQVN Workspace — Settings"),
        "open_settings_window must set window title"
    );

    let tray_rs = std::fs::read_to_string(manifest_dir.join("src/app/tray.rs")).unwrap();
    assert!(
        tray_rs.contains("Launcher Settings..."),
        "tray.rs must contain Launcher Settings... menu item"
    );
    assert!(
        tray_rs.contains("open_settings_window"),
        "tray.rs must route settings menu item to open_settings_window"
    );

    let lib_rs = std::fs::read_to_string(manifest_dir.join("src/lib.rs")).unwrap();
    assert!(
        lib_rs.contains("app::open_settings"),
        "lib.rs must register app::open_settings command"
    );

    let container_js = std::fs::read_to_string(manifest_dir.join("../ui/container.js")).unwrap();
    assert!(
        container_js.contains("open_settings"),
        "ui/container.js must invoke open_settings"
    );
    assert!(
        container_js.contains("ctrlKey") || container_js.contains("metaKey"),
        "ui/container.js must check ctrlKey or metaKey"
    );
}

#[test]
fn test_help_window_helper_exists() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let app_mod = std::fs::read_to_string(manifest_dir.join("src/app/mod.rs")).unwrap();
    assert!(
        app_mod.contains("pub fn open_help_window"),
        "app/mod.rs must define open_help_window"
    );
    assert!(
        app_mod.contains("pub fn open_help("),
        "app/mod.rs must define open_help IPC command"
    );
    assert!(
        app_mod.contains("help.html"),
        "open_help_window must load help.html"
    );
    assert!(
        app_mod.contains("AQVN Workspace — Help & Shortcuts"),
        "open_help_window must set window title"
    );

    let tray_rs = std::fs::read_to_string(manifest_dir.join("src/app/tray.rs")).unwrap();
    assert!(
        tray_rs.contains("Help & Shortcuts (F1)..."),
        "tray.rs must contain Help & Shortcuts (F1)... menu item"
    );
    assert!(
        tray_rs.contains("open_help_window"),
        "tray.rs must route help menu item to open_help_window"
    );

    let lib_rs = std::fs::read_to_string(manifest_dir.join("src/lib.rs")).unwrap();
    assert!(
        lib_rs.contains("app::open_help"),
        "lib.rs must register app::open_help command"
    );

    let cap = std::fs::read_to_string(manifest_dir.join("capabilities/default.json")).unwrap();
    assert!(
        cap.contains("\"help\""),
        "capabilities/default.json must include help window"
    );
}
