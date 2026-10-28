# Proposal: change42-feat-gateway-exec-handler

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `feat-gateway-exec-handler` (milestone `gateway-hub-phase2`) to an openspec change. The feature description:

> Implement connector_exec_op handler in src-tauri/src/connector/handlers/exec.rs using portable-pty to allocate and manage local PTY shell sessions for code execution.

Mission: `d398e923-10d9-46b0-af7c-32a71e95a347`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **42-001**: Handles connector_exec_op with runtime=terminal
- **42-002**: Executes shell commands inside stateful PTY session
- **42-003**: Streams output back to agent via connector_exec_op_result

## Goals

- Bind feature `feat-gateway-exec-handler` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change42-feat-gateway-exec-handler/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-GATEWAY-003`
