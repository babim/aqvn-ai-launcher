//! Window geometry persistence and restoration tests (VAL-UX-001).

use aqvn_workspace_lib::config::{
    apply_window_state, persist_window_geometry, AppConfig, ConfigWatcher, GeometryDebouncer,
    WindowState,
};
use tauri_plugin_store::StoreExt;

#[test]
fn test_window_geometry_persistence_and_restore() {
    let app = tauri::test::mock_builder()
        .plugin(tauri_plugin_store::Builder::new().build())
        .build(tauri::generate_context!())
        .expect("build mock app");

    if let Ok(store_path) = aqvn_workspace_lib::config::store_path(app.handle()) {
        let _ = std::fs::remove_file(&store_path);
    }
    if let Ok(store) = app
        .handle()
        .store(aqvn_workspace_lib::config::STORE_FILENAME)
    {
        let _ = store.delete("window_state");
    }

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

#[tokio::test]
async fn test_window_geometry_debounced_persistence() {
    let app = tauri::test::mock_builder()
        .plugin(tauri_plugin_store::Builder::new().build())
        .build(tauri::generate_context!())
        .expect("build mock app");

    if let Ok(store_path) = aqvn_workspace_lib::config::store_path(app.handle()) {
        let _ = std::fs::remove_file(&store_path);
    }
    // Also clear memory store if already loaded
    if let Ok(store) = app
        .handle()
        .store(aqvn_workspace_lib::config::STORE_FILENAME)
    {
        let _ = store.delete("window_state");
    }

    let debouncer = GeometryDebouncer::new(app.handle().clone());

    // Initially no window state stored
    let initial = AppConfig::load_from_store(app.handle()).expect("load store");
    assert_eq!(initial.window_state, None);

    // Rapid resize/move events
    for i in 1..=10 {
        debouncer.debounce(WindowState::new(
            1000.0 + (i as f64) * 10.0,
            700.0 + (i as f64) * 5.0,
            50.0 + (i as f64),
            50.0 + (i as f64),
            false,
        ));
    }

    // Immediately check: must NOT have written synchronously to disk
    let immediate = AppConfig::load_from_store(app.handle()).expect("load store");
    assert_eq!(
        immediate.window_state, None,
        "rapid resize events must not synchronously write to disk"
    );

    // Wait 200ms and send final resting state
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let resting_geom = WindowState::new(1280.0, 800.0, 100.0, 100.0, false);
    debouncer.debounce(resting_geom);

    // Wait 250ms (less than 500ms since last update): still not persisted
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    let mid = AppConfig::load_from_store(app.handle()).expect("load store");
    assert_eq!(
        mid.window_state, None,
        "geometry must not persist before 500ms debounce settle time"
    );

    // Wait another 350ms (total > 500ms since resting_geom was debounced)
    tokio::time::sleep(std::time::Duration::from_millis(350)).await;
    let settled = AppConfig::load_from_store(app.handle()).expect("load store");
    assert_eq!(
        settled.window_state,
        Some(resting_geom),
        "resting geometry must persist after 500ms delay"
    );

    // Verify flush immediately writes pending state
    let flush_geom = WindowState::new(1920.0, 1080.0, 0.0, 0.0, true);
    debouncer.debounce(flush_geom);
    debouncer.flush_wait().await;
    let flushed = AppConfig::load_from_store(app.handle()).expect("load store");
    assert_eq!(
        flushed.window_state,
        Some(flush_geom),
        "flush must immediately persist pending geometry"
    );
}

#[tokio::test]
async fn test_geometry_debouncer_coalesces_rapid_calls() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let write_count = Arc::new(AtomicUsize::new(0));
    let last_written = Arc::new(std::sync::Mutex::new(None));

    let wc = write_count.clone();
    let lw = last_written.clone();
    let debouncer =
        GeometryDebouncer::new_with_callback(std::time::Duration::from_millis(100), move |state| {
            wc.fetch_add(1, Ordering::SeqCst);
            *lw.lock().unwrap() = Some(state);
        });

    for i in 1..=20 {
        debouncer.debounce(WindowState::new(100.0 + (i as f64), 200.0, 0.0, 0.0, false));
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }

    // Immediately after rapid burst (< 100ms since last), 0 writes
    assert_eq!(
        write_count.load(Ordering::SeqCst),
        0,
        "must coalesce rapid updates without intermediate writes"
    );

    // Wait for debounce period (150ms > 100ms)
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    assert_eq!(
        write_count.load(Ordering::SeqCst),
        1,
        "exactly 1 write must be executed for the burst"
    );
    let final_state = *last_written.lock().unwrap();
    assert_eq!(
        final_state,
        Some(WindowState::new(120.0, 200.0, 0.0, 0.0, false)),
        "the final state must be the resting state"
    );
}
