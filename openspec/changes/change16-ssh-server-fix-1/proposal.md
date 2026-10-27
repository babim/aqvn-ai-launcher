# Proposal: change16-ssh-server-fix-1

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `ssh-server-fix-1` (milestone `rfc-ssh-bridge`) to an openspec change. The feature description:

> Scrutiny fix (blocking): SSH listener binds [IP]:55022 but ruling R2 requires wildcard 0.0.0.0:55022. Changes: (1) src-tauri/src/ssh/server.rs bind Ipv4Addr::UNSPECIFIED; (2) FAIL CLOSED when ssh_password is empty — refuse to bind (wildcard SSH with no password = catastrophic); (3) update tests/ssh_server.rs wildcard + empty-password tests; (4) document denylist blind spots (backslash variants, busybox/nohup prefixes, bash -c, eval, $()/backticks, Ctrl-C/Ctrl-U line-buffer desync) in missionDir library/ssh-server.md AND repo docs/rfc-spec/security-notes.md so no worker treats the guard as a security boundary (real boundary = password auth + master toggle); (5) rewrite library/ssh-server.md bind note to cite R2. Verify: cd src-tauri && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check. Conventional commit fix(ssh): ...

Mission: `c82a03df-dfc7-438f-b44b-9089dcc40a8f`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **16-001**: SSH binds 0.0.0.0:55022 when enabled AND ssh_password non-empty
- **16-002**: Empty ssh_password → no bind, fail-closed with log
- **16-003**: Denylist limitations documented in repo docs + missionDir library
- **16-004**: Full cargo test + clippy + fmt green; whitelabel gate clean

## Goals

- Bind feature `ssh-server-fix-1` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change16-ssh-server-fix-1/` during authoring.

## Validation Contract Assertions Fulfilled

(none declared)
