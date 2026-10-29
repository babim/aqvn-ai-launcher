# Proposal: change20-backend-proxy-routes-and-handlers

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `backend-proxy-routes-and-handlers` (milestone `backend-proxy`) to an openspec change. The feature description:

> Add proxy routes for /plugins/{*rest}, /extensions/{*rest}, and /public/{*rest} with handle_forward_raw calling upstream.forward_raw(...) in src-tauri/src/proxy/server.rs, plus unit tests in src-tauri/tests/proxy_server.rs.

Mission: `d398e923-10d9-46b0-af7c-32a71e95a347`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **20-001**: Requests to /plugins/*, /extensions/*, and /public/* are routed to handle_forward_raw
- **20-002**: handle_forward_raw forwards requests without requiring CSRF header
- **20-003**: Upstream session cookies are attached and response Set-Cookie headers update the CookieJar
- **20-004**: Response Content-Type and raw binary body bytes are preserved
- **20-005**: Integration tests in src-tauri/tests/proxy_server.rs pass cleanly

## Goals

- Bind feature `backend-proxy-routes-and-handlers` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change20-backend-proxy-routes-and-handlers/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-PROXY-001`
- `VAL-PROXY-002`
- `VAL-PROXY-003`
