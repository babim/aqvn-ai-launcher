# Spec delta: change45-feat-gateway-settings-scopes

## ADDED Requirements

### Requirement: change45-feat-gateway-settings-scopes binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `feat-gateway-settings-scopes`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change45-feat-gateway-settings-scopes-settings-ui-exposes-checkboxes-for-each-
- GIVEN the bridge script is invoked with `--feature-id feat-gateway-settings-scopes`
- WHEN the change scaffold is produced
- THEN the behavior "Settings UI exposes checkboxes for each scope and master connector switch" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change45-feat-gateway-settings-scopes-toggling-master-off-immediately-terminat
- GIVEN the bridge script is invoked with `--feature-id feat-gateway-settings-scopes`
- WHEN the change scaffold is produced
- THEN the behavior "Toggling master off immediately terminates the outbound connection" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change45-feat-gateway-settings-scopes-connector-hello-scopes-match-active-conf
- GIVEN the bridge script is invoked with `--feature-id feat-gateway-settings-scopes`
- WHEN the change scaffold is produced
- THEN the behavior "connector_hello scopes match active configuration" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
