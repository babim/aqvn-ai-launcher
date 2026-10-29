# Proposal: change38-feat-web-container-shell

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `feat-web-container-shell` (milestone `web-container-phase1`) to an openspec change. The feature description:

> Configure Tauri webview container to load the full upstream web UI via the local accelerated proxy (http://127.0.0.1:<port>/), ensuring internal app links stay in-window while external links open in OS browser.

Mission: `d398e923-10d9-46b0-af7c-32a71e95a347`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **38-001**: Webview displays the complete upstream web UI (Sidebar, Chat, DAG Canvas, Settings)
- **38-002**: Internal app routes and frames navigate inside the webview without opening external browser
- **38-003**: External hyperlinks open in system default browser via tauri-plugin-shell
- **38-004**: os_integration navigation tests pass cleanly

## Goals

- Bind feature `feat-web-container-shell` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change38-feat-web-container-shell/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-WEB-003`
