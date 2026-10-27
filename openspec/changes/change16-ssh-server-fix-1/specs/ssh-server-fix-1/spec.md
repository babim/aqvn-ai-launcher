# Spec delta: change16-ssh-server-fix-1

## ADDED Requirements

### Requirement: change16-ssh-server-fix-1 binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `ssh-server-fix-1`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change16-ssh-server-fix-1-ssh-binds-0-0-0-0-55022-when-enabled-and
- GIVEN the bridge script is invoked with `--feature-id ssh-server-fix-1`
- WHEN the change scaffold is produced
- THEN the behavior "SSH binds 0.0.0.0:55022 when enabled AND ssh_password non-empty" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change16-ssh-server-fix-1-empty-ssh-password-no-bind-fail-closed-w
- GIVEN the bridge script is invoked with `--feature-id ssh-server-fix-1`
- WHEN the change scaffold is produced
- THEN the behavior "Empty ssh_password → no bind, fail-closed with log" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change16-ssh-server-fix-1-denylist-limitations-documented-in-repo-
- GIVEN the bridge script is invoked with `--feature-id ssh-server-fix-1`
- WHEN the change scaffold is produced
- THEN the behavior "Denylist limitations documented in repo docs + missionDir library" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change16-ssh-server-fix-1-full-cargo-test-clippy-fmt-green-whitela
- GIVEN the bridge script is invoked with `--feature-id ssh-server-fix-1`
- WHEN the change scaffold is produced
- THEN the behavior "Full cargo test + clippy + fmt green; whitelabel gate clean" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
