# Proposal: change24-frontend-focus-mode-and-hotkeys

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `frontend-focus-mode-and-hotkeys` (milestone `frontend-chat-experience`) to an openspec change. The feature description:

> Implement Focus Mode in ui/chat.js and ui/chat.css triggered by F11 and Ctrl+Shift+F, hiding sidebars and centering the chat pane with auto-focused input.

Mission: `d398e923-10d9-46b0-af7c-32a71e95a347`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **24-001**: F11 and Ctrl+Shift+F toggle .focus-mode class on body
- **24-002**: .focus-mode hides #sidebar, #canvas-pane, and splitters
- **24-003**: .focus-mode centers #chat-pane with max-width: 840px
- **24-004**: Input #message-input is focused immediately upon entering focus mode

## Goals

- Bind feature `frontend-focus-mode-and-hotkeys` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change24-frontend-focus-mode-and-hotkeys/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-FOCUS-001`
