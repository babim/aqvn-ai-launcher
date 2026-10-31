//! Tests for configuration and store settings (VAL-SCAF-010/011/012).

use aqvn_workspace_lib::config::{
    AppConfig, ConfigWatcher, DEFAULT_ALLOWED_PATHS, DEFAULT_APP_LANGUAGE, DEFAULT_CLOSE_TO_TRAY,
    DEFAULT_GLOBAL_HOTKEY, DEFAULT_RFC_ENABLED, DEFAULT_RFC_PORT, DEFAULT_SERVER_URL,
    DEFAULT_SHOW_MENU_BAR, DEFAULT_SSH_PORT, DEFAULT_TASK_CHIME_ENABLED, STORE_FILENAME,
};
use tauri::Manager;

#[test]
fn default_config_matches_contract_requirements() {
    let conf = AppConfig::default();
    assert_eq!(conf.server_url, "https://a0.matmagoc.com");
    assert_eq!(conf.server_profiles, vec!["https://a0.matmagoc.com"]);
    assert!(!conf.rfc_enabled);
    assert_eq!(conf.rfc_port, 55080);
    assert_eq!(conf.ssh_port, 55022);
    assert_eq!(conf.allowed_paths, vec!["~/Workspace"]);
    assert_eq!(conf.rfc_password, "");
    assert_eq!(conf.ssh_password, "");

    assert_eq!(DEFAULT_SERVER_URL, "https://a0.matmagoc.com");
    const { assert!(!DEFAULT_RFC_ENABLED) };
    assert_eq!(DEFAULT_RFC_PORT, 55080);
    assert_eq!(DEFAULT_SSH_PORT, 55022);
    assert_eq!(DEFAULT_ALLOWED_PATHS, &["~/Workspace"]);
    assert_eq!(STORE_FILENAME, "settings.json");

    assert!(conf.connector_enabled);
    assert!(conf.scope_files);
    assert!(conf.scope_exec);
    assert!(conf.scope_browser);
    assert!(conf.scope_desktop);
    assert_eq!(conf.global_hotkey, DEFAULT_GLOBAL_HOTKEY);
    assert_eq!(DEFAULT_GLOBAL_HOTKEY, "Alt+Space");
    assert!(conf.close_to_tray);
    const { assert!(DEFAULT_CLOSE_TO_TRAY) };
    assert_eq!(conf.app_language, DEFAULT_APP_LANGUAGE);
    assert_eq!(DEFAULT_APP_LANGUAGE, "vi");
    assert!(!conf.show_menu_bar);
    const { assert!(!DEFAULT_SHOW_MENU_BAR) };
    assert!(conf.task_chime_enabled);
    const { assert!(DEFAULT_TASK_CHIME_ENABLED) };

    assert!(!conf.gateway_id.is_empty());
    assert_eq!(
        conf.gateway_id,
        aqvn_workspace_lib::config::default_gateway_id()
    );
    assert!(conf.gateway_id.starts_with("aqvn-gw-"));
}

#[test]
fn config_watcher_broadcasts_updates() {
    let watcher = ConfigWatcher::new(AppConfig::default());
    let mut rx = watcher.subscribe();
    assert!(!watcher.current().rfc_enabled);

    let updated = AppConfig {
        rfc_enabled: true,
        server_url: "https://custom.example".to_string(),
        rfc_password: "test-password".to_string(),
        allowed_paths: vec!["/custom/path".to_string()],
        ..Default::default()
    };

    watcher.update(updated.clone());

    assert_eq!(watcher.current(), updated);
    assert_eq!(*rx.borrow_and_update(), updated);
}

#[test]
fn config_json_serde_roundtrip() {
    let conf = AppConfig {
        server_url: "https://persist-check.example".to_string(),
        server_profiles: vec!["https://persist-check.example".to_string()],
        gateway_id: "test-gw-roundtrip".to_string(),
        workspace_path: "~/Workspace".to_string(),
        rfc_enabled: true,
        rfc_password: "secret-pass".to_string(),
        ssh_password: "ssh-pass".to_string(),
        allowed_paths: vec!["/tmp/workspace".to_string()],
        rfc_port: 55180,
        ssh_port: 55122,
        connector_enabled: false,
        scope_files: false,
        scope_exec: true,
        scope_browser: false,
        scope_desktop: true,
        global_hotkey: "Alt+Space".to_string(),
        close_to_tray: false,
        app_language: "en".to_string(),
        show_menu_bar: true,
        task_chime_enabled: false,
    };

    let json = serde_json::to_string(&conf).expect("serialize config");
    let parsed: AppConfig = serde_json::from_str(&json).expect("deserialize config");
    assert_eq!(conf, parsed);
}

#[test]
fn test_store_path_in_config_dir() {
    let app = tauri::test::mock_builder()
        .plugin(tauri_plugin_store::Builder::new().build())
        .build(tauri::generate_context!())
        .expect("build mock app");
    let store_path =
        aqvn_workspace_lib::config::store_path(app.handle()).expect("resolve store path");
    println!("Store path: {:?}", store_path);

    let config_dir = app.path().app_config_dir().expect("app_config_dir");
    println!("Config dir: {:?}", config_dir);
    assert_eq!(store_path, config_dir.join("settings.json"));
    assert!(store_path.to_string_lossy().contains("net.aqvn.workspace"));
}

