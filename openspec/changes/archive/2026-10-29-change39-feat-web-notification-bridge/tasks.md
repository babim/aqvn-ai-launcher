# Tasks: change39-feat-web-notification-bridge

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [x] C39-001: Client bridge script detects task completion from WebSocket/DOM state
- [x] C39-002: Dispatches Tauri IPC notify_task_complete when document.hidden === true
- [x] C39-003: Updates system tray icon state between idle and busy

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C39-001 | Client bridge script detects task completion from WebSocket/DOM state | aqvn-opsx-doit | `surf:39-feat-web-notification-bridge` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-WEB-004. Seeded RED: behavior unmet. GREEN: behavior met and VAL-WEB-004 pass. | — | P2 | 1 | native-subagents | complete |
| C39-002 | Dispatches Tauri IPC notify_task_complete when document.hidden === true | aqvn-opsx-doit | `surf:39-feat-web-notification-bridge` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-WEB-004. Seeded RED: behavior unmet. GREEN: behavior met and VAL-WEB-004 pass. | `C39-001` | P2 | 1 | native-subagents | complete |
| C39-003 | Updates system tray icon state between idle and busy | aqvn-opsx-doit | `surf:39-feat-web-notification-bridge` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-WEB-004. Seeded RED: behavior unmet. GREEN: behavior met and VAL-WEB-004 pass. | `C39-002` | P2 | 1 | native-subagents | complete |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:39-feat-web-notification-bridge` | `C39-001`, `C39-002`, `C39-003` | covered |

## Dependency graph

- `C39-001` -> `C39-002`
- `C39-002` -> `C39-003`

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

- Task ID: `C39-001`
- Reason: first atom on the critical path
