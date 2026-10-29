# Proposal: change21-frontend-vendor-marked-highlight

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `frontend-vendor-marked-highlight` (milestone `frontend-vendor-layout`) to an openspec change. The feature description:

> Stage minified marked.min.js and highlight.min.js into ui/vendor/ and declare their script tags in ui/index.html.

Mission: `d398e923-10d9-46b0-af7c-32a71e95a347`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **21-001**: ui/vendor/marked.min.js is present and valid JS
- **21-002**: ui/vendor/highlight.min.js is present and valid JS
- **21-003**: ui/index.html loads marked.min.js and highlight.min.js before chat.js
- **21-004**: window.marked and window.hljs are defined in the webview window scope

## Goals

- Bind feature `frontend-vendor-marked-highlight` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change21-frontend-vendor-marked-highlight/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-MD-001`
