# Spec delta: change30-scrutiny-validator-frontend-chat-experience

## ADDED Requirements

### Requirement: change30-scrutiny-validator-frontend-chat-experience binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `scrutiny-validator-frontend-chat-experience`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change30-scrutiny-validator-frontend-chat-experience-validators-pass-test-typecheck-lint
- GIVEN the bridge script is invoked with `--feature-id scrutiny-validator-frontend-chat-experience`
- WHEN the change scaffold is produced
- THEN the behavior "Validators pass (test, typecheck, lint)" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change30-scrutiny-validator-frontend-chat-experience-review-subagents-spawned-for-each-featur
- GIVEN the bridge script is invoked with `--feature-id scrutiny-validator-frontend-chat-experience`
- WHEN the change scaffold is produced
- THEN the behavior "Review subagents spawned for each feature" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change30-scrutiny-validator-frontend-chat-experience-findings-synthesized-into-scrutiny-repor
- GIVEN the bridge script is invoked with `--feature-id scrutiny-validator-frontend-chat-experience`
- WHEN the change scaffold is produced
- THEN the behavior "Findings synthesized into scrutiny report" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
