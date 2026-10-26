# Proposal: change11-cross-area-integration

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `cross-area-integration` (milestone `os-integration`) to an openspec change. The feature description:

> Cross-area journey hardening: run the full user journeys end-to-end and fix glue gaps (no new architecture). Journey A: launch (Xvfb) → settings enable RFC + password + allowed path → harness call_rfc reads/writes file inside allowed path → evidence. Journey B: chat via proxy → harness :5000 → notification on completion → tray busy→idle. Restart persistence: settings survive restart AND servers re-listen with saved config. Whitelabel sweep across named surfaces (window title, tray tooltip, notification text, settings UI, chat UI, Android label metadata). Toggle independence: RFC toggle OFF → 55080+55022 closed while chat keeps working. Concurrency: chat stays alive while RFC serves concurrent harness calls. Fix any defect found via minimal change + regression test named after the assertion ID.

Mission: `c82a03df-dfc7-438f-b44b-9089dcc40a8f`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **11-001**: Journeys A+B pass end-to-end with screenshots/logs
- **11-002**: Restart → settings + listeners restored from saved config
- **11-003**: Whitelabel sweep clean on all six named surfaces
- **11-004**: Toggle OFF closes both control ports, chat unaffected; chat stable under concurrent RFC load

## Goals

- Bind feature `cross-area-integration` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change11-cross-area-integration/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-CROSS-001`
- `VAL-CROSS-002`
- `VAL-CROSS-003`
- `VAL-CROSS-004`
- `VAL-CROSS-005`
- `VAL-CROSS-006`
