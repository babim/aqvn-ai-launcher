# Tasks: change44-feat-gateway-desktop-handler

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [x] C44-001: Captures host screen or active window on capture op
- [x] C44-002: Injects mouse movement, clicks, and keystrokes on click/type/key ops
- [x] C44-003: Replies with connector_computer_use_op_result

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C44-001 | Captures host screen or active window on capture op | aqvn-opsx-doit | `surf:44-feat-gateway-desktop-handler` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-GATEWAY-005. Seeded RED: behavior unmet. GREEN: behavior met and VAL-GATEWAY-005 pass. | — | P2 | 1 | native-subagents | complete |
| C44-002 | Injects mouse movement, clicks, and keystrokes on click/type/key ops | aqvn-opsx-doit | `surf:44-feat-gateway-desktop-handler` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-GATEWAY-005. Seeded RED: behavior unmet. GREEN: behavior met and VAL-GATEWAY-005 pass. | `C44-001` | P2 | 1 | native-subagents | complete |
| C44-003 | Replies with connector_computer_use_op_result | aqvn-opsx-doit | `surf:44-feat-gateway-desktop-handler` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-GATEWAY-005. Seeded RED: behavior unmet. GREEN: behavior met and VAL-GATEWAY-005 pass. | `C44-002` | P2 | 1 | native-subagents | complete |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:44-feat-gateway-desktop-handler` | `C44-001`, `C44-002`, `C44-003` | covered |

## Dependency graph

- `C44-001` -> `C44-002`
- `C44-002` -> `C44-003`

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

- Task ID: `C44-001`
- Reason: first atom on the critical path
