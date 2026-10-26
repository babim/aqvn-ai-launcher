# Spec delta: change11-cross-area-integration

## ADDED Requirements

### Requirement: change11-cross-area-integration binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `cross-area-integration`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change11-cross-area-integration-journeys-a-b-pass-end-to-end-with-screen
- GIVEN the bridge script is invoked with `--feature-id cross-area-integration`
- WHEN the change scaffold is produced
- THEN the behavior "Journeys A+B pass end-to-end with screenshots/logs" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change11-cross-area-integration-restart-settings-listeners-restored-from
- GIVEN the bridge script is invoked with `--feature-id cross-area-integration`
- WHEN the change scaffold is produced
- THEN the behavior "Restart → settings + listeners restored from saved config" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change11-cross-area-integration-whitelabel-sweep-clean-on-all-six-named-
- GIVEN the bridge script is invoked with `--feature-id cross-area-integration`
- WHEN the change scaffold is produced
- THEN the behavior "Whitelabel sweep clean on all six named surfaces" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change11-cross-area-integration-toggle-off-closes-both-control-ports-cha
- GIVEN the bridge script is invoked with `--feature-id cross-area-integration`
- WHEN the change scaffold is produced
- THEN the behavior "Toggle OFF closes both control ports, chat unaffected; chat stable under concurrent RFC load" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
