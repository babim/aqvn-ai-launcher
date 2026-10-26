# Spec delta: change8-bridge-proxy

## ADDED Requirements

### Requirement: change8-bridge-proxy binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `bridge-proxy`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change8-bridge-proxy-ss-shows-127-0-0-1-bind-only-port-random
- GIVEN the bridge script is invoked with `--feature-id bridge-proxy`
- WHEN the change scaffold is produced
- THEN the behavior "ss shows 127.0.0.1 bind only; port random per launch; UI receives it at init" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change8-bridge-proxy-csrf-bootstrap-login-on-behalf-work-agai
- GIVEN the bridge script is invoked with `--feature-id bridge-proxy`
- WHEN the change scaffold is produced
- THEN the behavior "CSRF bootstrap + login-on-behalf work against harness :5000 (admin/test-auth-2026)" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change8-bridge-proxy-ws-handshake-through-proxy-passes-valida
- GIVEN the bridge script is invoked with `--feature-id bridge-proxy`
- WHEN the change scaffold is produced
- THEN the behavior "WS handshake through proxy passes validate_ws_origin (Origin spoofed) — state_request ACK received" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change8-bridge-proxy-session-survives-50s-ping-interval-403-s
- GIVEN the bridge script is invoked with `--feature-id bridge-proxy`
- WHEN the change scaffold is produced
- THEN the behavior "Session survives ≥50s (ping interval); 403 → single refresh+retry; runtime rotation → auto re-bootstrap" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
