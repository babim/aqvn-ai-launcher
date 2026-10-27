# Spec delta: change1-tauri-scaffold-branding

## ADDED Requirements

### Requirement: change1-tauri-scaffold-branding binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `tauri-scaffold-branding`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change1-tauri-scaffold-branding-cargo-check-test-clippy-d-warnings-fmt-a
- GIVEN the bridge script is invoked with `--feature-id tauri-scaffold-branding`
- WHEN the change scaffold is produced
- THEN the behavior "cargo check/test/clippy -D warnings/fmt all exit 0, zero warnings" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change1-tauri-scaffold-branding-release-binary-builds-size-recorded-and-
- GIVEN the bridge script is invoked with `--feature-id tauri-scaffold-branding`
- WHEN the change scaffold is produced
- THEN the behavior "Release binary builds; size recorded and < 8MB" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change1-tauri-scaffold-branding-app-launches-under-xvfb-97-webkit-disabl
- GIVEN the bridge script is invoked with `--feature-id tauri-scaffold-branding`
- WHEN the change scaffold is produced
- THEN the behavior "App launches under Xvfb :97 (WEBKIT_DISABLE_DMABUF_RENDERER=1) and renders the placeholder UI (ffmpeg screenshot evidence)" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change1-tauri-scaffold-branding-tauri-conf-json-identity-fields-exactly-
- GIVEN the bridge script is invoked with `--feature-id tauri-scaffold-branding`
- WHEN the change scaffold is produced
- THEN the behavior "tauri.conf.json identity fields exactly net.aqvn.workspace / AQVN Workspace" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change1-tauri-scaffold-branding-rg-whitelabel-gate-zero-matches-outside-
- GIVEN the bridge script is invoked with `--feature-id tauri-scaffold-branding`
- WHEN the change scaffold is produced
- THEN the behavior "rg whitelabel gate: zero matches outside docs/rfc-spec/" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
