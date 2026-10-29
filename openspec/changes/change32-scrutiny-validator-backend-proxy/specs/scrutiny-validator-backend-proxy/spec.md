# Spec delta: change32-scrutiny-validator-backend-proxy

## ADDED Requirements

### Requirement: change32-scrutiny-validator-backend-proxy binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `scrutiny-validator-backend-proxy`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change32-scrutiny-validator-backend-proxy-validators-pass-test-typecheck-lint
- GIVEN the bridge script is invoked with `--feature-id scrutiny-validator-backend-proxy`
- WHEN the change scaffold is produced
- THEN the behavior "Validators pass (test, typecheck, lint)" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change32-scrutiny-validator-backend-proxy-review-subagents-spawned-for-each-featur
- GIVEN the bridge script is invoked with `--feature-id scrutiny-validator-backend-proxy`
- WHEN the change scaffold is produced
- THEN the behavior "Review subagents spawned for each feature" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change32-scrutiny-validator-backend-proxy-findings-synthesized-into-scrutiny-repor
- GIVEN the bridge script is invoked with `--feature-id scrutiny-validator-backend-proxy`
- WHEN the change scaffold is produced
- THEN the behavior "Findings synthesized into scrutiny report" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
