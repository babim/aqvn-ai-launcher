# Tasks: change40-feat-gateway-outbound-client

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [x] C40-001: Outbound connection established to /ws without requiring open inbound ports
- [x] C40-002: Sends connector_hello declaring scopes: files, code_execution, browser, computer_use
- [x] C40-003: Maintains heartbeat pings and auto-reconnects on disconnection

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C40-001 | Outbound connection established to /ws without requiring open inbound ports | aqvn-opsx-doit | `surf:40-feat-gateway-outbound-client` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-GATEWAY-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-GATEWAY-001 pass. | — | P2 | 1 | native-subagents | planned |
| C40-002 | Sends connector_hello declaring scopes: files, code_execution, browser, computer_use | aqvn-opsx-doit | `surf:40-feat-gateway-outbound-client` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-GATEWAY-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-GATEWAY-001 pass. | `C40-001` | P2 | 1 | native-subagents | planned |
| C40-003 | Maintains heartbeat pings and auto-reconnects on disconnection | aqvn-opsx-doit | `surf:40-feat-gateway-outbound-client` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-GATEWAY-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-GATEWAY-001 pass. | `C40-002` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:40-feat-gateway-outbound-client` | `C40-001`, `C40-002`, `C40-003` | covered |

## Dependency graph

- `C40-001` -> `C40-002`
- `C40-002` -> `C40-003`

## Granularity gate

- Status: pass
- One task per affected skill: yes
- No umbrella-only coverage: yes
- Dependency graph acyclic: yes
- Complexity expansion complete: yes (every atom complexity 1 < 7)

## Execution lane

- Implementation lane shape: bounded-single-lane
- Surface hint: native-subagents
- Owner skill: aqvn-opsx-doit

## Next task hint

- Task ID: `C40-001`
- Reason: first atom on the critical path
