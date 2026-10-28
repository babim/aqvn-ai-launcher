# Proposal: change44-feat-gateway-desktop-handler

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `feat-gateway-desktop-handler` (milestone `gateway-hub-phase2`) to an openspec change. The feature description:

> Implement connector_computer_use_op handler in src-tauri/src/connector/handlers/computer_use.rs for desktop screen capture and native mouse/keyboard event injection.

Mission: `d398e923-10d9-46b0-af7c-32a71e95a347`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **44-001**: Captures host screen or active window on capture op
- **44-002**: Injects mouse movement, clicks, and keystrokes on click/type/key ops
- **44-003**: Replies with connector_computer_use_op_result

## Goals

- Bind feature `feat-gateway-desktop-handler` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change44-feat-gateway-desktop-handler/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-GATEWAY-005`
