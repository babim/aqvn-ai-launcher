# Proposal: change14-benchmarks-readme

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `benchmarks-readme` (milestone `mobile-ci-benchmark`) to an openspec change. The feature description:

> tools/benchmark.sh (executable, bash -n clean): measures all five gates on release binary under Xvfb rig — (1) Linux binary <8MB (stat bytes), (2) main process RSS <35MB (ps -o rss= on main PID, exclude WebKitWebProcess/WebKitNetworkProcess), (3) total family RSS <250MB, (4) FCP <100ms after window shown (JS performance API paint entry; fallback first-layout timestamp — record method), (5) cold start <1.5s (exec → window mapped; record host load). Prints PASS/FAIL table, exits non-zero on any fail, kills all spawned PIDs. Run for real, capture numbers as evidence. If a gate fails after honest optimization → report exact numbers to orchestrator (do NOT weaken gates or drop features unilaterally). README.md: what/why, setup, run, test, build (Linux/Windows/Android), security model (toggle default OFF, sandbox, denylist, loopback proxy), known limitations (Wayland hotkey, Linux tray menu-only, CI-deferred Windows/Actions runtime, unsigned debug APK), whitelabel-clean.

Mission: `c82a03df-dfc7-438f-b44b-9089dcc40a8f`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **14-001**: benchmark.sh runs all 5 gates, prints table, correct exit code, no orphans
- **14-002**: Real measured numbers recorded; all gates PASS (or honest failure report to orchestrator)
- **14-003**: README complete for all platforms + security model + limitations; whitelabel grep clean

## Goals

- Bind feature `benchmarks-readme` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change14-benchmarks-readme/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-BENCH-001`
- `VAL-BENCH-002`
- `VAL-BENCH-003`
- `VAL-BENCH-004`
- `VAL-BENCH-005`
- `VAL-BENCH-006`
- `VAL-CI-005`
