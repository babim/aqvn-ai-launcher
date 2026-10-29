# Proposal: change26-frontend-zoom-and-hard-refresh

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `frontend-zoom-and-hard-refresh` (milestone `frontend-chat-experience`) to an openspec change. The feature description:

> Implement UI Zoom in/out/reset via Ctrl +/-/0 with localStorage persistence, and Hard Refresh via #hard-refresh-btn and Ctrl+Shift+R / F5 in ui/chat.js.

Mission: `d398e923-10d9-46b0-af7c-32a71e95a347`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **26-001**: Ctrl + increases zoom, Ctrl - decreases zoom, Ctrl 0 resets zoom to 1.0
- **26-002**: Zoom setting is stored in localStorage['aqvn_zoom'] and applied on init
- **26-003**: Hard refresh clears message RAM state, re-fetches CSRF, reconnects Socket.IO, and reloads canvas iframe

## Goals

- Bind feature `frontend-zoom-and-hard-refresh` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change26-frontend-zoom-and-hard-refresh/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-ZOOM-001`
- `VAL-ZOOM-002`
