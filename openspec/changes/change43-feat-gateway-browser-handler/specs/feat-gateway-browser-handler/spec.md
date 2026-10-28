# Spec delta: change43-feat-gateway-browser-handler

## ADDED Requirements

### Requirement: change43-feat-gateway-browser-handler binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `feat-gateway-browser-handler`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change43-feat-gateway-browser-handler-connects-to-local-chrome-cdp-at-http-127
- GIVEN the bridge script is invoked with `--feature-id feat-gateway-browser-handler`
- WHEN the change scaffold is produced
- THEN the behavior "Connects to local Chrome CDP at http://127.0.0.1:9222" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change43-feat-gateway-browser-handler-dispatches-open-navigate-click-type-scre
- GIVEN the bridge script is invoked with `--feature-id feat-gateway-browser-handler`
- WHEN the change scaffold is produced
- THEN the behavior "Dispatches open, navigate, click, type, screenshot, and evaluate actions" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change43-feat-gateway-browser-handler-handles-open-remote-debugging-by-launchi
- GIVEN the bridge script is invoked with `--feature-id feat-gateway-browser-handler`
- WHEN the change scaffold is produced
- THEN the behavior "Handles open_remote_debugging by launching browser with remote debugging" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
