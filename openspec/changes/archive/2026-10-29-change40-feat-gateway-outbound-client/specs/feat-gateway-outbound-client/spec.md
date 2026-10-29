# Spec delta: change40-feat-gateway-outbound-client

## ADDED Requirements

### Requirement: change40-feat-gateway-outbound-client binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `feat-gateway-outbound-client`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change40-feat-gateway-outbound-client-outbound-connection-established-to-ws-wi
- GIVEN the bridge script is invoked with `--feature-id feat-gateway-outbound-client`
- WHEN the change scaffold is produced
- THEN the behavior "Outbound connection established to /ws without requiring open inbound ports" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change40-feat-gateway-outbound-client-sends-connector-hello-declaring-scopes-f
- GIVEN the bridge script is invoked with `--feature-id feat-gateway-outbound-client`
- WHEN the change scaffold is produced
- THEN the behavior "Sends connector_hello declaring scopes: files, code_execution, browser, computer_use" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change40-feat-gateway-outbound-client-maintains-heartbeat-pings-and-auto-recon
- GIVEN the bridge script is invoked with `--feature-id feat-gateway-outbound-client`
- WHEN the change scaffold is produced
- THEN the behavior "Maintains heartbeat pings and auto-reconnects on disconnection" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
