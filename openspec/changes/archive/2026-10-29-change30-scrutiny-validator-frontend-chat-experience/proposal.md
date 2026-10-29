# Proposal: change30-scrutiny-validator-frontend-chat-experience

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `scrutiny-validator-frontend-chat-experience` (milestone `frontend-chat-experience`) to an openspec change. The feature description:

> Scrutiny validation for milestone "frontend-chat-experience". Runs test suite, typecheck, and lint. Spawns review subagents for each completed feature. Synthesizes findings. Always returns to orchestrator.

Mission: `d398e923-10d9-46b0-af7c-32a71e95a347`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **30-001**: Validators pass (test, typecheck, lint)
- **30-002**: Review subagents spawned for each feature
- **30-003**: Findings synthesized into scrutiny report

## Goals

- Bind feature `scrutiny-validator-frontend-chat-experience` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change30-scrutiny-validator-frontend-chat-experience/` during authoring.

## Validation Contract Assertions Fulfilled

(none declared)
