# Proposal: change40-feat-gateway-outbound-client

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `feat-gateway-outbound-client` (milestone `gateway-hub-phase2`) to an openspec change. The feature description:

> Implement outbound WebSocket gateway client in src-tauri/src/connector/ connecting to upstream /ws, executing connector_hello handshake with gateway ID, host label, and capabilities.

Mission: `d398e923-10d9-46b0-af7c-32a71e95a347`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **40-001**: Outbound connection established to /ws without requiring open inbound ports
- **40-002**: Sends connector_hello declaring scopes: files, code_execution, browser, computer_use
- **40-003**: Maintains heartbeat pings and auto-reconnects on disconnection

## Goals

- Bind feature `feat-gateway-outbound-client` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change40-feat-gateway-outbound-client/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-GATEWAY-001`
