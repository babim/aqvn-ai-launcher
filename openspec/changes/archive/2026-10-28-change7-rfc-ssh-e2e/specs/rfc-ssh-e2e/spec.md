# Spec delta: change7-rfc-ssh-e2e

## ADDED Requirements

### Requirement: change7-rfc-ssh-e2e binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `rfc-ssh-e2e`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change7-rfc-ssh-e2e-spec-client-gate-3-correct-hash-file-con
- GIVEN the bridge script is invoked with `--feature-id rfc-ssh-e2e`
- WHEN the change scaffold is produced
- THEN the behavior "spec_client Gate 3: correct hash → file content; wrong hash → 403; outside sandbox → 403" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change7-rfc-ssh-e2e-real-a0-call-rfc-harness-venv-succeeds-a
- GIVEN the bridge script is invoked with `--feature-id rfc-ssh-e2e`
- WHEN the change scaffold is produced
- THEN the behavior "Real A0 call_rfc (harness venv) succeeds against our server — wire-compatible proof" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change7-rfc-ssh-e2e-ssh-exchange-decrypts-to-exact-ssh-passw
- GIVEN the bridge script is invoked with `--feature-id rfc-ssh-e2e`
- WHEN the change scaffold is produced
- THEN the behavior "SSH exchange decrypts to exact ssh_password; paramiko session runs command end-to-end" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change7-rfc-ssh-e2e-harness-left-stopped-no-orphans-ports-fr
- GIVEN the bridge script is invoked with `--feature-id rfc-ssh-e2e`
- WHEN the change scaffold is produced
- THEN the behavior "Harness left stopped, no orphans, ports free" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
