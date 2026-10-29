# Spec delta: change37-feat-web-asset-cache

## ADDED Requirements

### Requirement: change37-feat-web-asset-cache binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `feat-web-asset-cache`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change37-feat-web-asset-cache-static-asset-requests-ttf-woff2-css-js-a
- GIVEN the bridge script is invoked with `--feature-id feat-web-asset-cache`
- WHEN the change scaffold is produced
- THEN the behavior "Static asset requests (.ttf, .woff2, .css, .js) are cached locally under app cache dir" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change37-feat-web-asset-cache-subsequent-requests-are-served-instantly
- GIVEN the bridge script is invoked with `--feature-id feat-web-asset-cache`
- WHEN the change scaffold is produced
- THEN the behavior "Subsequent requests are served instantly from disk cache without network roundtrips" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change37-feat-web-asset-cache-cache-hit-header-or-indicator-verified-v
- GIVEN the bridge script is invoked with `--feature-id feat-web-asset-cache`
- WHEN the change scaffold is produced
- THEN the behavior "Cache hit header or indicator verified via integration test" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
