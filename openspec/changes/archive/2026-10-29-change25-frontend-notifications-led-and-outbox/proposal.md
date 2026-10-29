# Proposal: change25-frontend-notifications-led-and-outbox

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `frontend-notifications-led-and-outbox` (milestone `frontend-chat-experience`) to an openspec change. The feature description:

> Implement OS desktop notifications when task completes while document.hidden, 3-state LED indicator (Online/Polling/Offline), and localStorage['aqvn_outbox'] queue with auto-drain on connect.

Mission: `d398e923-10d9-46b0-af7c-32a71e95a347`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **25-001**: applyProgress triggers invokeIpc('notify_task_complete') when wasActive && !state.active and document.hidden is true
- **25-002**: #connection-pill shows 🟢 online, 🟡 polling, and 🔴 offline
- **25-003**: Sending a message while offline queues it into localStorage['aqvn_outbox']
- **25-004**: Socket connect event automatically flushes and sends queued outbox messages

## Goals

- Bind feature `frontend-notifications-led-and-outbox` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change25-frontend-notifications-led-and-outbox/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-CONN-001`
- `VAL-CONN-002`
- `VAL-CONN-003`
