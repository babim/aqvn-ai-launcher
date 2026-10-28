# Spec delta: change44-feat-gateway-desktop-handler

## ADDED Requirements

### Requirement: change44-feat-gateway-desktop-handler binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `feat-gateway-desktop-handler`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change44-feat-gateway-desktop-handler-captures-host-screen-or-active-window-on
- GIVEN the bridge script is invoked with `--feature-id feat-gateway-desktop-handler`
- WHEN the change scaffold is produced
- THEN the behavior "Captures host screen or active window on capture op" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change44-feat-gateway-desktop-handler-injects-mouse-movement-clicks-and-keystr
- GIVEN the bridge script is invoked with `--feature-id feat-gateway-desktop-handler`
- WHEN the change scaffold is produced
- THEN the behavior "Injects mouse movement, clicks, and keystrokes on click/type/key ops" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change44-feat-gateway-desktop-handler-replies-with-connector-computer-use-op-r
- GIVEN the bridge script is invoked with `--feature-id feat-gateway-desktop-handler`
- WHEN the change scaffold is produced
- THEN the behavior "Replies with connector_computer_use_op_result" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
