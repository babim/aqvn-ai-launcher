//! Native task-completion notification.
//!
//! The chat UI invokes this when `log_progress_active` transitions from true to
//! false. The plugin talks to `org.freedesktop.Notifications` over D-Bus.

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

#[tauri::command]
pub fn notify_task_complete(app: AppHandle, title: String, body: String) -> Result<(), String> {
    let permission = app
        .notification()
        .permission_state()
        .map(|state| format!("{state:?}"))
        .unwrap_or_else(|err| format!("unknown ({err})"));

    match app
        .notification()
        .builder()
        .title(&title)
        .body(&body)
        .show()
    {
        Ok(()) => {
            eprintln!("aqvn: notification sent (permission={permission}) title={title:?}");
            Ok(())
        }
        Err(err) => {
            eprintln!("aqvn: notification failed (permission={permission}): {err}");
            Err(err.to_string())
        }
    }
}
