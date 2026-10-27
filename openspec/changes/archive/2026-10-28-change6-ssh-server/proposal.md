# Proposal: change6-ssh-server

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `ssh-server` (milestone `rfc-ssh-bridge`) to an openspec change. The feature description:

> Embedded SSH server (russh) on 0.0.0.0:55022 (port from config, hot-appliable), gated by the same master toggle (default OFF = no listener). Password auth against ssh_password setting; wrong password rejected. Session: PTY honoring CLIENT-requested dims (test 100x50 and 80x24), spawn bash (Linux), cwd = first allowed path. guard.rs command denylist (secondary layer): rm -rf / variants, mkfs, dd of=/dev/*, fork bomb, shutdown/reboot — refused, NOT executed, error surfaced to client, session stays alive; benign commands unimpeded. Unit tests for denylist population + auth; integration test drives a real SSH client (russh client or ssh CLI with sshpass, or harness-venv paramiko).

Mission: `c82a03df-dfc7-438f-b44b-9089dcc40a8f`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **6-001**: Listener only when toggle ON; port configurable + hot-applied
- **6-002**: Correct password → PTY shell (requested geometry, cwd = first allowed path); wrong password → rejected
- **6-003**: Denylist blocks named destructive population; benign passes; session survives refusal
- **6-004**: Security invariant: denylist is best-effort secondary — never weakens auth/toggle

## Goals

- Bind feature `ssh-server` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change6-ssh-server/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-SSH-001`
- `VAL-SSH-002`
- `VAL-SSH-003`
- `VAL-SSH-004`
- `VAL-SSH-005`
- `VAL-SSH-006`
- `VAL-SSH-009`
