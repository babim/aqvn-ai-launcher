# Tasks: change25-frontend-notifications-led-and-outbox

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [x] C25-001: applyProgress triggers invokeIpc('notify_task_complete') when wasActive && !state.active and document.hidden is true
- [x] C25-002: #connection-pill shows 🟢 online, 🟡 polling, and 🔴 offline
- [x] C25-003: Sending a message while offline queues it into localStorage['aqvn_outbox']
- [x] C25-004: Socket connect event automatically flushes and sends queued outbox messages

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C25-001 | applyProgress triggers invokeIpc('notify_task_complete') when wasActive && !state.active and document.hidden is true | aqvn-opsx-doit | `surf:25-frontend-notifications-led-and-outbox` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CONN-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CONN-001 pass. | — | P2 | 1 | native-subagents | planned |
| C25-002 | #connection-pill shows 🟢 online, 🟡 polling, and 🔴 offline | aqvn-opsx-doit | `surf:25-frontend-notifications-led-and-outbox` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CONN-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CONN-002 pass. | `C25-001` | P2 | 1 | native-subagents | planned |
| C25-003 | Sending a message while offline queues it into localStorage['aqvn_outbox'] | aqvn-opsx-doit | `surf:25-frontend-notifications-led-and-outbox` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CONN-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CONN-003 pass. | `C25-002` | P2 | 1 | native-subagents | planned |
| C25-004 | Socket connect event automatically flushes and sends queued outbox messages | aqvn-opsx-doit | `surf:25-frontend-notifications-led-and-outbox` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CONN-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CONN-001 pass. | `C25-003` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:25-frontend-notifications-led-and-outbox` | `C25-001`, `C25-002`, `C25-003`, `C25-004` | covered |

## Dependency graph

- `C25-001` -> `C25-002`
- `C25-002` -> `C25-003`
- `C25-003` -> `C25-004`

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

- Task ID: `C25-001`
- Reason: first atom on the critical path
