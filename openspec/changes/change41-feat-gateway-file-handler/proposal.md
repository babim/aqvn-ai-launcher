# Proposal: change41-feat-gateway-file-handler

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `feat-gateway-file-handler` (milestone `gateway-hub-phase2`) to an openspec change. The feature description:

> Implement connector_file_op handler in src-tauri/src/connector/handlers/files.rs supporting read, write, patch, and files_list strictly jailed within ~/Workspace, with Transfer Protocol v1 support.

Mission: `d398e923-10d9-46b0-af7c-32a71e95a347`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **41-001**: Handles connector_file_op for read, write, patch, and list
- **41-002**: Rejects path traversal or symlink escapes outside sandbox
- **41-003**: Replies with connector_file_op_result over WebSocket

## Goals

- Bind feature `feat-gateway-file-handler` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change41-feat-gateway-file-handler/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-GATEWAY-002`
