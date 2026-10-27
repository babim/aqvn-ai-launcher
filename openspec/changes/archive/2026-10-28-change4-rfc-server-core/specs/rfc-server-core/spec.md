# Spec delta: change4-rfc-server-core

## ADDED Requirements

### Requirement: change4-rfc-server-core binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `rfc-server-core`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change4-rfc-server-core-worked-example-request-spec-7-returns-20
- GIVEN the bridge script is invoked with `--feature-id rfc-server-core`
- WHEN the change scaffold is produced
- THEN the behavior "Worked-example request (spec §7) returns 200 + bare JSON" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change4-rfc-server-core-wrong-hash-missing-hash-403-malformed-rf
- GIVEN the bridge script is invoked with `--feature-id rfc-server-core`
- WHEN the change scaffold is produced
- THEN the behavior "Wrong hash / missing hash → 403; malformed rfc_input → 500; GET → 405; unknown route → 404 (exact bodies)" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change4-rfc-server-core-unknown-module-function-403-whitelist-en
- GIVEN the bridge script is invoked with `--feature-id rfc-server-core`
- WHEN the change scaffold is produced
- THEN the behavior "Unknown module/function → 403 (whitelist enforced)" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change4-rfc-server-core-toggle-off-port-55080-closed-ss-tln-evid
- GIVEN the bridge script is invoked with `--feature-id rfc-server-core`
- WHEN the change scaffold is produced
- THEN the behavior "Toggle OFF → port 55080 closed (ss -tln evidence)" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change4-rfc-server-core-hmac-constant-time-code-review-criterion
- GIVEN the bridge script is invoked with `--feature-id rfc-server-core`
- WHEN the change scaffold is produced
- THEN the behavior "HMAC constant-time (code review criterion)" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
