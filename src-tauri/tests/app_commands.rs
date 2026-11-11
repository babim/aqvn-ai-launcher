//!
//! Covers validation contract assertion VAL-RES-006:
//! Invoking the `reconnect_network` Tauri command triggers an immediate
//! reconnection of the gateway client and refreshes the loopback proxy state.

use aqvn_workspace_lib::app::reconnect_network;
use aqvn_workspace_lib::config::{AppConfig, ConfigWatcher};
use aqvn_workspace_lib::connector::client::{run_supervised_with_reconnect, GatewayManager};
use aqvn_workspace_lib::proxy::cache::AssetCache;
use aqvn_workspace_lib::proxy::server::ProxyState;
use aqvn_workspace_lib::proxy::upstream::Upstream;
use std::sync::Arc;
use std::time::Duration;
use tauri::Manager;

#[tokio::test]
async fn test_reconnect_network_aborts_active_gateway_and_reconnects() {
    let (gw_mgr, mut rx_reconnect) = GatewayManager::new();

    // Spawn a simulated active task that can be cancelled via JoinHandle::abort
    let task = tokio::spawn(async {
        tokio::time::sleep(Duration::from_secs(60)).await;
    });
    let abort_handle = task.abort_handle();

    let (shutdown_tx, _shutdown_rx) = tokio::sync::watch::channel(false);
    let (_handshake_tx, handshake_rx) = tokio::sync::watch::channel(false);
    let connected = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let handle = aqvn_workspace_lib::connector::client::GatewayHandle::new_for_test(
        connected,
        handshake_rx,
        shutdown_tx,
    );
    gw_mgr.set_session(handle, abort_handle.clone()).await;

    assert!(!task.is_finished(), "active task must be running");
    assert_eq!(gw_mgr.reconnect_count(), 0);

    let app = tauri::test::mock_builder()
        .plugin(tauri_plugin_store::Builder::new().build())
        .build(tauri::generate_context!())
        .expect("build mock app");

    app.manage(gw_mgr.clone());

    let res = reconnect_network(app.handle().clone()).await;
    assert!(res.is_ok(), "reconnect_network must return Ok(())");

    // Reconnect signal must be received
    let recv = rx_reconnect.try_recv();
    assert!(
        recv.is_ok(),
        "reconnect signal must be received by supervisor"
    );

    // Task must be aborted immediately
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(
        task.is_finished(),
        "invoking reconnect_network must abort active gateway task handle"
    );

    // Reconnect count must increment
    assert_eq!(
        gw_mgr.reconnect_count(),
        1,
        "reconnect count must increment"
    );
}

#[tokio::test]
async fn test_reconnect_network_refreshes_proxy_state() {
    let temp_cache_dir = std::env::temp_dir().join(format!(
        "aqvn-test-app-cmd-cache-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let cache = Arc::new(AssetCache::with_max_disk(
        temp_cache_dir.clone(),
        1024 * 1024,
    ));
    let target = "http://localhost:55088";
    cache
        .put(target, "/test.css", Some("text/css"), b"body{color:red;}")
        .await;
    assert!(cache.get(target, "/test.css").await.is_some());

    let upstream = Upstream::new("http://localhost:55088").expect("upstream");
    upstream.reset_jar().await;

    let proxy_state = ProxyState::with_cache(upstream, cache.clone());

    let app = tauri::test::mock_builder()
        .plugin(tauri_plugin_store::Builder::new().build())
        .build(tauri::generate_context!())
        .expect("build mock app");

    app.manage(proxy_state.clone());
    app.manage(cache.clone());

    let res = reconnect_network(app.handle().clone()).await;
    assert!(res.is_ok(), "reconnect_network must return Ok(())");

    // Asset cache must be cleared by proxy refresh
    assert!(
        cache.get(target, "/test.css").await.is_none(),
        "asset cache must be cleared after reconnect_network"
    );

    let _ = std::fs::remove_dir_all(&temp_cache_dir);
}

#[tokio::test]
async fn test_reconnect_network_full_mock_app_coordination() {
    let (gw_mgr, mut rx_reconnect) = GatewayManager::new();

    let task = tokio::spawn(async {
        tokio::time::sleep(Duration::from_secs(60)).await;
    });
    let abort_handle = task.abort_handle();

    let (shutdown_tx, _shutdown_rx) = tokio::sync::watch::channel(false);
    let (_handshake_tx, handshake_rx) = tokio::sync::watch::channel(false);
    let connected = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let handle = aqvn_workspace_lib::connector::client::GatewayHandle::new_for_test(
        connected,
        handshake_rx,
        shutdown_tx,
    );
    gw_mgr.set_session(handle, abort_handle.clone()).await;

    let temp_cache_dir = std::env::temp_dir().join(format!(
        "aqvn-test-app-cmd-full-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let cache = Arc::new(AssetCache::with_max_disk(
        temp_cache_dir.clone(),
        1024 * 1024,
    ));
    cache
        .put(
            "http://localhost:55088",
            "/app.js",
            Some("application/javascript"),
            b"console.log(1);",
        )
        .await;

    let upstream = Upstream::new("http://localhost:55088").expect("upstream");
    let proxy_state = ProxyState::with_cache(upstream, cache.clone());
    let watcher = ConfigWatcher::new(AppConfig::default());

    let app = tauri::test::mock_builder()
        .plugin(tauri_plugin_store::Builder::new().build())
        .build(tauri::generate_context!())
        .expect("build mock app");

    app.manage(gw_mgr.clone());
    app.manage(proxy_state.clone());
    app.manage(cache.clone());
    app.manage(watcher);

    let res = reconnect_network(app.handle().clone()).await;
    assert!(res.is_ok(), "reconnect_network must return Ok(())");

    // Both gateway abort & restart and proxy refresh occurred
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(task.is_finished(), "active task must be aborted");
    assert!(rx_reconnect.try_recv().is_ok());
    assert_eq!(gw_mgr.reconnect_count(), 1);
    assert!(cache
        .get("http://localhost:55088", "/app.js")
        .await
        .is_none());

    let _ = std::fs::remove_dir_all(&temp_cache_dir);
}

#[tokio::test]
async fn test_reconnect_network_without_managed_state_is_safe_noop() {
    let app = tauri::test::mock_builder()
        .plugin(tauri_plugin_store::Builder::new().build())
        .build(tauri::generate_context!())
        .expect("build mock app");

    let res = reconnect_network(app.handle().clone()).await;
    assert!(
        res.is_ok(),
        "reconnect_network without managed state must succeed gracefully"
    );
}

#[tokio::test]
async fn test_run_supervised_with_reconnect_handles_signal() {
    let (gw_mgr, rx_reconnect) = GatewayManager::new();
    let watcher = ConfigWatcher::new(AppConfig {
        connector_enabled: false,
        server_url: "http://127.0.0.1:9".to_string(),
        ..Default::default()
    });

    let sup_watcher = watcher.clone();
    let sup_mgr = gw_mgr.clone();
    let sup_task = tokio::spawn(async move {
        let _ = run_supervised_with_reconnect(sup_watcher, sup_mgr, rx_reconnect).await;
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Trigger reconnect
    let res = gw_mgr.abort_and_reconnect().await;
    assert!(res.is_ok());
    assert_eq!(gw_mgr.reconnect_count(), 1);

    sup_task.abort();
}
