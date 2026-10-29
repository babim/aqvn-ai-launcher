# Proposal: change36-feat-web-proxy-passthrough

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `feat-web-proxy-passthrough` (milestone `web-container-phase1`) to an openspec change. The feature description:

> Expand Axum loopback proxy in src-tauri/src/proxy/server.rs so that root path (/), login (/login.css), static assets (/vendor/{*rest}, /css/{*rest}, /js/{*rest}), and all app resources are forwarded upstream via forward_raw, serving the full web application.

Mission: `d398e923-10d9-46b0-af7c-32a71e95a347`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **36-001**: Requests to /, /login.css, /vendor/*, /css/*, /js/* are forwarded to upstream
- **36-002**: Content-Type, status codes, and headers are preserved
- **36-003**: Session cookies from CookieJar are replayed and upstream Set-Cookie headers are absorbed
- **36-004**: Integration tests in src-tauri/tests/proxy_server.rs pass cleanly

## Goals

- Bind feature `feat-web-proxy-passthrough` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change36-feat-web-proxy-passthrough/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-WEB-001`
