# Proposal: change13-taskfile-ci

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `taskfile-ci` (milestone `mobile-ci-benchmark`) to an openspec change. The feature description:

> Taskfile.yml: build:linux (cargo tauri build --bundles deb,rpm — verify runs locally, long → fireAndForget), build:windows (documented CI-only), build:android (apk), plus test/lint/bench passthroughs; task --list parses clean. .github/workflows/release.yml: on tag v* push; jobs — linux (ubuntu-24.04, webkit2gtk-4.1 deps, deb+rpm artifacts), windows (windows-latest, tauri-apps/tauri-action@v1, nsis+msi, explicit size-gate step failing job if .exe > 10MB — user-approved 2026-10-26), android (tauri-action@v1 mobile:'android' EXPERIMENTAL, unsigned apk artifact upload). Validate: actionlint exit 0 + python yaml.safe_load exit 0. GH Actions runtime NOT testable locally — execution deferred to user push (log it).

Mission: `c82a03df-dfc7-438f-b44b-9089dcc40a8f`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **13-001**: task --list shows build:linux/build:windows/build:android; task build:linux exits 0 with deb+rpm
- **13-002**: release.yml: tag trigger, 3 jobs, windows size gate 10MB step present
- **13-003**: actionlint + YAML parse clean
- **13-004**: CI-runtime deferral logged

## Goals

- Bind feature `taskfile-ci` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change13-taskfile-ci/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-MOB-005`
- `VAL-CI-001`
- `VAL-CI-002`
- `VAL-CI-003`
- `VAL-CI-004`
