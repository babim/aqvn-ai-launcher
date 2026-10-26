# Proposal: change8-bridge-proxy

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `bridge-proxy` (milestone `os-integration`) to an openspec change. The feature description:

> Local bridge proxy per research/chat-protocol.md §9 (option c). axum on 127.0.0.1:0 (random free port); actual port published to UI via Tauri init script/app state (never hardcoded). Rust owns: cookie jar (session_<rid>, csrf_token_<rid>), CSRF bootstrap (GET /api/csrf_token → set cookie + inject X-CSRF-Token header on /api/* forwards), POST /login form on behalf of UI, Origin/Host spoof matching target server (passes A0 validate_ws_origin), CORS headers for webview origin, 2-leg WS proxy for /socket.io/ (browser ⇄ proxy ⇄ server, namespace /ws, survives 45s ping interval), 403 → CSRF refresh + retry exactly once, runtime_id rotation detection (server restart) → invalidate jar + re-bootstrap. Bind loopback ONLY. Target server from config (default https://a0.matmagoc.com; harness :5000 for tests). Tests: unit (jar/csrf logic vs local stub) + integration against harness :5000 (login → csrf → WS state_request ACK via harness-venv python-socketio through proxy).

Mission: `c82a03df-dfc7-438f-b44b-9089dcc40a8f`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **8-001**: ss shows 127.0.0.1 bind only; port random per launch; UI receives it at init
- **8-002**: CSRF bootstrap + login-on-behalf work against harness :5000 (admin/test-auth-2026)
- **8-003**: WS handshake through proxy passes validate_ws_origin (Origin spoofed) — state_request ACK received
- **8-004**: Session survives ≥50s (ping interval); 403 → single refresh+retry; runtime rotation → auto re-bootstrap

## Goals

- Bind feature `bridge-proxy` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change8-bridge-proxy/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-PROXY-001`
- `VAL-PROXY-002`
- `VAL-PROXY-003`
- `VAL-PROXY-004`
- `VAL-PROXY-005`
- `VAL-PROXY-006`
- `VAL-PROXY-007`
- `VAL-PROXY-008`
