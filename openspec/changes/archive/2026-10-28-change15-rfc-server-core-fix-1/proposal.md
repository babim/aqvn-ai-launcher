# Proposal: change15-rfc-server-core-fix-1

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `rfc-server-core-fix-1` (milestone `rfc-ssh-bridge`) to an openspec change. The feature description:

> Scrutiny fix (blocking): RFC listener currently binds [IP]:55080 but contract ruling R2 requires wildcard 0.0.0.0:55080 (cloud-reachable). Changes: (1) src-tauri/src/rfc/server.rs bind Ipv4Addr::UNSPECIFIED instead of LOCALHOST; (2) FAIL CLOSED when rfc_password is empty — refuse to bind/start, log clear reason (no unauthenticated wildcard listener ever); (3) update tests/rfc_server.rs is_loopback assertion to expect wildcard bind + add empty-password refuse-to-bind test; (4) sandbox hardening (non-blocking from scrutiny): deny-all when allowed_paths is empty (no implicit ~/Workspace creation), honor ALL allowed_paths entries not just [0]; update sandbox tests; (5) add constant-time regression guard test per VAL-RFC-002 intent; (6) rewrite missionDir library/rfc-server-core.md bind bullet to cite R2. Verify: cd src-tauri && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check. Conventional commit fix(rfc): ...

Mission: `c82a03df-dfc7-438f-b44b-9089dcc40a8f`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **15-001**: RFC binds 0.0.0.0:55080 when enabled AND rfc_password non-empty
- **15-002**: Empty rfc_password → no bind, fail-closed with log
- **15-003**: Empty allowed_paths → sandbox deny-all; multi-entry allowed_paths all honored
- **15-004**: Full cargo test + clippy + fmt green; whitelabel gate clean

## Goals

- Bind feature `rfc-server-core-fix-1` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change15-rfc-server-core-fix-1/` during authoring.

## Validation Contract Assertions Fulfilled

(none declared)
