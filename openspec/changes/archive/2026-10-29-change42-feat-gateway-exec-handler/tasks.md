# Tasks: change42-feat-gateway-exec-handler

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [x] C42-001: Handles connector_exec_op with runtime=terminal
- [x] C42-002: Executes shell commands inside stateful PTY session
- [x] C42-003: Streams output back to agent via connector_exec_op_result

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C42-001 | Handles connector_exec_op with runtime=terminal | aqvn-opsx-doit | `surf:42-feat-gateway-exec-handler` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-GATEWAY-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-GATEWAY-003 pass. | — | P2 | 1 | native-subagents | completed |
| C42-002 | Executes shell commands inside stateful PTY session | aqvn-opsx-doit | `surf:42-feat-gateway-exec-handler` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-GATEWAY-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-GATEWAY-003 pass. | `C42-001` | P2 | 1 | native-subagents | completed |
| C42-003 | Streams output back to agent via connector_exec_op_result | aqvn-opsx-doit | `surf:42-feat-gateway-exec-handler` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-GATEWAY-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-GATEWAY-003 pass. | `C42-002` | P2 | 1 | native-subagents | completed |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:42-feat-gateway-exec-handler` | `C42-001`, `C42-002`, `C42-003` | covered |

## Dependency graph

- `C42-001` -> `C42-002`
- `C42-002` -> `C42-003`

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

- Task ID: `C42-001`
- Reason: first atom on the critical path
