# Proposal: change10-os-native-integration

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `os-native-integration` (milestone `os-integration`) to an openspec change. The feature description:

> Native OS integration. Tray (tray-icon feature): TrayIconBuilder with menu [Show/Hide, Quit]; menu-based show/hide (Linux emits no click events — by design); Quit exits app AND closes all servers (ports verified closed); icon switches busy/idle driven by chat state (UI→Rust command from log_progress_active). Global hotkey Alt+Space via tauri-plugin-global-shortcut (capability permissions allow-register/unregister/is-registered): toggle window visibility; LOG grab result (success or failure+reason); Wayland limitation documented in README/docs + tray-menu fallback always available. Notification: tauri-plugin-notification; chat task-completion event → native notification (D-Bus daemon in Xvfb rig if installable, else notify-rust/log evidence — record which path). External links: webview navigation to non-app http(s) blocked, opened via tauri-plugin-shell system browser. tools/tray_rig.sh: Xvfb + SNI/appindicator host (e.g. zypper install xfce4-panel or snixembed — pick lightest that works; sudo -n available) so tray is visually verifiable; if rig infeasible after honest attempt → log-evidence fallback + report, do NOT fake.

Mission: `c82a03df-dfc7-438f-b44b-9089dcc40a8f`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **10-001**: Tray visible in rig screenshot with Show/Hide + Quit menu; menu toggles window; Quit closes app + ports
- **10-002**: Busy/idle icon switch driven by chat activity
- **10-003**: Alt+Space toggles window under X11/Xvfb; grab logged; fallback documented
- **10-004**: Notification fires on task completion (evidence path recorded)
- **10-005**: External link opens system browser; webview stays on app pages

## Goals

- Bind feature `os-native-integration` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change10-os-native-integration/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-OS-001`
- `VAL-OS-002`
- `VAL-OS-003`
- `VAL-OS-004`
- `VAL-OS-005`
- `VAL-OS-006`
- `VAL-OS-007`
- `VAL-OS-008`
