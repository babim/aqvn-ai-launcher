# Proposal: change19-multi-server-url-switcher

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `multi-server-url-switcher` (milestone `server-profiles`) to an openspec change. The feature description:

> Multi-server URL switcher & profile presets: (1) In ui/index.html & ui/chat.js: expose an editable Server URL field directly on the login form (prefilled from store server_url), with a dropdown/datalist of saved server profiles so any user can enter or select their instance link before signing in; (2) When signing in or changing URL, save server_url to store and append to server_profiles in settings.json, triggering Rust proxy retargeting automatically; (3) In ui/settings.html & ui/settings.js: display and manage the list of saved server profiles (add/remove/select active); (4) In the chat header: display the active connected server URL so the user always knows which instance they are using; (5) Support browser-mode fallback in bridge.js (localStorage profiles); (6) Verify via agent-browser and live proxy retargeting tests. Fulfills VAL-CHAT-016.

Mission: `c82a03df-dfc7-438f-b44b-9089dcc40a8f`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **19-001**: Login screen displays editable Server URL field and saved profile options
- **19-002**: Changing Server URL persists to store and saves to server_profiles list
- **19-003**: Settings page displays and allows managing saved server profiles
- **19-004**: Chat header displays the active connected Server URL
- **19-005**: agent-browser tests verify profile selection, custom URL entry, and persistence without console errors
- **19-006**: cargo check, clippy -D warnings, fmt clean, whitelabel 0 matches

## Goals

- Bind feature `multi-server-url-switcher` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change19-multi-server-url-switcher/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-CHAT-016`
