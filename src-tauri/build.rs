fn main() {
    let attributes =
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[
            "set_tray_busy",
            "notify_task_complete",
            "open_settings",
            "open_help",
            "open_about",
            "close_current_window",
            "show_context_menu",
            "toggle_fullscreen",
            "clear_disk_cache",
            "reconnect_network",
        ]));
    tauri_build::try_build(attributes).expect("failed to run tauri-build");
}
