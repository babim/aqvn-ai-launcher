# Spec delta: change26-frontend-zoom-and-hard-refresh

## ADDED Requirements

### Requirement: change26-frontend-zoom-and-hard-refresh binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `frontend-zoom-and-hard-refresh`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change26-frontend-zoom-and-hard-refresh-ctrl-increases-zoom-ctrl-decreases-zoom-
- GIVEN the bridge script is invoked with `--feature-id frontend-zoom-and-hard-refresh`
- WHEN the change scaffold is produced
- THEN the behavior "Ctrl + increases zoom, Ctrl - decreases zoom, Ctrl 0 resets zoom to 1.0" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change26-frontend-zoom-and-hard-refresh-zoom-setting-is-stored-in-localstorage-a
- GIVEN the bridge script is invoked with `--feature-id frontend-zoom-and-hard-refresh`
- WHEN the change scaffold is produced
- THEN the behavior "Zoom setting is stored in localStorage['aqvn_zoom'] and applied on init" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change26-frontend-zoom-and-hard-refresh-hard-refresh-clears-message-ram-state-re
- GIVEN the bridge script is invoked with `--feature-id frontend-zoom-and-hard-refresh`
- WHEN the change scaffold is produced
- THEN the behavior "Hard refresh clears message RAM state, re-fetches CSRF, reconnects Socket.IO, and reloads canvas iframe" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