#[test]
fn test_store_persistence_roundtrip() {
    let app = tauri::test::mock_builder()
        .plugin(tauri_plugin_store::Builder::new().build())
        .build(tauri::generate_context!())
        .expect("build mock app");

    let new_config = AppConfig {
        server_url: "https://persist-check.example".to_string(),
        server_profiles: vec!["https://persist-check.example".to_string()],
        rfc_enabled: true,
        rfc_password: "secret-pass".to_string(),
        allowed_paths: vec!["/tmp/test-path".to_string()],
        connector_enabled: false,
        scope_files: false,
        close_to_tray: false,
        show_menu_bar: true,
        ..Default::default()
    };

    new_config
        .save_to_store(app.handle())
        .expect("save to store");

    let store_path = aqvn_workspace_lib::config::store_path(app.handle()).expect("store path");
    assert!(store_path.exists(), "store file must exist on disk");

    let raw = std::fs::read_to_string(&store_path).expect("read store file");
    let json: serde_json::Value = serde_json::from_str(&raw).expect("parse store json");
    assert_eq!(json["server_url"], "https://persist-check.example");
    assert_eq!(json["rfc_enabled"], true);
    assert_eq!(json["rfc_password"], "secret-pass");
    assert_eq!(json["allowed_paths"][0], "/tmp/test-path");
    assert_eq!(json["connector_enabled"], false);
    assert_eq!(json["scope_files"], false);
    assert_eq!(json["scope_exec"], true);
    assert_eq!(json["close_to_tray"], false);
    assert_eq!(json["show_menu_bar"], true);

    // Load from store
    let loaded = AppConfig::load_from_store(app.handle()).expect("load from store");
    assert_eq!(loaded.server_url, "https://persist-check.example");
    assert_eq!(
        loaded.server_profiles,
        vec!["https://persist-check.example".to_string()]
    );
    assert!(loaded.rfc_enabled);
    assert_eq!(loaded.rfc_password, "secret-pass");
    assert_eq!(loaded.allowed_paths, vec!["/tmp/test-path".to_string()]);
    assert!(!loaded.connector_enabled);
    assert!(!loaded.scope_files);
    assert!(loaded.scope_exec);
    assert!(!loaded.close_to_tray);
    assert!(loaded.show_menu_bar);

    // Clean up
    let _ = std::fs::remove_file(store_path);
}

#[test]
fn load_from_file_applies_partial_settings() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "aqvn-settings-{}-{unique}.json",
        std::process::id()
    ));
    std::fs::write(&path, r#"{"rfc_enabled": true, "rfc_port": 55999}"#)
        .expect("write settings fixture");

    let loaded =
        aqvn_workspace_lib::config::load_from_file(&path).expect("settings file must parse");
    assert!(loaded.rfc_enabled);
    assert_eq!(loaded.rfc_port, 55999);
    // Missing keys keep their defaults instead of failing the parse.
    assert_eq!(loaded.ssh_port, DEFAULT_SSH_PORT);
    assert_eq!(loaded.allowed_paths, vec!["~/Workspace"]);
    assert!(loaded.connector_enabled);
    assert!(loaded.scope_files);
    assert!(loaded.scope_exec);
    assert!(loaded.scope_browser);
    assert!(loaded.scope_desktop);
    assert!(!loaded.show_menu_bar);
    assert!(loaded.task_chime_enabled);

    let _ = std::fs::remove_file(path);

    // Test partial settings with connector scopes and aliases
    let path2 = std::env::temp_dir().join(format!(
        "aqvn-settings-scopes-{}-{unique}.json",
        std::process::id()
    ));
    std::fs::write(
        &path2,
        r#"{"connector_enabled": false, "scope_files": false, "scope_code_execution": false, "scope_computer_use": false}"#,
    )
    .expect("write settings fixture 2");

    let loaded2 =
        aqvn_workspace_lib::config::load_from_file(&path2).expect("settings file 2 must parse");
    assert!(!loaded2.connector_enabled);
    assert!(!loaded2.scope_files);
    assert!(!loaded2.scope_exec);
    assert!(loaded2.scope_browser);
    assert!(!loaded2.scope_desktop);
    assert!(!loaded2.show_menu_bar);

    let _ = std::fs::remove_file(path2);

    // Test partial settings with show_menu_bar true
    let path3 = std::env::temp_dir().join(format!(
        "aqvn-settings-menu-{}-{unique}.json",
        std::process::id()
    ));
    std::fs::write(&path3, r#"{"show_menu_bar": true}"#).expect("write settings fixture 3");

    let loaded3 =
        aqvn_workspace_lib::config::load_from_file(&path3).expect("settings file 3 must parse");
    assert!(loaded3.show_menu_bar);

    let _ = std::fs::remove_file(path3);

    // Test partial settings with task_chime_enabled false
    let path4 = std::env::temp_dir().join(format!(
        "aqvn-settings-chime-{}-{unique}.json",
        std::process::id()
    ));
    std::fs::write(&path4, r#"{"task_chime_enabled": false}"#).expect("write settings fixture 4");

    let loaded4 =
        aqvn_workspace_lib::config::load_from_file(&path4).expect("settings file 4 must parse");
    assert!(!loaded4.task_chime_enabled);

    let _ = std::fs::remove_file(path4);
}
