# Spec delta: change13-taskfile-ci

## ADDED Requirements

### Requirement: change13-taskfile-ci binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `taskfile-ci`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change13-taskfile-ci-task-list-shows-build-linux-build-window
- GIVEN the bridge script is invoked with `--feature-id taskfile-ci`
- WHEN the change scaffold is produced
- THEN the behavior "task --list shows build:linux/build:windows/build:android; task build:linux exits 0 with deb+rpm" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change13-taskfile-ci-release-yml-tag-trigger-3-jobs-windows-s
- GIVEN the bridge script is invoked with `--feature-id taskfile-ci`
- WHEN the change scaffold is produced
- THEN the behavior "release.yml: tag trigger, 3 jobs, windows size gate 10MB step present" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change13-taskfile-ci-actionlint-yaml-parse-clean
- GIVEN the bridge script is invoked with `--feature-id taskfile-ci`
- WHEN the change scaffold is produced
- THEN the behavior "actionlint + YAML parse clean" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change13-taskfile-ci-ci-runtime-deferral-logged
- GIVEN the bridge script is invoked with `--feature-id taskfile-ci`
- WHEN the change scaffold is produced
- THEN the behavior "CI-runtime deferral logged" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
