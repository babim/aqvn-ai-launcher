//! Tests for Native OS Context Menu (VAL-UI-002 and VAL-UI-003).

use aqvn_workspace_lib::app::context_menu::{
    format_zoom_label, get_zoom_percent, get_zoom_scale, reset_zoom_scale, set_zoom_scale,
    zoom_in_scale, zoom_out_scale, ContextMenuAction, ID_ABOUT, ID_HARD_REFRESH, ID_HELP,
    ID_RELOAD, ID_RESET_ZOOM, ID_SETTINGS, ID_ZOOM_IN, ID_ZOOM_LEVEL, ID_ZOOM_OUT, LABEL_ABOUT,
    LABEL_HARD_REFRESH, LABEL_HELP, LABEL_RELOAD, LABEL_RESET_ZOOM, LABEL_SETTINGS, LABEL_ZOOM_IN,
    LABEL_ZOOM_LEVEL, LABEL_ZOOM_OUT, ZOOM_DEFAULT, ZOOM_MAX, ZOOM_MIN,
};

#[test]
fn test_context_menu_item_constants() {
    assert_eq!(ID_SETTINGS, "settings");
    assert_eq!(ID_ZOOM_LEVEL, "zoom_level");
    assert_eq!(ID_ZOOM_IN, "zoom_in");
    assert_eq!(ID_ZOOM_OUT, "zoom_out");
    assert_eq!(ID_RESET_ZOOM, "reset_zoom");
    assert_eq!(ID_RELOAD, "reload");
    assert_eq!(ID_HARD_REFRESH, "hard_refresh");
    assert_eq!(ID_HELP, "help");
    assert_eq!(ID_ABOUT, "about");

    assert_eq!(LABEL_SETTINGS, "Settings");
    assert_eq!(LABEL_ZOOM_LEVEL, "Zoom");
    assert_eq!(LABEL_ZOOM_IN, "Zoom In");
    assert_eq!(LABEL_ZOOM_OUT, "Zoom Out");
    assert_eq!(LABEL_RESET_ZOOM, "Reset Zoom");
    assert_eq!(LABEL_RELOAD, "Reload");
    assert_eq!(LABEL_HARD_REFRESH, "Hard Refresh");
    assert_eq!(LABEL_HELP, "Help & Shortcuts");
    assert_eq!(LABEL_ABOUT, "About AQVN Workspace");
}

#[test]
fn test_context_menu_action_resolution() {
    assert_eq!(
        ContextMenuAction::from_id("settings"),
        Some(ContextMenuAction::Settings)
    );
    assert_eq!(
        ContextMenuAction::from_id("zoom_in"),
        Some(ContextMenuAction::ZoomIn)
    );
    assert_eq!(
        ContextMenuAction::from_id("zoom_out"),
        Some(ContextMenuAction::ZoomOut)
    );
    assert_eq!(
        ContextMenuAction::from_id("reset_zoom"),
        Some(ContextMenuAction::ResetZoom)
    );
    assert_eq!(
        ContextMenuAction::from_id("zoom_reset"),
        Some(ContextMenuAction::ResetZoom)
    );
    assert_eq!(
        ContextMenuAction::from_id("reload"),
        Some(ContextMenuAction::Reload)
    );
    assert_eq!(
        ContextMenuAction::from_id("hard_refresh"),
        Some(ContextMenuAction::HardRefresh)
    );
    assert_eq!(
        ContextMenuAction::from_id("help"),
        Some(ContextMenuAction::Help)
    );
    assert_eq!(
        ContextMenuAction::from_id("about"),
        Some(ContextMenuAction::About)
    );
    assert_eq!(ContextMenuAction::from_id("unknown"), None);
}

#[test]
fn test_context_menu_action_scripts() {
    let zoom_in_js = ContextMenuAction::ZoomIn.js_action().unwrap();
    assert!(
        zoom_in_js.contains("zoomIn"),
        "ZoomIn action must call zoomIn on container bridge"
    );

    let zoom_out_js = ContextMenuAction::ZoomOut.js_action().unwrap();
    assert!(
        zoom_out_js.contains("zoomOut"),
        "ZoomOut action must call zoomOut on container bridge"
    );

    let reset_zoom_js = ContextMenuAction::ResetZoom.js_action().unwrap();
    assert!(
        reset_zoom_js.contains("resetZoom"),
        "ResetZoom action must call resetZoom on container bridge"
    );

    let reload_js = ContextMenuAction::Reload.js_action().unwrap();
    assert!(
        reload_js.contains("location.reload"),
        "Reload action must call location.reload"
    );

    let hard_refresh_js = ContextMenuAction::HardRefresh.js_action().unwrap();
    assert!(
        hard_refresh_js.contains("hardRefresh"),
        "HardRefresh action must call hardRefresh on container bridge"
    );

    assert_eq!(
        ContextMenuAction::Settings.js_action(),
        None,
        "Settings is a native window action, not js_action"
    );
    assert_eq!(
        ContextMenuAction::Help.js_action(),
        None,
        "Help is a native window action, not js_action"
    );
    assert_eq!(
        ContextMenuAction::About.js_action(),
        None,
        "About is a native window action, not js_action"
    );
}

