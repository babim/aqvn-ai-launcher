# Proposal: change7-rfc-ssh-e2e

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `rfc-ssh-e2e` (milestone `rfc-ssh-bridge`) to an openspec change. The feature description:

> E2E proof with real upstream wire code. tools/spec_client.py: standalone replica of A0 helpers/crypto.py+rfc.py client (HMAC-SHA256 hexdigest over json.dumps input, aiohttp-style POST) — scenarios: valid read, wrong hash, outside-sandbox path (Gate 3 trio). tools/a0_e2e.sh: starts harness (start_a0.sh, wait LISTEN :5000), runs via harness venv python: (1) helpers.rfc.call_rfc round-trip read_file/write_file against our :55080 with RFC_PASSWORD; (2) full SSH flow: generate RSA-2048 keypair → call_rfc _provide_root_password(hex PEM) → helpers/crypto decrypt (RSA-OAEP-SHA256) → assert == ssh_password → paramiko connect :55022 → run 'echo ok' → assert output; stops harness (stop_a0.sh), kills orphans by PID. NEVER modify harness source. Harness boot trap: plain run_ui.py (never --dockerized=false).

Mission: `c82a03df-dfc7-438f-b44b-9089dcc40a8f`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **7-001**: spec_client Gate 3: correct hash → file content; wrong hash → 403; outside sandbox → 403
- **7-002**: Real A0 call_rfc (harness venv) succeeds against our server — wire-compatible proof
- **7-003**: SSH exchange decrypts to exact ssh_password; paramiko session runs command end-to-end
- **7-004**: Harness left stopped, no orphans, ports free

## Goals

- Bind feature `rfc-ssh-e2e` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change7-rfc-ssh-e2e/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-RFC-027`
- `VAL-RFC-028`
- `VAL-SSH-007`
- `VAL-SSH-008`
