# Spec delta: change18-chat-harness-model-stub

## ADDED Requirements

### Requirement: change18-chat-harness-model-stub binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `chat-harness-model-stub`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change18-chat-harness-model-stub-tools-model-stub-server-py-serves-openai
- GIVEN the bridge script is invoked with `--feature-id chat-harness-model-stub`
- WHEN the change scaffold is produced
- THEN the behavior "tools/model_stub_server.py serves OpenAI-compatible completions on loopback" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change18-chat-harness-model-stub-harness-config-points-chat-model-at-stub
- GIVEN the bridge script is invoked with `--feature-id chat-harness-model-stub`
- WHEN the change scaffold is produced
- THEN the behavior "Harness config points chat_model at stub (config files only, no source edits)" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change18-chat-harness-model-stub-chat-e2e-sh-flow-yields-an-assistant-rec
- GIVEN the bridge script is invoked with `--feature-id chat-harness-model-stub`
- WHEN the change scaffold is produced
- THEN the behavior "chat_e2e.sh flow yields an assistant record with non-empty content (exit 0)" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change18-chat-harness-model-stub-clean-shutdown-stub-harness-app-stopped-
- GIVEN the bridge script is invoked with `--feature-id chat-harness-model-stub`
- WHEN the change scaffold is produced
- THEN the behavior "Clean shutdown: stub + harness + app stopped, ports free, no orphans" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
