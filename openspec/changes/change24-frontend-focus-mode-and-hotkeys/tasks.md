# Tasks: change24-frontend-focus-mode-and-hotkeys

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [x] C24-001: F11 and Ctrl+Shift+F toggle .focus-mode class on body
- [x] C24-002: .focus-mode hides #sidebar, #canvas-pane, and splitters
- [x] C24-003: .focus-mode centers #chat-pane with max-width: 840px
- [x] C24-004: Input #message-input is focused immediately upon entering focus mode

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C24-001 | F11 and Ctrl+Shift+F toggle .focus-mode class on body | aqvn-opsx-doit | `surf:24-frontend-focus-mode-and-hotkeys` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-FOCUS-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-FOCUS-001 pass. | — | P2 | 1 | native-subagents | planned |
| C24-002 | .focus-mode hides #sidebar, #canvas-pane, and splitters | aqvn-opsx-doit | `surf:24-frontend-focus-mode-and-hotkeys` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-FOCUS-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-FOCUS-001 pass. | `C24-001` | P2 | 1 | native-subagents | planned |
| C24-003 | .focus-mode centers #chat-pane with max-width: 840px | aqvn-opsx-doit | `surf:24-frontend-focus-mode-and-hotkeys` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-FOCUS-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-FOCUS-001 pass. | `C24-002` | P2 | 1 | native-subagents | planned |
| C24-004 | Input #message-input is focused immediately upon entering focus mode | aqvn-opsx-doit | `surf:24-frontend-focus-mode-and-hotkeys` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-FOCUS-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-FOCUS-001 pass. | `C24-003` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:24-frontend-focus-mode-and-hotkeys` | `C24-001`, `C24-002`, `C24-003`, `C24-004` | covered |

## Dependency graph

- `C24-001` -> `C24-002`
- `C24-002` -> `C24-003`
- `C24-003` -> `C24-004`

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

- Task ID: `C24-001`
- Reason: first atom on the critical path