#[test]
fn test_show_context_menu_command_registered() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib_rs = std::fs::read_to_string(manifest_dir.join("src/lib.rs")).unwrap();
    assert!(
        lib_rs.contains("show_context_menu"),
        "src/lib.rs must register show_context_menu command in invoke_handler"
    );
    assert!(
        lib_rs.contains("on_menu_event"),
        "src/lib.rs must register on_menu_event listener to dispatch menu actions"
    );

    let app_mod = std::fs::read_to_string(manifest_dir.join("src/app/mod.rs")).unwrap();
    assert!(
        app_mod.contains("context_menu"),
        "src/app/mod.rs must declare context_menu module"
    );

    let ctx_rs = std::fs::read_to_string(manifest_dir.join("src/app/context_menu.rs")).unwrap();
    assert!(
        ctx_rs.contains("pub fn show_context_menu"),
        "src/app/context_menu.rs must define show_context_menu command"
    );
    assert!(
        ctx_rs.contains("popup_menu"),
        "show_context_menu must call popup_menu on the target window"
    );
}

#[test]
fn test_context_menu_items_construction() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ctx_rs = std::fs::read_to_string(manifest_dir.join("src/app/context_menu.rs")).unwrap();

    // Verify all required menu item labels and separators exist
    for label in [
        "Settings",
        "Zoom In",
        "Zoom Out",
        "Reset Zoom",
        "Reload",
        "Hard Refresh",
        "Help & Shortcuts",
        "About AQVN Workspace",
    ] {
        assert!(
            ctx_rs.contains(label),
            "context_menu.rs must construct menu item with label '{label}'"
        );
    }

    assert!(
        ctx_rs.contains("PredefinedMenuItem::separator") || ctx_rs.contains("separator"),
        "context_menu.rs must include menu separators"
    );
    assert!(
        ctx_rs.contains("format_zoom_label") || ctx_rs.contains("Zoom ("),
        "context_menu.rs must construct dynamic zoom level indicator (VAL-MENU-001)"
    );
    assert!(
        ctx_rs.contains("ID_ZOOM_LEVEL") || ctx_rs.contains("zoom_level"),
        "context_menu.rs must include zoom level indicator item"
    );
}

#[test]
fn test_ui_container_contextmenu_listener() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let container_js = std::fs::read_to_string(manifest_dir.join("../ui/container.js")).unwrap();

    assert!(
        container_js.contains("contextmenu"),
        "ui/container.js must listen for contextmenu event"
    );
    assert!(
        container_js.contains("show_context_menu"),
        "ui/container.js must invoke show_context_menu command"
    );
    assert!(
        container_js.contains("getSelection") || container_js.contains("selection"),
        "ui/container.js must check text selection before triggering context menu"
    );
}

