# Proposal: change31-user-testing-validator-frontend-chat-experience

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `user-testing-validator-frontend-chat-experience` (milestone `frontend-chat-experience`) to an openspec change. The feature description:

> User testing validation for milestone "frontend-chat-experience". Determines testable assertions from features' fulfills field, sets up environment, spawns flow validator subagents, synthesizes results. Always returns to orchestrator.

Mission: `d398e923-10d9-46b0-af7c-32a71e95a347`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **31-001**: Testable assertions determined from fulfills mapping
- **31-002**: Environment set up (services started, data seeded)
- **31-003**: Flow validator subagents spawned and completed
- **31-004**: Results synthesized, validation-state.json updated

## Goals

- Bind feature `user-testing-validator-frontend-chat-experience` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change31-user-testing-validator-frontend-chat-experience/` during authoring.

## Validation Contract Assertions Fulfilled

(none declared)
