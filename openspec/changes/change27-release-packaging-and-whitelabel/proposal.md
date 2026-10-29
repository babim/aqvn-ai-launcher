# Proposal: change27-release-packaging-and-whitelabel

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `release-packaging-and-whitelabel` (milestone `release-packaging`) to an openspec change. The feature description:

> Bump version to 0.1.6 in Cargo.toml, tauri.conf.json, and CHANGELOG.md; verify 100% whitelabel compliance (zero Agent Zero strings); verify all cargo test, clippy, and fmt checks pass.

Mission: `d398e923-10d9-46b0-af7c-32a71e95a347`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **27-001**: Cargo.toml and tauri.conf.json specify version = '0.1.6'
- **27-002**: CHANGELOG.md documents [0.1.6] release features
- **27-003**: Zero occurrences of 'Agent Zero' or 'agent-zero' across the repository
- **27-004**: cd src-tauri && cargo test passes 100% (125+ tests)
- **27-005**: cargo clippy --all-targets -- -D warnings and cargo fmt --check pass cleanly

## Goals

- Bind feature `release-packaging-and-whitelabel` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change27-release-packaging-and-whitelabel/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-REL-001`
- `VAL-REL-002`
- `VAL-REL-003`
