# Tasks: change41-feat-gateway-file-handler

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [ ] C41-001: Handles connector_file_op for read, write, patch, and list
- [ ] C41-002: Rejects path traversal or symlink escapes outside sandbox
- [ ] C41-003: Replies with connector_file_op_result over WebSocket

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C41-001 | Handles connector_file_op for read, write, patch, and list | aqvn-opsx-doit | `surf:41-feat-gateway-file-handler` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-GATEWAY-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-GATEWAY-002 pass. | — | P2 | 1 | native-subagents | complete |
| C41-002 | Rejects path traversal or symlink escapes outside sandbox | aqvn-opsx-doit | `surf:41-feat-gateway-file-handler` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-GATEWAY-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-GATEWAY-002 pass. | `C41-001` | P2 | 1 | native-subagents | complete |
| C41-003 | Replies with connector_file_op_result over WebSocket | aqvn-opsx-doit | `surf:41-feat-gateway-file-handler` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-GATEWAY-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-GATEWAY-002 pass. | `C41-002` | P2 | 1 | native-subagents | complete |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:41-feat-gateway-file-handler` | `C41-001`, `C41-002`, `C41-003` | covered |

## Dependency graph

- `C41-001` -> `C41-002`
- `C41-002` -> `C41-003`

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

- Task ID: `C41-001`
- Reason: first atom on the critical path
