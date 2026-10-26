//! AQVN Workspace core library.

pub mod config;
pub mod rfc;

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
            app.manage(config::ConfigWatcher::new(initial_config.clone()));

            // RFC bridge: binds only when the master toggle is enabled (default OFF).
            let rfc_config = initial_config;
            tauri::async_runtime::spawn(async move {
                if let Err(err) = rfc::server::run(rfc_config).await {
                    eprintln!("{err}");
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
