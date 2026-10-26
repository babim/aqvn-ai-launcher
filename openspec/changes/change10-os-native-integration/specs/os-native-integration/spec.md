# Spec delta: change10-os-native-integration

## ADDED Requirements

### Requirement: change10-os-native-integration binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `os-native-integration`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change10-os-native-integration-tray-visible-in-rig-screenshot-with-show
- GIVEN the bridge script is invoked with `--feature-id os-native-integration`
- WHEN the change scaffold is produced
- THEN the behavior "Tray visible in rig screenshot with Show/Hide + Quit menu; menu toggles window; Quit closes app + ports" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change10-os-native-integration-busy-idle-icon-switch-driven-by-chat-act
- GIVEN the bridge script is invoked with `--feature-id os-native-integration`
- WHEN the change scaffold is produced
- THEN the behavior "Busy/idle icon switch driven by chat activity" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change10-os-native-integration-alt-space-toggles-window-under-x11-xvfb-
- GIVEN the bridge script is invoked with `--feature-id os-native-integration`
- WHEN the change scaffold is produced
- THEN the behavior "Alt+Space toggles window under X11/Xvfb; grab logged; fallback documented" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change10-os-native-integration-notification-fires-on-task-completion-ev
- GIVEN the bridge script is invoked with `--feature-id os-native-integration`
- WHEN the change scaffold is produced
- THEN the behavior "Notification fires on task completion (evidence path recorded)" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change10-os-native-integration-external-link-opens-system-browser-webvi
- GIVEN the bridge script is invoked with `--feature-id os-native-integration`
- WHEN the change scaffold is produced
- THEN the behavior "External link opens system browser; webview stays on app pages" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
