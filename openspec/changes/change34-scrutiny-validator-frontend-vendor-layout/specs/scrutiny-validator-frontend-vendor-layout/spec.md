# Spec delta: change34-scrutiny-validator-frontend-vendor-layout

## ADDED Requirements

### Requirement: change34-scrutiny-validator-frontend-vendor-layout binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `scrutiny-validator-frontend-vendor-layout`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change34-scrutiny-validator-frontend-vendor-layout-validators-pass-test-typecheck-lint
- GIVEN the bridge script is invoked with `--feature-id scrutiny-validator-frontend-vendor-layout`
- WHEN the change scaffold is produced
- THEN the behavior "Validators pass (test, typecheck, lint)" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change34-scrutiny-validator-frontend-vendor-layout-review-subagents-spawned-for-each-featur
- GIVEN the bridge script is invoked with `--feature-id scrutiny-validator-frontend-vendor-layout`
- WHEN the change scaffold is produced
- THEN the behavior "Review subagents spawned for each feature" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change34-scrutiny-validator-frontend-vendor-layout-findings-synthesized-into-scrutiny-repor
- GIVEN the bridge script is invoked with `--feature-id scrutiny-validator-frontend-vendor-layout`
- WHEN the change scaffold is produced
- THEN the behavior "Findings synthesized into scrutiny report" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
