# Proposal: change17-rfc-ssh-e2e-fix-1

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `rfc-ssh-e2e-fix-1` (milestone `rfc-ssh-bridge`) to an openspec change. The feature description:

> Scrutiny fix (non-blocking, hardening): tools/a0_e2e.sh + tools/_e2e_lib.sh cleanup robustness: (1) before killing harness PID from pidfile, verify ps -p PID -o args= names the harness (provenance check); (2) keep pidfile if kill fails; (3) use the existing wait_port_closed loop for port 5000 too (currently probed once right after SIGTERM); (4) final cleanup asserts harness/Xvfb/app processes actually exited (not just ports free). Verify: bash tools/a0_e2e.sh exits 0 end-to-end, then bash -n + shellcheck -S warning clean. Conventional commit fix(e2e): ...

Mission: `c82a03df-dfc7-438f-b44b-9089dcc40a8f`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **17-001**: a0_e2e.sh runs full harness E2E, exits 0, leaves zero orphans
- **17-002**: Kill paths provenance-checked; pidfile retained on kill failure
- **17-003**: Port 5000 waits to close like 55080/55022

## Goals

- Bind feature `rfc-ssh-e2e-fix-1` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change17-rfc-ssh-e2e-fix-1/` during authoring.

## Validation Contract Assertions Fulfilled

(none declared)
