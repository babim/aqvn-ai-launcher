# Spec delta: change2-title-sync-ui-shell

## ADDED Requirements

### Requirement: change2-title-sync-ui-shell binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `title-sync-ui-shell`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change2-title-sync-ui-shell-rg-finds-literal-document-title-assignme
- GIVEN the bridge script is invoked with `--feature-id title-sync-ui-shell`
- WHEN the change scaffold is produced
- THEN the behavior "rg finds literal document.title assignment in ui/" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change2-title-sync-ui-shell-runtime-native-window-title-aqvn-ai-work
- GIVEN the bridge script is invoked with `--feature-id title-sync-ui-shell`
- WHEN the change scaffold is produced
- THEN the behavior "Runtime native window title == AQVN AI Workspace under Xvfb" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change2-title-sync-ui-shell-title-survives-navigation-index-settings
- GIVEN the bridge script is invoked with `--feature-id title-sync-ui-shell`
- WHEN the change scaffold is produced
- THEN the behavior "Title survives navigation index→settings→index round-trip" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
