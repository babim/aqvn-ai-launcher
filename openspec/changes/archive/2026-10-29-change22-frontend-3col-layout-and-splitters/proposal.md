# Proposal: change22-frontend-3col-layout-and-splitters

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `frontend-3col-layout-and-splitters` (milestone `frontend-vendor-layout`) to an openspec change. The feature description:

> Restructure ui/index.html DOM into 3 columns (#sidebar, #chat-pane, #canvas-pane) with draggable splitters (#splitter-left, #splitter-right), topbar toggle buttons, and Alt+S / Alt+C shortcut support in ui/chat.js and ui/chat.css.

Mission: `d398e923-10d9-46b0-af7c-32a71e95a347`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **22-001**: #sidebar contains session context list, New Chat button, and Server Switcher
- **22-002**: #canvas-pane contains tab header with 'Intel Suite', #close-canvas-btn, and iframe #canvas-frame
- **22-003**: #canvas-frame loads /plugins/aqvn_intel_suite/webui/canvas.html
- **22-004**: Alt+S and #toggle-sidebar-btn toggle #sidebar collapsed state with localStorage persistence
- **22-005**: Alt+C and #toggle-canvas-btn toggle #canvas-pane collapsed state with localStorage persistence
- **22-006**: Splitters support smooth horizontal drag resizing within defined bounds

## Goals

- Bind feature `frontend-3col-layout-and-splitters` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change22-frontend-3col-layout-and-splitters/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-LAYOUT-001`
- `VAL-LAYOUT-002`
- `VAL-LAYOUT-003`
- `VAL-LAYOUT-004`
