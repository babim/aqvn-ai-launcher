# Spec delta: change21-frontend-vendor-marked-highlight

## ADDED Requirements

### Requirement: change21-frontend-vendor-marked-highlight binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `frontend-vendor-marked-highlight`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change21-frontend-vendor-marked-highlight-ui-vendor-marked-min-js-is-present-and-v
- GIVEN the bridge script is invoked with `--feature-id frontend-vendor-marked-highlight`
- WHEN the change scaffold is produced
- THEN the behavior "ui/vendor/marked.min.js is present and valid JS" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change21-frontend-vendor-marked-highlight-ui-vendor-highlight-min-js-is-present-an
- GIVEN the bridge script is invoked with `--feature-id frontend-vendor-marked-highlight`
- WHEN the change scaffold is produced
- THEN the behavior "ui/vendor/highlight.min.js is present and valid JS" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change21-frontend-vendor-marked-highlight-ui-index-html-loads-marked-min-js-and-hi
- GIVEN the bridge script is invoked with `--feature-id frontend-vendor-marked-highlight`
- WHEN the change scaffold is produced
- THEN the behavior "ui/index.html loads marked.min.js and highlight.min.js before chat.js" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change21-frontend-vendor-marked-highlight-window-marked-and-window-hljs-are-define
- GIVEN the bridge script is invoked with `--feature-id frontend-vendor-marked-highlight`
- WHEN the change scaffold is produced
- THEN the behavior "window.marked and window.hljs are defined in the webview window scope" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
