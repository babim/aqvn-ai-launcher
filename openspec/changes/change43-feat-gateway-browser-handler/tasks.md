# Tasks: change43-feat-gateway-browser-handler

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [ ] C43-001: Connects to local Chrome CDP at http://127.0.0.1:9222
- [ ] C43-002: Dispatches open, navigate, click, type, screenshot, and evaluate actions
- [ ] C43-003: Handles open_remote_debugging by launching browser with remote debugging

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C43-001 | Connects to local Chrome CDP at http://127.0.0.1:9222 | aqvn-opsx-doit | `surf:43-feat-gateway-browser-handler` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-GATEWAY-004. Seeded RED: behavior unmet. GREEN: behavior met and VAL-GATEWAY-004 pass. | — | P2 | 1 | native-subagents | completed |
| C43-002 | Dispatches open, navigate, click, type, screenshot, and evaluate actions | aqvn-opsx-doit | `surf:43-feat-gateway-browser-handler` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-GATEWAY-004. Seeded RED: behavior unmet. GREEN: behavior met and VAL-GATEWAY-004 pass. | `C43-001` | P2 | 1 | native-subagents | completed |
| C43-003 | Handles open_remote_debugging by launching browser with remote debugging | aqvn-opsx-doit | `surf:43-feat-gateway-browser-handler` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-GATEWAY-004. Seeded RED: behavior unmet. GREEN: behavior met and VAL-GATEWAY-004 pass. | `C43-002` | P2 | 1 | native-subagents | completed |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:43-feat-gateway-browser-handler` | `C43-001`, `C43-002`, `C43-003` | covered |

## Dependency graph

- `C43-001` -> `C43-002`
- `C43-002` -> `C43-003`

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

- Task ID: `C43-001`
- Reason: first atom on the critical path
