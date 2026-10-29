# Spec delta: change28-frontend-markdown-line-breaks-fix

## ADDED Requirements

### Requirement: change28-frontend-markdown-line-breaks-fix binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `frontend-markdown-line-breaks-fix`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change28-frontend-markdown-line-breaks-fix-marked-parse-is-invoked-with-breaks-true
- GIVEN the bridge script is invoked with `--feature-id frontend-markdown-line-breaks-fix`
- WHEN the change scaffold is produced
- THEN the behavior "marked.parse is invoked with breaks:true so a single newline becomes a <br> in rendered output" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change28-frontend-markdown-line-breaks-fix-a-two-line-plain-message-renders-with-it
- GIVEN the bridge script is invoked with `--feature-id frontend-markdown-line-breaks-fix`
- WHEN the change scaffold is produced
- THEN the behavior "A two-line plain message renders with its line break preserved" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change28-frontend-markdown-line-breaks-fix-node-tools-tests-test-chat-handlers-mjs-
- GIVEN the bridge script is invoked with `--feature-id frontend-markdown-line-breaks-fix`
- WHEN the change scaffold is produced
- THEN the behavior "node tools/tests/test_chat_handlers.mjs passes with the new assertion" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change28-frontend-markdown-line-breaks-fix-cargo-test-clippy-d-warnings-and-cargo-f
- GIVEN the bridge script is invoked with `--feature-id frontend-markdown-line-breaks-fix`
- WHEN the change scaffold is produced
- THEN the behavior "cargo test, clippy -D warnings and cargo fmt --check remain clean" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
