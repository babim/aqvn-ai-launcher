//! Window geometry persistence and restoration tests (VAL-UX-001).

use aqvn_workspace_lib::config::{
    apply_window_state, persist_window_geometry, AppConfig, ConfigWatcher, WindowState,
};

#[test]
fn test_window_geometry_persistence_and_restore() {
    let app = tauri::test::mock_builder()
        .plugin(tauri_plugin_store::Builder::new().build())
        .build(tauri::generate_context!())
        .expect("build mock app");

    // Default configuration has no window_state yet (None)
    let default_conf = AppConfig::default();
    assert_eq!(default_conf.window_state, None);

    // Initial window state tracking and persistence
    let initial_geom = WindowState {
        width: 1280.0,
        height: 800.0,
        x: 150.0,
        y: 100.0,
        is_maximized: false,
    };

    // Persist geometry directly through store
    persist_window_geometry(app.handle(), initial_geom).expect("persist window geometry");

    // Restore on startup via load_from_store
    let loaded = AppConfig::load_from_store(app.handle()).expect("load from store");
    assert_eq!(loaded.window_state, Some(initial_geom));

    // Verify window config restoration
    let window_config = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == "main")
        .expect("main window config in tauri.conf.json");

    let mut configured = window_config.clone();
    apply_window_state(&mut configured, &initial_geom);

    assert_eq!(configured.width, 1280.0);
    assert_eq!(configured.height, 800.0);
    assert_eq!(configured.x, Some(150.0));
    assert_eq!(configured.y, Some(100.0));
    assert!(!configured.center);
    assert!(!configured.maximized);

    // Test maximized state persistence and restore
    let maximized_geom = WindowState {
        width: 1280.0,
        height: 800.0,
        x: 150.0,
        y: 100.0,
        is_maximized: true,
    };
    persist_window_geometry(app.handle(), maximized_geom).expect("persist maximized geometry");

    let loaded_maximized = AppConfig::load_from_store(app.handle()).expect("load from store");
    assert_eq!(loaded_maximized.window_state, Some(maximized_geom));

    apply_window_state(&mut configured, &maximized_geom);
    assert!(configured.maximized);
}

#[test]
fn test_window_state_serde_roundtrip() {
    let ws = WindowState {
        width: 1440.0,
        height: 900.0,
        x: 200.0,
        y: 120.0,
        is_maximized: true,
    };
    let json = serde_json::to_string(&ws).expect("serialize WindowState");
    let deserialized: WindowState = serde_json::from_str(&json).expect("deserialize WindowState");
    assert_eq!(ws, deserialized);

    let conf = AppConfig {
        window_state: Some(ws),
        ..Default::default()
    };
    let conf_json = serde_json::to_string(&conf).expect("serialize AppConfig");
    let conf_deserialized: AppConfig =
        serde_json::from_str(&conf_json).expect("deserialize AppConfig");
    assert_eq!(conf.window_state, conf_deserialized.window_state);
}

#[test]
fn test_config_watcher_window_state_update() {
    let initial = AppConfig::default();
    let watcher = ConfigWatcher::new(initial);
    let mut rx = watcher.subscribe();
    assert_eq!(watcher.current().window_state, None);

    let mut updated = watcher.current();
    let ws = WindowState::new(1024.0, 768.0, 50.0, 50.0, false);
    updated.window_state = Some(ws);
    watcher.update(updated.clone());

    assert_eq!(watcher.current().window_state, Some(ws));
    assert_eq!(*rx.borrow_and_update(), updated);
}
