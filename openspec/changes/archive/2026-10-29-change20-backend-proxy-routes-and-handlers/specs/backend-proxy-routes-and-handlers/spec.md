# Spec delta: change20-backend-proxy-routes-and-handlers

## ADDED Requirements

### Requirement: change20-backend-proxy-routes-and-handlers binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `backend-proxy-routes-and-handlers`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change20-backend-proxy-routes-and-handlers-requests-to-plugins-extensions-and-publi
- GIVEN the bridge script is invoked with `--feature-id backend-proxy-routes-and-handlers`
- WHEN the change scaffold is produced
- THEN the behavior "Requests to /plugins/*, /extensions/*, and /public/* are routed to handle_forward_raw" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change20-backend-proxy-routes-and-handlers-handle-forward-raw-forwards-requests-wit
- GIVEN the bridge script is invoked with `--feature-id backend-proxy-routes-and-handlers`
- WHEN the change scaffold is produced
- THEN the behavior "handle_forward_raw forwards requests without requiring CSRF header" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change20-backend-proxy-routes-and-handlers-upstream-session-cookies-are-attached-an
- GIVEN the bridge script is invoked with `--feature-id backend-proxy-routes-and-handlers`
- WHEN the change scaffold is produced
- THEN the behavior "Upstream session cookies are attached and response Set-Cookie headers update the CookieJar" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change20-backend-proxy-routes-and-handlers-response-content-type-and-raw-binary-bod
- GIVEN the bridge script is invoked with `--feature-id backend-proxy-routes-and-handlers`
- WHEN the change scaffold is produced
- THEN the behavior "Response Content-Type and raw binary body bytes are preserved" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change20-backend-proxy-routes-and-handlers-integration-tests-in-src-tauri-tests-pro
- GIVEN the bridge script is invoked with `--feature-id backend-proxy-routes-and-handlers`
- WHEN the change scaffold is produced
- THEN the behavior "Integration tests in src-tauri/tests/proxy_server.rs pass cleanly" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
