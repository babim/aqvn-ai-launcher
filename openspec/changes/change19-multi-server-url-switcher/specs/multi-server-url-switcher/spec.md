# Spec delta: change19-multi-server-url-switcher

## ADDED Requirements

### Requirement: change19-multi-server-url-switcher binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `multi-server-url-switcher`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change19-multi-server-url-switcher-login-screen-displays-editable-server-ur
- GIVEN the bridge script is invoked with `--feature-id multi-server-url-switcher`
- WHEN the change scaffold is produced
- THEN the behavior "Login screen displays editable Server URL field and saved profile options" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change19-multi-server-url-switcher-changing-server-url-persists-to-store-an
- GIVEN the bridge script is invoked with `--feature-id multi-server-url-switcher`
- WHEN the change scaffold is produced
- THEN the behavior "Changing Server URL persists to store and saves to server_profiles list" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change19-multi-server-url-switcher-settings-page-displays-and-allows-managi
- GIVEN the bridge script is invoked with `--feature-id multi-server-url-switcher`
- WHEN the change scaffold is produced
- THEN the behavior "Settings page displays and allows managing saved server profiles" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change19-multi-server-url-switcher-chat-header-displays-the-active-connecte
- GIVEN the bridge script is invoked with `--feature-id multi-server-url-switcher`
- WHEN the change scaffold is produced
- THEN the behavior "Chat header displays the active connected Server URL" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change19-multi-server-url-switcher-agent-browser-tests-verify-profile-selec
- GIVEN the bridge script is invoked with `--feature-id multi-server-url-switcher`
- WHEN the change scaffold is produced
- THEN the behavior "agent-browser tests verify profile selection, custom URL entry, and persistence without console errors" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change19-multi-server-url-switcher-cargo-check-clippy-d-warnings-fmt-clean-
- GIVEN the bridge script is invoked with `--feature-id multi-server-url-switcher`
- WHEN the change scaffold is produced
- THEN the behavior "cargo check, clippy -D warnings, fmt clean, whitelabel 0 matches" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
