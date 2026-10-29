# Spec delta: change42-feat-gateway-exec-handler

## ADDED Requirements

### Requirement: change42-feat-gateway-exec-handler binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `feat-gateway-exec-handler`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change42-feat-gateway-exec-handler-handles-connector-exec-op-with-runtime-t
- GIVEN the bridge script is invoked with `--feature-id feat-gateway-exec-handler`
- WHEN the change scaffold is produced
- THEN the behavior "Handles connector_exec_op with runtime=terminal" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change42-feat-gateway-exec-handler-executes-shell-commands-inside-stateful-
- GIVEN the bridge script is invoked with `--feature-id feat-gateway-exec-handler`
- WHEN the change scaffold is produced
- THEN the behavior "Executes shell commands inside stateful PTY session" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change42-feat-gateway-exec-handler-streams-output-back-to-agent-via-connect
- GIVEN the bridge script is invoked with `--feature-id feat-gateway-exec-handler`
- WHEN the change scaffold is produced
- THEN the behavior "Streams output back to agent via connector_exec_op_result" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
