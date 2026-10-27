# Spec delta: change15-rfc-server-core-fix-1

## ADDED Requirements

### Requirement: change15-rfc-server-core-fix-1 binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `rfc-server-core-fix-1`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change15-rfc-server-core-fix-1-rfc-binds-0-0-0-0-55080-when-enabled-and
- GIVEN the bridge script is invoked with `--feature-id rfc-server-core-fix-1`
- WHEN the change scaffold is produced
- THEN the behavior "RFC binds 0.0.0.0:55080 when enabled AND rfc_password non-empty" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change15-rfc-server-core-fix-1-empty-rfc-password-no-bind-fail-closed-w
- GIVEN the bridge script is invoked with `--feature-id rfc-server-core-fix-1`
- WHEN the change scaffold is produced
- THEN the behavior "Empty rfc_password → no bind, fail-closed with log" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change15-rfc-server-core-fix-1-empty-allowed-paths-sandbox-deny-all-mul
- GIVEN the bridge script is invoked with `--feature-id rfc-server-core-fix-1`
- WHEN the change scaffold is produced
- THEN the behavior "Empty allowed_paths → sandbox deny-all; multi-entry allowed_paths all honored" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change15-rfc-server-core-fix-1-full-cargo-test-clippy-fmt-green-whitela
- GIVEN the bridge script is invoked with `--feature-id rfc-server-core-fix-1`
- WHEN the change scaffold is produced
- THEN the behavior "Full cargo test + clippy + fmt green; whitelabel gate clean" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
