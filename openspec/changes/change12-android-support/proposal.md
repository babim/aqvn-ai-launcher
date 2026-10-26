# Proposal: change12-android-support

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `android-support` (milestone `mobile-ci-benchmark`) to an openspec change. The feature description:

> Android support. source ~/.bashrc (ANDROID_HOME/NDK_HOME/JAVA_HOME). cargo tauri android init → commit src-tauri/gen/android. Cargo features desktop/mobile split: rfc/, ssh/, tray, hotkey behind #[cfg(feature = "desktop")] (or cfg(not(target_os="android"))) — chat + proxy + notification remain on mobile. AndroidManifest: AQVN app label (whitelabel), package net.aqvn.workspace, INTERNET permission only. Build: cargo tauri android build --apk (unsigned debug — agreed gate; long build → fireAndForget). Verify: APK exists, unzip -l shows lib/arm64-v8a/*.so, apkanalyzer manifest correct, RFC/SSH symbols absent from mobile .so (strings/nm: no russh, no /api/rfc). Static chat-only surface check (rg over ui/ + capabilities). No emulator on box — runtime test CI/user-deferred (log it).

Mission: `c82a03df-dfc7-438f-b44b-9089dcc40a8f`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **12-001**: gen/android committed and clean; APK builds exit 0 with arm64 libs
- **12-002**: Manifest: net.aqvn.workspace + AQVN label + INTERNET-only
- **12-003**: RFC/SSH/tray/hotkey absent from Android build (source gating + .so symbol check)
- **12-004**: Deferral of runtime test explicitly logged

## Goals

- Bind feature `android-support` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change12-android-support/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-MOB-001`
- `VAL-MOB-002`
- `VAL-MOB-003`
- `VAL-MOB-004`
