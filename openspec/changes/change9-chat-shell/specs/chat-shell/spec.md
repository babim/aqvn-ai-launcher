# Spec delta: change9-chat-shell

## ADDED Requirements

### Requirement: change9-chat-shell binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `chat-shell`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change9-chat-shell-agent-browser-login-send-streaming-rende
- GIVEN the bridge script is invoked with `--feature-id chat-shell`
- WHEN the change scaffold is produced
- THEN the behavior "agent-browser: login → send → streaming render (merge by id) → stop → new chat, zero console errors" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change9-chat-shell-harness-e2e-full-chat-round-trip-through
- GIVEN the bridge script is invoked with `--feature-id chat-shell`
- WHEN the change scaffold is produced
- THEN the behavior "Harness E2E: full chat round-trip through proxy against :5000" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change9-chat-shell-real-server-e2e-login-state-request-mess
- GIVEN the bridge script is invoked with `--feature-id chat-shell`
- WHEN the change scaffold is produced
- THEN the behavior "Real-server E2E: login + state_request + message_async + response records vs a0.matmagoc.com (creds redacted)" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change9-chat-shell-reconnect-re-handshakes-session-expiry-s
- GIVEN the bridge script is invoked with `--feature-id chat-shell`
- WHEN the change scaffold is produced
- THEN the behavior "Reconnect re-handshakes; session expiry surfaces re-login UI; browser-mode and real app consistent" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
