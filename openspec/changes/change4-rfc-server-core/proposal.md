# Proposal: change4-rfc-server-core

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `rfc-server-core` (milestone `rfc-ssh-bridge`) to an openspec change. The feature description:

> RFC server core: axum on tauri tokio runtime, bind 0.0.0.0:55080 (port from config). POST /api/rfc only: parse {rfc_input:str, hash:str}; HMAC-SHA256(password UTF-8, rfc_input EXACT received bytes) lowercase hex, constant-time verify (hmac::Mac::verify_slice); wrong/missing hash → 403 text/plain; unparseable rfc_input → 500 text/plain; non-POST verbs → 405 text/html 'Method X not allowed for: rfc'; unknown paths → 404 'API endpoint not found: ...'. Dispatch skeleton: hard whitelist table keyed (module, function_name); unknown module/function → 403. Master toggle (rfc_enabled, default OFF) = no listener at all. Success path: 200 application/json bare result. Unit tests per research/rfc-protocol.md §§2-4 incl. worked example §7 (password 'secret', precomputed hash).

Mission: `c82a03df-dfc7-438f-b44b-9089dcc40a8f`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **4-001**: Worked-example request (spec §7) returns 200 + bare JSON
- **4-002**: Wrong hash / missing hash → 403; malformed rfc_input → 500; GET → 405; unknown route → 404 (exact bodies)
- **4-003**: Unknown module/function → 403 (whitelist enforced)
- **4-004**: Toggle OFF → port 55080 closed (ss -tln evidence)
- **4-005**: HMAC constant-time (code review criterion)

## Goals

- Bind feature `rfc-server-core` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change4-rfc-server-core/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-RFC-001`
- `VAL-RFC-002`
- `VAL-RFC-003`
- `VAL-RFC-004`
- `VAL-RFC-005`
- `VAL-RFC-006`
- `VAL-RFC-007`
- `VAL-RFC-008`
- `VAL-RFC-025`
