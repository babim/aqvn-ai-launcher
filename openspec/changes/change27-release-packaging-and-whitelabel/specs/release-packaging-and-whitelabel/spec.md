# Spec delta: change27-release-packaging-and-whitelabel

## ADDED Requirements

### Requirement: change27-release-packaging-and-whitelabel binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `release-packaging-and-whitelabel`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change27-release-packaging-and-whitelabel-cargo-toml-and-tauri-conf-json-specify-v
- GIVEN the bridge script is invoked with `--feature-id release-packaging-and-whitelabel`
- WHEN the change scaffold is produced
- THEN the behavior "Cargo.toml and tauri.conf.json specify version = '0.1.6'" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change27-release-packaging-and-whitelabel-changelog-md-documents-0-1-6-release-fea
- GIVEN the bridge script is invoked with `--feature-id release-packaging-and-whitelabel`
- WHEN the change scaffold is produced
- THEN the behavior "CHANGELOG.md documents [0.1.6] release features" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change27-release-packaging-and-whitelabel-zero-occurrences-of-agent-zero-or-agent-
- GIVEN the bridge script is invoked with `--feature-id release-packaging-and-whitelabel`
- WHEN the change scaffold is produced
- THEN the behavior "Zero occurrences of 'Agent Zero' or 'agent-zero' across the repository" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change27-release-packaging-and-whitelabel-cd-src-tauri-cargo-test-passes-100-125-t
- GIVEN the bridge script is invoked with `--feature-id release-packaging-and-whitelabel`
- WHEN the change scaffold is produced
- THEN the behavior "cd src-tauri && cargo test passes 100% (125+ tests)" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change27-release-packaging-and-whitelabel-cargo-clippy-all-targets-d-warnings-and-
- GIVEN the bridge script is invoked with `--feature-id release-packaging-and-whitelabel`
- WHEN the change scaffold is produced
- THEN the behavior "cargo clippy --all-targets -- -D warnings and cargo fmt --check pass cleanly" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
