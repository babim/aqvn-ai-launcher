# OS integration

Desktop-only wiring for the AQVN Workspace client. Android builds keep the chat
shell and proxy but compile out everything on this page.

## System tray

The tray is registered from Rust with `TrayIconBuilder` (`tray-icon` feature) and
a two-item menu:

| Item | Behaviour |
|---|---|
| **Show / Hide window** | Toggles the main window; showing also focuses it. |
| **Quit AQVN Workspace** | Exits the process, which closes the bridge proxy and the RFC/SSH listeners. |

**Linux never emits tray click events** (libappindicator/StatusNotifier has no
activation signal), so the menu items are the only interaction path by design —
there is no "click the icon to restore" behaviour on Linux.

The icon is also a state indicator, driven by chat activity:

| Chat state | Icon |
|---|---|
| `log_progress_active: false` | Idle — teal ring |
| `log_progress_active: true` | Busy — amber disc |

The chat UI invokes the `set_tray_busy` command whenever that flag changes, so
the icon tracks real streaming state from the server snapshot.

## Global hotkey (`Alt+Space`)

`Alt+Space` toggles the window through `tauri-plugin-global-shortcut`
(`global-hotkey 0.8`). Registration result is logged at startup, success or
failure with the reason.

**Wayland limitation:** `global-hotkey 0.8` grabs keys with X11 (`XGrabKey`) and
does not support Wayland — the portal-based backends are still upstream work in
progress. On a Wayland session the grab fails (logged) and `Alt+Space` does not
toggle the window.

**Fallback:** the tray menu **Show / Hide window** item always works on both
X11 and Wayland, and is the documented path when the global grab is
unavailable. The hotkey is also known to collide with the window-manager menu
binding on some X11 desktops; the same tray item covers that case.

Verified under `Xvfb :97` (X11, no window manager): grab succeeds, both
visibility transitions work. See `tools/tray_rig.sh`.

## Task-completion notification

`tauri-plugin-notification` (pure-Rust D-Bus backend, no system packages) sends
a native notification when a chat response finishes: the UI invokes
`notify_task_complete` on the `log_progress_active: true → false` transition,
with the last progress/result text as the body.

Runtime requirement: a session D-Bus daemon plus an
`org.freedesktop.Notifications` service (any desktop notification daemon). The
headless rig starts `dunst` for exactly this purpose and captures the
`Notify` call with `dbus-monitor`. If no notification daemon is available the
Rust log records the failed send and the permission state; that log is the
fallback evidence path.

## External links

Navigation policy for the main webview:

| Target | Behaviour |
|---|---|
| App pages (`tauri://localhost/...`, `about:`, `data:`, `blob:`) | Load in the webview. |
| Other `http(s)` URLs | Navigation cancelled; URL opened in the system browser via `tauri-plugin-shell`. |
| Any other scheme | Blocked, logged, webview stays on the current page. |

Chat messages render `http(s)` URLs as real links, so clicking one never
navigates the app webview away from the chat surface.

## Headless rig

`tools/tray_rig.sh` starts one isolated X11 + D-Bus session:

- `Xvfb :97` (configurable via `AQVN_RIG_DISPLAY`)
- a private D-Bus session bus and daemon
- `stalonetray` as the system-tray host (XEmbed fallback of libappindicator)
- `dunst` as the `org.freedesktop.Notifications` daemon
- the release binary with an isolated `XDG_CONFIG_HOME`, RFC/SSH enabled so
  the quit path can assert every listener closes

```
bash tools/tray_rig.sh start
bash tools/tray_rig.sh shot /tmp/tray-idle.png
bash tools/tray_rig.sh click-tray        # opens the menu
bash tools/tray_rig.sh shot /tmp/tray-menu.png
bash tools/tray_rig.sh stop
```

Evidence lives in `/tmp/aqvn-tray-rig/` (app log, bus address, screenshots).