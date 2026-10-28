# Proposal: change43-feat-gateway-browser-handler

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `feat-gateway-browser-handler` (milestone `gateway-hub-phase2`) to an openspec change. The feature description:

> Implement connector_browser_op handler in src-tauri/src/connector/handlers/browser.rs bridging browser operations (open, navigate, click, type, screenshot, evaluate) to local Chrome/Edge on CDP port 9222.

Mission: `d398e923-10d9-46b0-af7c-32a71e95a347`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **43-001**: Connects to local Chrome CDP at http://127.0.0.1:9222
- **43-002**: Dispatches open, navigate, click, type, screenshot, and evaluate actions
- **43-003**: Handles open_remote_debugging by launching browser with remote debugging

## Goals

- Bind feature `feat-gateway-browser-handler` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change43-feat-gateway-browser-handler/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-GATEWAY-004`
