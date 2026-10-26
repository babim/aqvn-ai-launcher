//! AQVN Workspace core library.

/// Runs the desktop application.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .run(tauri::generate_context!())
        .expect("failed to run AQVN Workspace");
}
