# Proposal: change1-tauri-scaffold-branding

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `tauri-scaffold-branding` (milestone `scaffold-whitelabel`) to an openspec change. The feature description:

> Tauri v2 scaffold + AQVN branding. src-tauri/ (Cargo.toml: tauri 2 tray-icon feature, tauri-build, axum 0.8, tokio, serde/serde_json, hmac, sha2, base64, thiserror; tauri plugins shell/notification/global-shortcut/store; release profile codegen-units=1 lto=true opt-level="s" panic="abort" strip=true), tauri.conf.json (identifier net.aqvn.workspace, productName AQVN Workspace, frontendDist ../ui, removeUnusedCommands), capabilities/default.json (explicit permissions), icons/ (icon.png MANDATORY + 32x32, 128x128, [REDACTED_EMAIL], icon.ico — AQVN-branded, generate programmatically e.g. via ImageMagick/rsvg from a simple AQVN mark), minimal ui/index.html placeholder, minimal main.rs/lib.rs builder. Create docs/rfc-spec/ dir (protocol notes home, whitelabel exclusion). Initial git commit including existing .gitignore (NEVER .env). Measure release binary size baseline.

Mission: `c82a03df-dfc7-438f-b44b-9089dcc40a8f`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **1-001**: cargo check/test/clippy -D warnings/fmt all exit 0, zero warnings
- **1-002**: Release binary builds; size recorded and < 8MB
- **1-003**: App launches under Xvfb :97 (WEBKIT_DISABLE_DMABUF_RENDERER=1) and renders the placeholder UI (ffmpeg screenshot evidence)
- **1-004**: tauri.conf.json identity fields exactly net.aqvn.workspace / AQVN Workspace
- **1-005**: rg whitelabel gate: zero matches outside docs/rfc-spec/

## Goals

- Bind feature `tauri-scaffold-branding` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change1-tauri-scaffold-branding/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-SCAF-001`
- `VAL-SCAF-002`
- `VAL-SCAF-003`
- `VAL-SCAF-004`
- `VAL-SCAF-005`
- `VAL-SCAF-006`
- `VAL-SCAF-013`
- `VAL-SCAF-014`
