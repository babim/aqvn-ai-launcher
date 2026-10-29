# Proposal: change37-feat-web-asset-cache

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `feat-web-asset-cache` (milestone `web-container-phase1`) to an openspec change. The feature description:

> Implement local asset caching layer in src-tauri/src/proxy/ to pre-cache heavy static assets (Inter fonts, Ace editor, CSS/JS vendor files) in local storage/memory, serving them immediately with <100ms FCP.

Mission: `d398e923-10d9-46b0-af7c-32a71e95a347`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **37-001**: Static asset requests (.ttf, .woff2, .css, .js) are cached locally under app cache dir
- **37-002**: Subsequent requests are served instantly from disk cache without network roundtrips
- **37-003**: Cache hit header or indicator verified via integration test

## Goals

- Bind feature `feat-web-asset-cache` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change37-feat-web-asset-cache/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-WEB-002`
