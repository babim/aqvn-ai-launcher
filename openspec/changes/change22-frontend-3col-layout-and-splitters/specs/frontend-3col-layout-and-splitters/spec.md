# Spec delta: change22-frontend-3col-layout-and-splitters

## ADDED Requirements

### Requirement: change22-frontend-3col-layout-and-splitters binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `frontend-3col-layout-and-splitters`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change22-frontend-3col-layout-and-splitters-sidebar-contains-session-context-list-ne
- GIVEN the bridge script is invoked with `--feature-id frontend-3col-layout-and-splitters`
- WHEN the change scaffold is produced
- THEN the behavior "#sidebar contains session context list, New Chat button, and Server Switcher" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change22-frontend-3col-layout-and-splitters-canvas-pane-contains-tab-header-with-int
- GIVEN the bridge script is invoked with `--feature-id frontend-3col-layout-and-splitters`
- WHEN the change scaffold is produced
- THEN the behavior "#canvas-pane contains tab header with 'Intel Suite', #close-canvas-btn, and iframe #canvas-frame" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change22-frontend-3col-layout-and-splitters-canvas-frame-loads-plugins-aqvn-intel-su
- GIVEN the bridge script is invoked with `--feature-id frontend-3col-layout-and-splitters`
- WHEN the change scaffold is produced
- THEN the behavior "#canvas-frame loads /plugins/aqvn_intel_suite/webui/canvas.html" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change22-frontend-3col-layout-and-splitters-alt-s-and-toggle-sidebar-btn-toggle-side
- GIVEN the bridge script is invoked with `--feature-id frontend-3col-layout-and-splitters`
- WHEN the change scaffold is produced
- THEN the behavior "Alt+S and #toggle-sidebar-btn toggle #sidebar collapsed state with localStorage persistence" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change22-frontend-3col-layout-and-splitters-alt-c-and-toggle-canvas-btn-toggle-canva
- GIVEN the bridge script is invoked with `--feature-id frontend-3col-layout-and-splitters`
- WHEN the change scaffold is produced
- THEN the behavior "Alt+C and #toggle-canvas-btn toggle #canvas-pane collapsed state with localStorage persistence" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change22-frontend-3col-layout-and-splitters-splitters-support-smooth-horizontal-drag
- GIVEN the bridge script is invoked with `--feature-id frontend-3col-layout-and-splitters`
- WHEN the change scaffold is produced
- THEN the behavior "Splitters support smooth horizontal drag resizing within defined bounds" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
