# Spec delta: change36-feat-web-proxy-passthrough

## ADDED Requirements

### Requirement: change36-feat-web-proxy-passthrough binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `feat-web-proxy-passthrough`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change36-feat-web-proxy-passthrough-requests-to-login-css-vendor-css-js-are-
- GIVEN the bridge script is invoked with `--feature-id feat-web-proxy-passthrough`
- WHEN the change scaffold is produced
- THEN the behavior "Requests to /, /login.css, /vendor/*, /css/*, /js/* are forwarded to upstream" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change36-feat-web-proxy-passthrough-content-type-status-codes-and-headers-ar
- GIVEN the bridge script is invoked with `--feature-id feat-web-proxy-passthrough`
- WHEN the change scaffold is produced
- THEN the behavior "Content-Type, status codes, and headers are preserved" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change36-feat-web-proxy-passthrough-session-cookies-from-cookiejar-are-repla
- GIVEN the bridge script is invoked with `--feature-id feat-web-proxy-passthrough`
- WHEN the change scaffold is produced
- THEN the behavior "Session cookies from CookieJar are replayed and upstream Set-Cookie headers are absorbed" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change36-feat-web-proxy-passthrough-integration-tests-in-src-tauri-tests-pro
- GIVEN the bridge script is invoked with `--feature-id feat-web-proxy-passthrough`
- WHEN the change scaffold is produced
- THEN the behavior "Integration tests in src-tauri/tests/proxy_server.rs pass cleanly" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