#[test]
fn test_capabilities_and_build_manifest_permissions() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let build_rs = std::fs::read_to_string(manifest_dir.join("build.rs")).unwrap();
    let caps_default =
        std::fs::read_to_string(manifest_dir.join("capabilities/default.json")).unwrap();

    // Verify build.rs declares app commands via AppManifest
    assert!(
        build_rs.contains("AppManifest"),
        "build.rs must configure AppManifest with application commands"
    );
    let expected_commands = [
        "show_context_menu",
        "open_settings",
        "open_help",
        "open_about",
        "set_tray_busy",
        "notify_task_complete",
        "close_current_window",
    ];
    for cmd in expected_commands {
        assert!(
            build_rs.contains(&format!("\"{cmd}\"")),
            "build.rs AppManifest commands list must include {cmd}"
        );
    }

    // Verify capabilities allow the app commands for remote proxy origin
    let parsed: serde_json::Value =
        serde_json::from_str(&caps_default).expect("capabilities/default.json must be valid JSON");
    let remote_urls = parsed["remote"]["urls"]
        .as_array()
        .expect("remote.urls must be an array")
        .iter()
        .filter_map(|v| v.as_str())
        .collect::<Vec<_>>();

    assert!(
        remote_urls.contains(&"http://127.0.0.1:*"),
        "capabilities/default.json remote.urls must include http://127.0.0.1:*"
    );
    assert!(
        remote_urls.contains(&"http://localhost:*"),
        "capabilities/default.json remote.urls must include http://localhost:*"
    );

    let permissions = parsed["permissions"]
        .as_array()
        .expect("permissions must be an array")
        .iter()
        .filter_map(|v| v.as_str())
        .collect::<Vec<_>>();

    let expected_permissions = [
        "allow-show-context-menu",
        "allow-open-settings",
        "allow-open-help",
        "allow-open-about",
        "allow-set-tray-busy",
        "allow-notify-task-complete",
        "allow-close-current-window",
    ];
    for perm in expected_permissions {
        assert!(
            permissions.contains(&perm),
            "capabilities/default.json permissions must include {perm}"
        );
    }

    // Verify generated ACL schemas
    let acl_manifest =
        std::fs::read_to_string(manifest_dir.join("gen/schemas/acl-manifests.json")).unwrap();
    let parsed_acl: serde_json::Value =
        serde_json::from_str(&acl_manifest).expect("acl-manifests.json must be valid JSON");
    let app_acl_permissions = &parsed_acl["__app-acl__"]["permissions"];
    for perm in expected_permissions {
        assert!(
            app_acl_permissions.get(perm).is_some(),
            "gen/schemas/acl-manifests.json must include {perm} in __app-acl__"
        );
    }
}

#[test]
fn test_dispatch_action_handles_native_set_zoom() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ctx_rs = std::fs::read_to_string(manifest_dir.join("src/app/context_menu.rs")).unwrap();
    assert!(
        ctx_rs.contains(".set_zoom("),
        "dispatch_action in context_menu.rs must invoke native webview.set_zoom"
    );

    // Verify zoom scale clamping and atomic state management (VAL-MENU-003)
    reset_zoom_scale();
    assert_eq!(get_zoom_percent(), 100);
    assert!((get_zoom_scale() - 1.0).abs() < f64::EPSILON);

    // Zoom in step (0.1)
    let s = zoom_in_scale();
    assert!((s - 1.1).abs() < 1e-5);
    assert_eq!(get_zoom_percent(), 110);

    // Zoom out step (0.1)
    let s = zoom_out_scale();
    assert!((s - 1.0).abs() < 1e-5);
    assert_eq!(get_zoom_percent(), 100);

    // Zoom max clamping (2.0)
    let s = set_zoom_scale(5.0);
    assert!((s - ZOOM_MAX).abs() < 1e-5);
    assert_eq!(get_zoom_percent(), 200);

    // Cannot zoom in past max
    let s = zoom_in_scale();
    assert!((s - ZOOM_MAX).abs() < 1e-5);
    assert_eq!(get_zoom_percent(), 200);

    // Zoom min clamping (0.7)
    let s = set_zoom_scale(0.1);
    assert!((s - ZOOM_MIN).abs() < 1e-5);
    assert_eq!(get_zoom_percent(), 70);

    // Cannot zoom out past min
    let s = zoom_out_scale();
    assert!((s - ZOOM_MIN).abs() < 1e-5);
    assert_eq!(get_zoom_percent(), 70);

    // Reset back to default (1.0)
    let s = reset_zoom_scale();
    assert!((s - ZOOM_DEFAULT).abs() < 1e-5);
    assert_eq!(get_zoom_percent(), 100);
}

#[test]
fn test_end_to_end_zoom_scale_synchronization() {
    // VAL-CROSS-001: End-to-end zoom scale synchronization
    reset_zoom_scale();
    assert_eq!(get_zoom_percent(), 100);
    assert_eq!(format_zoom_label(get_zoom_percent()), "Zoom (100%)");

    zoom_in_scale();
    assert_eq!(get_zoom_percent(), 110);
    assert_eq!(format_zoom_label(get_zoom_percent()), "Zoom (110%)");

    zoom_in_scale();
    assert_eq!(get_zoom_percent(), 120);
    assert_eq!(format_zoom_label(get_zoom_percent()), "Zoom (120%)");

    zoom_out_scale();
    assert_eq!(get_zoom_percent(), 110);
    assert_eq!(format_zoom_label(get_zoom_percent()), "Zoom (110%)");

    reset_zoom_scale();
    assert_eq!(get_zoom_percent(), 100);
    assert_eq!(format_zoom_label(get_zoom_percent()), "Zoom (100%)");
}
