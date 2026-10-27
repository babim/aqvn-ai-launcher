# Spec delta: change17-rfc-ssh-e2e-fix-1

## ADDED Requirements

### Requirement: change17-rfc-ssh-e2e-fix-1 binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `rfc-ssh-e2e-fix-1`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change17-rfc-ssh-e2e-fix-1-a0-e2e-sh-runs-full-harness-e2e-exits-0-
- GIVEN the bridge script is invoked with `--feature-id rfc-ssh-e2e-fix-1`
- WHEN the change scaffold is produced
- THEN the behavior "a0_e2e.sh runs full harness E2E, exits 0, leaves zero orphans" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change17-rfc-ssh-e2e-fix-1-kill-paths-provenance-checked-pidfile-re
- GIVEN the bridge script is invoked with `--feature-id rfc-ssh-e2e-fix-1`
- WHEN the change scaffold is produced
- THEN the behavior "Kill paths provenance-checked; pidfile retained on kill failure" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change17-rfc-ssh-e2e-fix-1-port-5000-waits-to-close-like-55080-5502
- GIVEN the bridge script is invoked with `--feature-id rfc-ssh-e2e-fix-1`
- WHEN the change scaffold is produced
- THEN the behavior "Port 5000 waits to close like 55080/55022" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
