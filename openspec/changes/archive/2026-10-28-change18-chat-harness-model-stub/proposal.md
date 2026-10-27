# Proposal: change18-chat-harness-model-stub

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `chat-harness-model-stub` (milestone `os-integration`) to an openspec change. The feature description:

> Enable VAL-CHAT-014 assistant-record evidence: the local harness (agent-zero-e2e/agent-zero, READ-ONLY source) has no model configured, so no assistant response is ever emitted. Build a bounded stub model WITHOUT modifying harness source: (1) tools/model_stub_server.py — minimal OpenAI-compatible server: POST /v1/chat/completions returning a chat.completion JSON (support both plain and streaming SSE forms if the harness client expects stream; check harness helpers/model usage first), deterministic reply 'AQVN stub model reply: <echo of last user message>'; bind 127.0.0.1 free port; (2) configure the harness via its own config FILES only (usr/settings.json / model profile settings — inspect harness helpers/settings.py + extensions for where chat_model/model profiles live; harness venv python may be used to call its settings API) to point chat_model at the stub endpoint (base_url http://127.0.0.1:<port>, api_key dummy) — settings edits are allowed, SOURCE edits are not; (3) verify: start stub + harness (start_a0.sh) → send message via tools/chat_e2e.py/sh through the proxy → assert an assistant/response record with non-empty content arrives via state_push; stop everything, ports free; (4) wire the stub lifecycle into tools/chat_e2e.sh (start/stop with provenance checks per _e2e_lib.sh patterns). Whitelabel gate clean. Conventional commit feat(tools): ...

Mission: `c82a03df-dfc7-438f-b44b-9089dcc40a8f`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **18-001**: tools/model_stub_server.py serves OpenAI-compatible completions on loopback
- **18-002**: Harness config points chat_model at stub (config files only, no source edits)
- **18-003**: chat_e2e.sh flow yields an assistant record with non-empty content (exit 0)
- **18-004**: Clean shutdown: stub + harness + app stopped, ports free, no orphans

## Goals

- Bind feature `chat-harness-model-stub` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change18-chat-harness-model-stub/` during authoring.

## Validation Contract Assertions Fulfilled

(none declared)
