# Spec delta: change41-feat-gateway-file-handler

## ADDED Requirements

### Requirement: change41-feat-gateway-file-handler binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `feat-gateway-file-handler`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change41-feat-gateway-file-handler-handles-connector-file-op-for-read-write
- GIVEN the bridge script is invoked with `--feature-id feat-gateway-file-handler`
- WHEN the change scaffold is produced
- THEN the behavior "Handles connector_file_op for read, write, patch, and list" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change41-feat-gateway-file-handler-rejects-path-traversal-or-symlink-escape
- GIVEN the bridge script is invoked with `--feature-id feat-gateway-file-handler`
- WHEN the change scaffold is produced
- THEN the behavior "Rejects path traversal or symlink escapes outside sandbox" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change41-feat-gateway-file-handler-replies-with-connector-file-op-result-ov
- GIVEN the bridge script is invoked with `--feature-id feat-gateway-file-handler`
- WHEN the change scaffold is produced
- THEN the behavior "Replies with connector_file_op_result over WebSocket" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
