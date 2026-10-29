# Spec delta: change24-frontend-focus-mode-and-hotkeys

## ADDED Requirements

### Requirement: change24-frontend-focus-mode-and-hotkeys binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `frontend-focus-mode-and-hotkeys`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change24-frontend-focus-mode-and-hotkeys-f11-and-ctrl-shift-f-toggle-focus-mode-c
- GIVEN the bridge script is invoked with `--feature-id frontend-focus-mode-and-hotkeys`
- WHEN the change scaffold is produced
- THEN the behavior "F11 and Ctrl+Shift+F toggle .focus-mode class on body" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change24-frontend-focus-mode-and-hotkeys-focus-mode-hides-sidebar-canvas-pane-and
- GIVEN the bridge script is invoked with `--feature-id frontend-focus-mode-and-hotkeys`
- WHEN the change scaffold is produced
- THEN the behavior ".focus-mode hides #sidebar, #canvas-pane, and splitters" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change24-frontend-focus-mode-and-hotkeys-focus-mode-centers-chat-pane-with-max-wi
- GIVEN the bridge script is invoked with `--feature-id frontend-focus-mode-and-hotkeys`
- WHEN the change scaffold is produced
- THEN the behavior ".focus-mode centers #chat-pane with max-width: 840px" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change24-frontend-focus-mode-and-hotkeys-input-message-input-is-focused-immediate
- GIVEN the bridge script is invoked with `--feature-id frontend-focus-mode-and-hotkeys`
- WHEN the change scaffold is produced
- THEN the behavior "Input #message-input is focused immediately upon entering focus mode" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
