# Spec delta: change31-user-testing-validator-frontend-chat-experience

## ADDED Requirements

### Requirement: change31-user-testing-validator-frontend-chat-experience binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `user-testing-validator-frontend-chat-experience`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change31-user-testing-validator-frontend-chat-experience-testable-assertions-determined-from-fulf
- GIVEN the bridge script is invoked with `--feature-id user-testing-validator-frontend-chat-experience`
- WHEN the change scaffold is produced
- THEN the behavior "Testable assertions determined from fulfills mapping" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change31-user-testing-validator-frontend-chat-experience-environment-set-up-services-started-data
- GIVEN the bridge script is invoked with `--feature-id user-testing-validator-frontend-chat-experience`
- WHEN the change scaffold is produced
- THEN the behavior "Environment set up (services started, data seeded)" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change31-user-testing-validator-frontend-chat-experience-flow-validator-subagents-spawned-and-com
- GIVEN the bridge script is invoked with `--feature-id user-testing-validator-frontend-chat-experience`
- WHEN the change scaffold is produced
- THEN the behavior "Flow validator subagents spawned and completed" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change31-user-testing-validator-frontend-chat-experience-results-synthesized-validation-state-jso
- GIVEN the bridge script is invoked with `--feature-id user-testing-validator-frontend-chat-experience`
- WHEN the change scaffold is produced
- THEN the behavior "Results synthesized, validation-state.json updated" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
