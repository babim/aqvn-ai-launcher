# Spec delta: change39-feat-web-notification-bridge

## ADDED Requirements

### Requirement: change39-feat-web-notification-bridge binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `feat-web-notification-bridge`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change39-feat-web-notification-bridge-client-bridge-script-detects-task-comple
- GIVEN the bridge script is invoked with `--feature-id feat-web-notification-bridge`
- WHEN the change scaffold is produced
- THEN the behavior "Client bridge script detects task completion from WebSocket/DOM state" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change39-feat-web-notification-bridge-dispatches-tauri-ipc-notify-task-complet
- GIVEN the bridge script is invoked with `--feature-id feat-web-notification-bridge`
- WHEN the change scaffold is produced
- THEN the behavior "Dispatches Tauri IPC notify_task_complete when document.hidden === true" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change39-feat-web-notification-bridge-updates-system-tray-icon-state-between-i
- GIVEN the bridge script is invoked with `--feature-id feat-web-notification-bridge`
- WHEN the change scaffold is produced
- THEN the behavior "Updates system tray icon state between idle and busy" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
