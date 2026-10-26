# Proposal: change2-title-sync-ui-shell

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `title-sync-ui-shell` (milestone `scaffold-whitelabel`) to an openspec change. The feature description:

> Window title auto-sync + UI shell skeleton. Rust: WebviewWindowBuilder .on_document_title_changed(|win,title| win.set_title(&title)) — no JS setTitle, no core:window:allow-set-title permission. UI: ui/index.html + minimal chat placeholder page that sets document.title = "AQVN AI Workspace"; internal navigation link to settings.html and back; title re-syncs per navigation. Validate runtime title under Xvfb (install xdotool via sudo zypper if needed, or xprop/xwininfo; fallback ffmpeg title-bar screenshot).

Mission: `c82a03df-dfc7-438f-b44b-9089dcc40a8f`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **2-001**: rg finds literal document.title assignment in ui/
- **2-002**: Runtime native window title == AQVN AI Workspace under Xvfb
- **2-003**: Title survives navigation index→settings→index round-trip

## Goals

- Bind feature `title-sync-ui-shell` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change2-title-sync-ui-shell/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-SCAF-007`
- `VAL-SCAF-008`
- `VAL-SCAF-009`
