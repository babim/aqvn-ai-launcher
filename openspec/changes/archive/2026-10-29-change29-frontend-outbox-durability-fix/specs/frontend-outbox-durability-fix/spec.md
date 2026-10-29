# Spec delta: change29-frontend-outbox-durability-fix

## ADDED Requirements

### Requirement: change29-frontend-outbox-durability-fix binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `frontend-outbox-durability-fix`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change29-frontend-outbox-durability-fix-two-overlapping-flushoutbox-calls-send-e
- GIVEN the bridge script is invoked with `--feature-id frontend-outbox-durability-fix`
- WHEN the change scaffold is produced
- THEN the behavior "Two overlapping flushOutbox calls send exactly one message_async per queued entry" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change29-frontend-outbox-durability-fix-a-queued-entry-is-claimed-before-sending
- GIVEN the bridge script is invoked with `--feature-id frontend-outbox-durability-fix`
- WHEN the change scaffold is produced
- THEN the behavior "A queued entry is claimed before sending so a reconnect cannot re-send it" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change29-frontend-outbox-durability-fix-the-outbox-is-capped-by-entry-count-and-
- GIVEN the bridge script is invoked with `--feature-id frontend-outbox-durability-fix`
- WHEN the change scaffold is produced
- THEN the behavior "The outbox is capped by entry count and/or total bytes" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change29-frontend-outbox-durability-fix-when-localstorage-setitem-throws-a-user-
- GIVEN the bridge script is invoked with `--feature-id frontend-outbox-durability-fix`
- WHEN the change scaffold is produced
- THEN the behavior "When localStorage.setItem throws, a user-visible error is surfaced and the row is not left claiming delivery" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change29-frontend-outbox-durability-fix-node-tools-tests-test-chat-handlers-mjs-
- GIVEN the bridge script is invoked with `--feature-id frontend-outbox-durability-fix`
- WHEN the change scaffold is produced
- THEN the behavior "node tools/tests/test_chat_handlers.mjs passes with the new assertions" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change29-frontend-outbox-durability-fix-cargo-test-clippy-d-warnings-and-cargo-f
- GIVEN the bridge script is invoked with `--feature-id frontend-outbox-durability-fix`
- WHEN the change scaffold is produced
- THEN the behavior "cargo test, clippy -D warnings and cargo fmt --check remain clean" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
