# Spec delta: change33-user-testing-validator-backend-proxy

## ADDED Requirements

### Requirement: change33-user-testing-validator-backend-proxy binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `user-testing-validator-backend-proxy`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change33-user-testing-validator-backend-proxy-auto-skipped-via-g8-policy-for-all-progr
- GIVEN the bridge script is invoked with `--feature-id user-testing-validator-backend-proxy`
- WHEN the change scaffold is produced
- THEN the behavior "Auto-skipped via G8 policy for all-programmatic milestone" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
