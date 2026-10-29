# Proposal: change39-feat-web-notification-bridge

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `feat-web-notification-bridge` (milestone `web-container-phase1`) to an openspec change. The feature description:

> Inject client bridge script into the webview container to detect agent task completion events and dispatch native OS desktop notifications (notify_task_complete) when the window is hidden/minimized.

Mission: `d398e923-10d9-46b0-af7c-32a71e95a347`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **39-001**: Client bridge script detects task completion from WebSocket/DOM state
- **39-002**: Dispatches Tauri IPC notify_task_complete when document.hidden === true
- **39-003**: Updates system tray icon state between idle and busy

## Goals

- Bind feature `feat-web-notification-bridge` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change39-feat-web-notification-bridge/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-WEB-004`
