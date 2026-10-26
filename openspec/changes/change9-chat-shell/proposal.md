# Proposal: change9-chat-shell

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `chat-shell` (milestone `os-integration`) to an openspec change. The feature description:

> Full chat shell per research/chat-protocol.md. Vendor socket.io-client 4.8.1 (copy harness webui/vendor/socket.io.min.js, preserve MIT header) — no other JS deps, no build step. chat.js: connect io(<proxy>/ws) auth {csrf_token, handlers:["ws_webui"]}; state_request handshake after EVERY connect/reconnect/context-switch (log_from:0); merge state_push snapshots logs[] by id (cumulative content REPLACE not append); log_progress status line while log_progress_active; login form (proxy /login) with error state; send via POST /api/message_async (client GUID message_id, optimistic user row merges with server record); stop → POST /api/stop; new chat → POST /api/chat_create + re-handshake; context switch loads history log_from:0; redirect-to-/login detection → session-expired re-login UI; 403 → CSRF refresh + retry once; reconnect → re-handshake without page reload. document.title stays 'AQVN AI Workspace'. Browser-mode parity: same page testable via agent-browser on static serve (bridge.js fallback; proxy origin configurable). E2E: deterministic flow vs harness :5000 through proxy; protocol-level vs real https://a0.matmagoc.com with .env creds (REDACT in evidence; unreachable/empty creds → blocked not failed).

Mission: `c82a03df-dfc7-438f-b44b-9089dcc40a8f`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **9-001**: agent-browser: login → send → streaming render (merge by id) → stop → new chat, zero console errors
- **9-002**: Harness E2E: full chat round-trip through proxy against :5000
- **9-003**: Real-server E2E: login + state_request + message_async + response records vs a0.matmagoc.com (creds redacted)
- **9-004**: Reconnect re-handshakes; session expiry surfaces re-login UI; browser-mode and real app consistent

## Goals

- Bind feature `chat-shell` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change9-chat-shell/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-CHAT-001`
- `VAL-CHAT-002`
- `VAL-CHAT-003`
- `VAL-CHAT-004`
- `VAL-CHAT-005`
- `VAL-CHAT-006`
- `VAL-CHAT-007`
- `VAL-CHAT-008`
- `VAL-CHAT-009`
- `VAL-CHAT-010`
- `VAL-CHAT-011`
- `VAL-CHAT-012`
- `VAL-CHAT-013`
- `VAL-CHAT-014`
- `VAL-CHAT-015`
