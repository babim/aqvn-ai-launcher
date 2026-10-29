# Proposal: change29-frontend-outbox-durability-fix

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `frontend-outbox-durability-fix` (milestone `frontend-chat-experience`) to an openspec change. The feature description:

> Scrutiny fix (frontend-notifications-led-and-outbox): two confirmed outbox defects. (1) flushOutbox has no in-flight or claim-before-send guard, so a second socket connect during a pending drain re-sends the same queued message. (2) The outbox is unbounded and every storage failure is swallowed, so a queued message is silently lost while the UI promises delivery. Add a flushing guard (or claim-then-send), cap the queue by entry count and/or bytes, surface a user-visible error when storage is unavailable, and add tests for overlapping flushes and throwing localStorage.

Mission: `d398e923-10d9-46b0-af7c-32a71e95a347`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **29-001**: Two overlapping flushOutbox calls send exactly one message_async per queued entry
- **29-002**: A queued entry is claimed before sending so a reconnect cannot re-send it
- **29-003**: The outbox is capped by entry count and/or total bytes
- **29-004**: When localStorage.setItem throws, a user-visible error is surfaced and the row is not left claiming delivery
- **29-005**: node tools/tests/test_chat_handlers.mjs passes with the new assertions
- **29-006**: cargo test, clippy -D warnings and cargo fmt --check remain clean

## Goals

- Bind feature `frontend-outbox-durability-fix` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change29-frontend-outbox-durability-fix/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-CONN-003`
