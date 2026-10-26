//! AQVN Workspace core library.

pub mod config;
pub mod proxy;
pub mod rfc;
pub mod ssh;

use tauri::{Manager, WebviewWindowBuilder};

/// Runs the desktop application.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            let initial_config =
                config::AppConfig::load_from_store(app.handle()).unwrap_or_default();
            let watcher = config::ConfigWatcher::new(initial_config.clone());
            app.manage(watcher.clone());

            // Local bridge proxy: loopback-only, kernel-chosen port. Binds before
            // the window exists so the resolved port can be injected into the
            // page as `window.__AQVN_PROXY__`.
            let (proxy_listener, proxy_state, proxy_info) =
                tauri::async_runtime::block_on(proxy::server::bind(&initial_config.server_url))?;
            app.manage(proxy_info.clone());

            let serve_state = proxy_state.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) = proxy::server::serve(proxy_listener, serve_state).await {
                    eprintln!("Bridge proxy error: {err}");
                }
            });
            let watch_state = proxy_state.clone();
            let watch_config = watcher.clone();
            tauri::async_runtime::spawn(async move {
                proxy::server::watch_target(watch_config, watch_state).await;
            });

            // RFC bridge: binds only when the master toggle is enabled (default OFF)
            // and restarts its listener on any settings change (hot reload).
            let supervisor = watcher.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) = rfc::server::run_supervised(supervisor).await {
                    eprintln!("{err}");
                }
            });

            // SSH bridge: binds only when the master toggle is enabled and password is set
            // (default OFF) and restarts its listener on any settings change (hot reload).
            let ssh_supervisor = watcher.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) = ssh::server::run_supervised(ssh_supervisor).await {
                    eprintln!("{err}");
                }
            });

            // The settings UI writes through the store plugin; poll the settings file so
            // those writes reach the watcher without an app restart.
            let poll_handle = app.handle().clone();
            let poll_watcher = watcher;
            let store_file = config::store_path(&poll_handle).ok();
            tauri::async_runtime::spawn(async move {
                let mut last = initial_config;
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    let next = store_file
                        .as_deref()
                        .and_then(config::load_from_file)
                        .or_else(|| config::AppConfig::load_from_store(&poll_handle).ok());
                    let Some(next) = next else { continue };
                    if next != last {
                        last = next.clone();
                        poll_watcher.update(next);
                    }
                }
            });

            let window_config = app
                .config()
                .app
                .windows
                .iter()
                .find(|w| w.label == "main")
                .expect("main window config not found in tauri.conf.json");

            WebviewWindowBuilder::from_config(app.handle(), window_config)?
                .initialization_script(proxy::server::init_script(proxy_info.port))
                .on_document_title_changed(|window, title| {
                    // Rust-side sync needs no `core:window:allow-set-title`.
                    let _ = window.set_title(&title);
                })
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run AQVN Workspace");
}
