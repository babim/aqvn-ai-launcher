# Proposal: change32-scrutiny-validator-backend-proxy

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `scrutiny-validator-backend-proxy` (milestone `backend-proxy`) to an openspec change. The feature description:

> Scrutiny validation for milestone "backend-proxy". Runs test suite, typecheck, and lint. Spawns review subagents for each completed feature. Synthesizes findings. Always returns to orchestrator.

Mission: `d398e923-10d9-46b0-af7c-32a71e95a347`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **32-001**: Validators pass (test, typecheck, lint)
- **32-002**: Review subagents spawned for each feature
- **32-003**: Findings synthesized into scrutiny report

## Goals

- Bind feature `scrutiny-validator-backend-proxy` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change32-scrutiny-validator-backend-proxy/` during authoring.

## Validation Contract Assertions Fulfilled

(none declared)
