# Spec delta: change6-ssh-server

## ADDED Requirements

### Requirement: change6-ssh-server binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `ssh-server`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change6-ssh-server-listener-only-when-toggle-on-port-config
- GIVEN the bridge script is invoked with `--feature-id ssh-server`
- WHEN the change scaffold is produced
- THEN the behavior "Listener only when toggle ON; port configurable + hot-applied" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change6-ssh-server-correct-password-pty-shell-requested-geo
- GIVEN the bridge script is invoked with `--feature-id ssh-server`
- WHEN the change scaffold is produced
- THEN the behavior "Correct password → PTY shell (requested geometry, cwd = first allowed path); wrong password → rejected" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change6-ssh-server-denylist-blocks-named-destructive-popula
- GIVEN the bridge script is invoked with `--feature-id ssh-server`
- WHEN the change scaffold is produced
- THEN the behavior "Denylist blocks named destructive population; benign passes; session survives refusal" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change6-ssh-server-security-invariant-denylist-is-best-effo
- GIVEN the bridge script is invoked with `--feature-id ssh-server`
- WHEN the change scaffold is produced
- THEN the behavior "Security invariant: denylist is best-effort secondary — never weakens auth/toggle" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
