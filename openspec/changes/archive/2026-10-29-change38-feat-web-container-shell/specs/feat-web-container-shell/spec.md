# Spec delta: change38-feat-web-container-shell

## ADDED Requirements

### Requirement: change38-feat-web-container-shell binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `feat-web-container-shell`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change38-feat-web-container-shell-webview-displays-the-complete-upstream-w
- GIVEN the bridge script is invoked with `--feature-id feat-web-container-shell`
- WHEN the change scaffold is produced
- THEN the behavior "Webview displays the complete upstream web UI (Sidebar, Chat, DAG Canvas, Settings)" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change38-feat-web-container-shell-internal-app-routes-and-frames-navigate-
- GIVEN the bridge script is invoked with `--feature-id feat-web-container-shell`
- WHEN the change scaffold is produced
- THEN the behavior "Internal app routes and frames navigate inside the webview without opening external browser" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change38-feat-web-container-shell-external-hyperlinks-open-in-system-defau
- GIVEN the bridge script is invoked with `--feature-id feat-web-container-shell`
- WHEN the change scaffold is produced
- THEN the behavior "External hyperlinks open in system default browser via tauri-plugin-shell" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change38-feat-web-container-shell-os-integration-navigation-tests-pass-cle
- GIVEN the bridge script is invoked with `--feature-id feat-web-container-shell`
- WHEN the change scaffold is produced
- THEN the behavior "os_integration navigation tests pass cleanly" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
