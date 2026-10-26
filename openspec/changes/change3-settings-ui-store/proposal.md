# Proposal: change3-settings-ui-store

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `settings-ui-store` (milestone `scaffold-whitelabel`) to an openspec change. The feature description:

> Settings page + persistence. ui/settings.html + settings.js: inputs for server URL (default https://a0.matmagoc.com), RFC master toggle ('Allow agent control', default OFF, explicit security warning text), RFC password, SSH password, allowed paths (list editor), RFC port (55080), SSH port (55022). bridge.js: Tauri IPC wrapper with browser-mode fallback (detect window.__TAURI_INTERNALS__; localStorage mock) so agent-browser can test headless. Rust config.rs: typed settings over tauri-plugin-store settings.json with the same defaults; store file lands under identifier-derived config dir. Persistence round-trip verified under Xvfb (change 4 values → restart → values survive) AND via agent-browser on static serve (port 3100-3199).

Mission: `c82a03df-dfc7-438f-b44b-9089dcc40a8f`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **3-001**: All settings controls present in DOM with correct defaults (URL prefilled, toggle OFF)
- **3-002**: Values persist across app restart via store plugin; store JSON contains changed keys
- **3-003**: Store file path derived from identifier net.aqvn.workspace
- **3-004**: Settings page loads and works in browser-mode fallback (agent-browser, zero console errors)

## Goals

- Bind feature `settings-ui-store` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change3-settings-ui-store/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-SCAF-010`
- `VAL-SCAF-011`
- `VAL-SCAF-012`
