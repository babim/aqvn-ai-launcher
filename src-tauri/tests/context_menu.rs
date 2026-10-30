//! Tests for Native OS Context Menu (VAL-UI-002 and VAL-UI-003).

use aqvn_workspace_lib::app::context_menu::{
    ContextMenuAction, ID_HARD_REFRESH, ID_HELP, ID_RELOAD, ID_RESET_ZOOM, ID_SETTINGS, ID_ZOOM_IN,
    ID_ZOOM_OUT, LABEL_HARD_REFRESH, LABEL_HELP, LABEL_RELOAD, LABEL_RESET_ZOOM, LABEL_SETTINGS,
    LABEL_ZOOM_IN, LABEL_ZOOM_OUT,
};

#[test]
fn test_context_menu_item_constants() {
    assert_eq!(ID_SETTINGS, "settings");
    assert_eq!(ID_ZOOM_IN, "zoom_in");
    assert_eq!(ID_ZOOM_OUT, "zoom_out");
    assert_eq!(ID_RESET_ZOOM, "reset_zoom");
    assert_eq!(ID_RELOAD, "reload");
    assert_eq!(ID_HARD_REFRESH, "hard_refresh");
    assert_eq!(ID_HELP, "help");

    assert_eq!(LABEL_SETTINGS, "Settings");
    assert_eq!(LABEL_ZOOM_IN, "Zoom In");
    assert_eq!(LABEL_ZOOM_OUT, "Zoom Out");
    assert_eq!(LABEL_RESET_ZOOM, "Reset Zoom");
    assert_eq!(LABEL_RELOAD, "Reload");
    assert_eq!(LABEL_HARD_REFRESH, "Hard Refresh");
    assert_eq!(LABEL_HELP, "Help & Shortcuts");
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
