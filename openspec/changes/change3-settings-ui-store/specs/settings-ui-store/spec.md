# Spec delta: change3-settings-ui-store

## ADDED Requirements

### Requirement: change3-settings-ui-store binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `settings-ui-store`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change3-settings-ui-store-all-settings-controls-present-in-dom-wit
- GIVEN the bridge script is invoked with `--feature-id settings-ui-store`
- WHEN the change scaffold is produced
- THEN the behavior "All settings controls present in DOM with correct defaults (URL prefilled, toggle OFF)" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change3-settings-ui-store-values-persist-across-app-restart-via-st
- GIVEN the bridge script is invoked with `--feature-id settings-ui-store`
- WHEN the change scaffold is produced
- THEN the behavior "Values persist across app restart via store plugin; store JSON contains changed keys" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change3-settings-ui-store-store-file-path-derived-from-identifier-
- GIVEN the bridge script is invoked with `--feature-id settings-ui-store`
- WHEN the change scaffold is produced
- THEN the behavior "Store file path derived from identifier net.aqvn.workspace" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change3-settings-ui-store-settings-page-loads-and-works-in-browser
- GIVEN the bridge script is invoked with `--feature-id settings-ui-store`
- WHEN the change scaffold is produced
- THEN the behavior "Settings page loads and works in browser-mode fallback (agent-browser, zero console errors)" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
