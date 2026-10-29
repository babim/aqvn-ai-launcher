# Spec delta: change25-frontend-notifications-led-and-outbox

## ADDED Requirements

### Requirement: change25-frontend-notifications-led-and-outbox binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `frontend-notifications-led-and-outbox`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change25-frontend-notifications-led-and-outbox-applyprogress-triggers-invokeipc-notify-
- GIVEN the bridge script is invoked with `--feature-id frontend-notifications-led-and-outbox`
- WHEN the change scaffold is produced
- THEN the behavior "applyProgress triggers invokeIpc('notify_task_complete') when wasActive && !state.active and document.hidden is true" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change25-frontend-notifications-led-and-outbox-connection-pill-shows-online-polling-and
- GIVEN the bridge script is invoked with `--feature-id frontend-notifications-led-and-outbox`
- WHEN the change scaffold is produced
- THEN the behavior "#connection-pill shows 🟢 online, 🟡 polling, and 🔴 offline" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change25-frontend-notifications-led-and-outbox-sending-a-message-while-offline-queues-i
- GIVEN the bridge script is invoked with `--feature-id frontend-notifications-led-and-outbox`
- WHEN the change scaffold is produced
- THEN the behavior "Sending a message while offline queues it into localStorage['aqvn_outbox']" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change25-frontend-notifications-led-and-outbox-socket-connect-event-automatically-flush
- GIVEN the bridge script is invoked with `--feature-id frontend-notifications-led-and-outbox`
- WHEN the change scaffold is produced
- THEN the behavior "Socket connect event automatically flushes and sends queued outbox messages" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
