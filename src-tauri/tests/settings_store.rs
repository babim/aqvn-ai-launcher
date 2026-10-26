//! Tests for configuration and store settings (VAL-SCAF-010/011/012).

use aqvn_workspace_lib::config::{
    AppConfig, ConfigWatcher, DEFAULT_ALLOWED_PATHS, DEFAULT_RFC_ENABLED, DEFAULT_RFC_PORT,
    DEFAULT_SERVER_URL, DEFAULT_SSH_PORT, STORE_FILENAME,
};
use tauri::Manager;

#[test]
fn default_config_matches_contract_requirements() {
    let conf = AppConfig::default();
    assert_eq!(conf.server_url, "https://a0.matmagoc.com");
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
        rfc_enabled: true,
        rfc_password: "secret-pass".to_string(),
        ssh_password: "ssh-pass".to_string(),
        allowed_paths: vec!["/tmp/workspace".to_string()],
        rfc_port: 55180,
        ssh_port: 55122,
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
        rfc_enabled: true,
        rfc_password: "secret-pass".to_string(),
        allowed_paths: vec!["/tmp/test-path".to_string()],
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

    // Load from store
    let loaded = AppConfig::load_from_store(app.handle()).expect("load from store");
    assert_eq!(loaded.server_url, "https://persist-check.example");
    assert!(loaded.rfc_enabled);
    assert_eq!(loaded.rfc_password, "secret-pass");
    assert_eq!(loaded.allowed_paths, vec!["/tmp/test-path".to_string()]);

    // Clean up
    let _ = std::fs::remove_file(store_path);
}
