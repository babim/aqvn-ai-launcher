# Spec delta: change5-rfc-sandbox-handlers

## ADDED Requirements

### Requirement: change5-rfc-sandbox-handlers binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `rfc-sandbox-handlers`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change5-rfc-sandbox-handlers-every-whitelist-fn-returns-exact-spec-sh
- GIVEN the bridge script is invoked with `--feature-id rfc-sandbox-handlers`
- WHEN the change scaffold is produced
- THEN the behavior "Every whitelist fn returns exact spec shape (integration tests per family)" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change5-rfc-sandbox-handlers-sandbox-escape-read-write-delete-travers
- GIVEN the bridge script is invoked with `--feature-id rfc-sandbox-handlers`
- WHEN the change scaffold is produced
- THEN the behavior "Sandbox: escape read/write/delete, ../ traversal, symlink escape → 403; /a0/ prefix maps into sandbox root" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change5-rfc-sandbox-handlers-provide-root-password-round-trips-python
- GIVEN the bridge script is invoked with `--feature-id rfc-sandbox-handlers`
- WHEN the change scaffold is produced
- THEN the behavior "_provide_root_password round-trips: python harness crypto decrypts to exact ssh_password" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change5-rfc-sandbox-handlers-make-dir-jailed-like-all-path-ops
- GIVEN the bridge script is invoked with `--feature-id rfc-sandbox-handlers`
- WHEN the change scaffold is produced
- THEN the behavior "make_dir jailed like all path ops" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change5-rfc-sandbox-handlers-settings-hot-reload-works-password-paths
- GIVEN the bridge script is invoked with `--feature-id rfc-sandbox-handlers`
- WHEN the change scaffold is produced
- THEN the behavior "Settings hot-reload works (password/paths/port/toggle), process PID unchanged" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
