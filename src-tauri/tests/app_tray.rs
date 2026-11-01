//! Tests for System Tray Server Profiles submenu and Open Workspace Folder launcher (VAL-UX-003).

use aqvn_workspace_lib::app::tray::{
    build_tray_menu, build_tray_menu_with_profiles, format_gateway_status_label,
    format_server_url_label, get_server_profiles, get_server_url, get_workspace_path,
    open_workspace_folder, parse_profile_menu_id, profile_menu_id, resolve_workspace_path,
    set_server_profiles, set_workspace_path, switch_server_profile, ID_ABOUT, ID_GATEWAY_STATUS,
    ID_HELP, ID_OPEN_WORKSPACE, ID_QUIT, ID_SERVER_PROFILES, ID_SERVER_STATUS, ID_SETTINGS,
    ID_TOGGLE, LABEL_ABOUT, LABEL_HELP, LABEL_OPEN_WORKSPACE, LABEL_QUIT, LABEL_SERVER_PROFILES,
    LABEL_SETTINGS, LABEL_TOGGLE, PREFIX_PROFILE,
};
use aqvn_workspace_lib::config::{AppConfig, ConfigWatcher};
use tauri::menu::MenuItemKind;
use tauri::Manager;

#[test]
fn test_tray_server_profiles_and_workspace_launcher() {
    // 1. Verify constant IDs and labels
    assert_eq!(ID_SERVER_STATUS, "server_status");
    assert_eq!(ID_GATEWAY_STATUS, "gateway_status");
    assert_eq!(ID_SERVER_PROFILES, "server_profiles");
    assert_eq!(LABEL_SERVER_PROFILES, "Server Profiles");
    assert_eq!(ID_OPEN_WORKSPACE, "open_workspace");
    assert_eq!(LABEL_OPEN_WORKSPACE, "Open Workspace Folder");
    assert_eq!(ID_TOGGLE, "toggle");
    assert_eq!(LABEL_TOGGLE, "Show / Hide window");
    assert_eq!(ID_SETTINGS, "settings");
    assert_eq!(LABEL_SETTINGS, "Launcher Settings...");
    assert_eq!(ID_HELP, "help");
    assert_eq!(LABEL_HELP, "Help & Shortcuts (F1)...");
    assert_eq!(ID_ABOUT, "about");
    assert_eq!(LABEL_ABOUT, "About AQVN Workspace...");
    assert_eq!(ID_QUIT, "quit");
    assert_eq!(LABEL_QUIT, "Quit AQVN Workspace");
    assert_eq!(PREFIX_PROFILE, "profile:");

    // 2. Verify profile menu ID formatting and parsing
    assert_eq!(
        profile_menu_id("https://a0.matmagoc.com"),
        "profile:https://a0.matmagoc.com"
    );
    assert_eq!(
        profile_menu_id("http://localhost:3156"),
        "profile:http://localhost:3156"
    );
    assert_eq!(
        parse_profile_menu_id("profile:https://a0.matmagoc.com"),
        Some("https://a0.matmagoc.com")
    );
    assert_eq!(
        parse_profile_menu_id("profile:http://localhost:3156"),
        Some("http://localhost:3156")
    );
    assert_eq!(parse_profile_menu_id("open_workspace"), None);
    assert_eq!(parse_profile_menu_id("settings"), None);

    // 3. Verify workspace path resolution with ~ expansion
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .expect("home directory must exist");
    let home_path = std::path::PathBuf::from(&home);

    let resolved_default = resolve_workspace_path("~/Workspace");
    assert_eq!(resolved_default, home_path.join("Workspace"));

    let resolved_custom = resolve_workspace_path("~/my/custom/dir");
    assert_eq!(resolved_custom, home_path.join("my/custom/dir"));

    let resolved_home = resolve_workspace_path("~");
    assert_eq!(resolved_home, home_path);

    let resolved_empty = resolve_workspace_path("");
    assert_eq!(resolved_empty, home_path.join("Workspace"));

    let abs_path = std::path::PathBuf::from("/tmp/aqvn-test-ws");
    let resolved_abs = resolve_workspace_path("/tmp/aqvn-test-ws");
    assert_eq!(resolved_abs, abs_path);

    // 4. Test state getters and setters
    set_server_profiles(vec![
        "https://a0.matmagoc.com".to_string(),
        "https://custom.example.com".to_string(),
    ]);
    let profiles = get_server_profiles();
    assert_eq!(profiles.len(), 2);
    assert_eq!(profiles[0], "https://a0.matmagoc.com");
    assert_eq!(profiles[1], "https://custom.example.com");

    set_workspace_path("/tmp/test-workspace");
    assert_eq!(get_workspace_path(), "/tmp/test-workspace");

    // 5. Build mock app and verify Menu structure
    let initial_config = AppConfig {
        server_url: "https://a0.matmagoc.com".to_string(),
        server_profiles: vec![
            "https://a0.matmagoc.com".to_string(),
            "http://localhost:3156".to_string(),
        ],
        workspace_path: "~/Workspace".to_string(),
        ..Default::default()
    };

    let app = tauri::test::mock_builder()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .build(tauri::generate_context!())
        .expect("build mock app");

    let watcher = ConfigWatcher::new(initial_config.clone());
    app.manage(watcher.clone());

    let menu = build_tray_menu_with_profiles(
        app.handle(),
        &initial_config.server_url,
        true,
        &initial_config.server_profiles,
    )
    .expect("build tray menu with profiles");

    let items = menu.items().expect("menu items");
    assert_eq!(items.len(), 11, "tray menu must have 11 items");

    // Item 0: Server status (disabled)
    match &items[0] {
        MenuItemKind::MenuItem(item) => {
            assert_eq!(item.id().as_ref(), ID_SERVER_STATUS);
            assert_eq!(
                item.text().expect("text"),
                format_server_url_label("https://a0.matmagoc.com")
            );
            assert!(!item.is_enabled().expect("enabled"));
        }
        _ => panic!("Item 0 must be MenuItem for server status"),
    }

    // Item 1: Gateway status (disabled)
    match &items[1] {
        MenuItemKind::MenuItem(item) => {
            assert_eq!(item.id().as_ref(), ID_GATEWAY_STATUS);
            assert_eq!(
                item.text().expect("text"),
                format_gateway_status_label(true)
            );
            assert!(!item.is_enabled().expect("enabled"));
        }
        _ => panic!("Item 1 must be MenuItem for gateway status"),
    }

    // Item 2: Server Profiles submenu with checkmarks
    match &items[2] {
        MenuItemKind::Submenu(sub) => {
            assert_eq!(sub.id().as_ref(), ID_SERVER_PROFILES);
            assert_eq!(sub.text().expect("text"), LABEL_SERVER_PROFILES);

            let sub_items = sub.items().expect("submenu items");
            assert_eq!(sub_items.len(), 2, "must have 2 server profiles");

            // Profile 1: https://a0.matmagoc.com (ACTIVE -> checked)
            match &sub_items[0] {
                MenuItemKind::Check(check) => {
                    assert_eq!(check.id().as_ref(), "profile:https://a0.matmagoc.com");
                    assert_eq!(check.text().expect("text"), "https://a0.matmagoc.com");
                    assert!(
                        check.is_checked().expect("checked"),
                        "active profile must have checkmark"
                    );
                }
                _ => panic!("Submenu item 0 must be CheckMenuItem"),
            }

            // Profile 2: http://localhost:3156 (INACTIVE -> unchecked)
            match &sub_items[1] {
                MenuItemKind::Check(check) => {
                    assert_eq!(check.id().as_ref(), "profile:http://localhost:3156");
                    assert_eq!(check.text().expect("text"), "http://localhost:3156");
                    assert!(
                        !check.is_checked().expect("checked"),
                        "inactive profile must not have checkmark"
                    );
                }
                _ => panic!("Submenu item 1 must be CheckMenuItem"),
            }
        }
        _ => panic!("Item 2 must be Submenu for Server Profiles"),
    }

    // Item 3: Separator
    assert!(matches!(&items[3], MenuItemKind::Predefined(_)));

    // Item 4: Open Workspace Folder
    match &items[4] {
        MenuItemKind::MenuItem(item) => {
            assert_eq!(item.id().as_ref(), ID_OPEN_WORKSPACE);
            assert_eq!(item.text().expect("text"), LABEL_OPEN_WORKSPACE);
            assert!(item.is_enabled().expect("enabled"));
        }
        _ => panic!("Item 4 must be MenuItem for Open Workspace Folder"),
    }

    // Item 5: Show / Hide window
    match &items[5] {
        MenuItemKind::MenuItem(item) => {
            assert_eq!(item.id().as_ref(), ID_TOGGLE);
            assert_eq!(item.text().expect("text"), LABEL_TOGGLE);
        }
        _ => panic!("Item 5 must be MenuItem for Toggle"),
    }

    // Item 6: Settings
    match &items[6] {
        MenuItemKind::MenuItem(item) => {
            assert_eq!(item.id().as_ref(), ID_SETTINGS);
            assert_eq!(item.text().expect("text"), LABEL_SETTINGS);
        }
        _ => panic!("Item 6 must be MenuItem for Settings"),
    }

    // Item 7: Help
    match &items[7] {
        MenuItemKind::MenuItem(item) => {
            assert_eq!(item.id().as_ref(), ID_HELP);
            assert_eq!(item.text().expect("text"), LABEL_HELP);
        }
        _ => panic!("Item 7 must be MenuItem for Help"),
    }

    // Item 8: About
    match &items[8] {
        MenuItemKind::MenuItem(item) => {
            assert_eq!(item.id().as_ref(), ID_ABOUT);
            assert_eq!(item.text().expect("text"), LABEL_ABOUT);
        }
        _ => panic!("Item 8 must be MenuItem for About"),
    }

    // Item 9: Separator
    assert!(matches!(&items[9], MenuItemKind::Predefined(_)));

    // Item 10: Quit
    match &items[10] {
        MenuItemKind::MenuItem(item) => {
            assert_eq!(item.id().as_ref(), ID_QUIT);
            assert_eq!(item.text().expect("text"), LABEL_QUIT);
        }
        _ => panic!("Item 10 must be MenuItem for Quit"),
    }

    // 6. Test menu rebuilding when switching active profile
    let switched_menu = build_tray_menu_with_profiles(
        app.handle(),
        "http://localhost:3156",
        false,
        &initial_config.server_profiles,
    )
    .expect("build switched tray menu");

    let switched_items = switched_menu.items().expect("switched items");
    match &switched_items[2] {
        MenuItemKind::Submenu(sub) => {
            let sub_items = sub.items().expect("submenu items");
            match (&sub_items[0], &sub_items[1]) {
                (MenuItemKind::Check(c1), MenuItemKind::Check(c2)) => {
                    assert!(
                        !c1.is_checked().expect("checked"),
                        "old profile must now be unchecked"
                    );
                    assert!(
                        c2.is_checked().expect("checked"),
                        "switched profile must now be checked"
                    );
                }
                _ => panic!("Submenu items must be CheckMenuItems"),
            }
        }
        _ => panic!("Item 2 must be Submenu"),
    }

    // 7. Test switch_server_profile functionality
    initial_config
        .save_to_store(app.handle())
        .expect("save initial store");

    switch_server_profile(app.handle(), "http://localhost:3156").expect("switch server profile");

    assert_eq!(get_server_url(), "http://localhost:3156");
    assert_eq!(watcher.current().server_url, "http://localhost:3156");

    let loaded_config = AppConfig::load_from_store(app.handle()).expect("load from store");
    assert_eq!(loaded_config.server_url, "http://localhost:3156");

    // 8. Test open_workspace_folder with existing and new paths
    let temp_dir = std::env::temp_dir().join(format!(
        "aqvn-ws-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let res = open_workspace_folder(app.handle(), temp_dir.to_str().unwrap());
    assert!(res.is_ok(), "open_workspace_folder should return Ok(())");
    assert!(
        temp_dir.exists(),
        "open_workspace_folder must create target directory if not present"
    );
    let _ = std::fs::remove_dir_all(&temp_dir);

    // 9. Verify build_tray_menu fallback with ConfigWatcher
    let auto_menu =
        build_tray_menu(app.handle(), "http://localhost:3156", true).expect("build auto tray menu");
    assert_eq!(auto_menu.items().expect("auto items").len(), 11);
}

#[test]
fn test_tray_menu_fallback_profiles() {
    let app = tauri::test::mock_builder()
        .build(tauri::generate_context!())
        .expect("build mock app");

    // When empty profiles list is passed, active server_url must still be included
    let menu =
        build_tray_menu_with_profiles(app.handle(), "https://fallback.example.com", false, &[])
            .expect("build menu");

    let items = menu.items().expect("items");
    match &items[2] {
        MenuItemKind::Submenu(sub) => {
            let sub_items = sub.items().expect("sub items");
            assert_eq!(sub_items.len(), 1);
            match &sub_items[0] {
                MenuItemKind::Check(c) => {
                    assert_eq!(c.id().as_ref(), "profile:https://fallback.example.com");
                    assert!(c.is_checked().expect("checked"));
                }
                _ => panic!("expected check"),
            }
        }
        _ => panic!("expected submenu"),
    }
}

#[test]
fn test_tray_menu_deduplication() {
    let app = tauri::test::mock_builder()
        .build(tauri::generate_context!())
        .expect("build mock app");

    let duplicates = vec![
        "https://a0.matmagoc.com".to_string(),
        "https://a0.matmagoc.com".to_string(),
        "http://localhost:3156".to_string(),
    ];

    let menu =
        build_tray_menu_with_profiles(app.handle(), "https://a0.matmagoc.com", true, &duplicates)
            .expect("build menu");

    let items = menu.items().expect("items");
    match &items[2] {
        MenuItemKind::Submenu(sub) => {
            let sub_items = sub.items().expect("sub items");
            assert_eq!(
                sub_items.len(),
                2,
                "duplicate profiles must be deduplicated"
            );
        }
        _ => panic!("expected submenu"),
    }
}

#[test]
fn test_switch_empty_profile_fails() {
    let app = tauri::test::mock_builder()
        .build(tauri::generate_context!())
        .expect("build mock app");

    let res = switch_server_profile(app.handle(), "   ");
    assert!(res.is_err(), "empty server url must return Err");
}
