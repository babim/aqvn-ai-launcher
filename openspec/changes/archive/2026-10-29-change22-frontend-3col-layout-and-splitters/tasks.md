# Tasks: change22-frontend-3col-layout-and-splitters

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [x] C22-001: #sidebar contains session context list, New Chat button, and Server Switcher
- [x] C22-002: #canvas-pane contains tab header with 'Intel Suite', #close-canvas-btn, and iframe #canvas-frame
- [x] C22-003: #canvas-frame loads /plugins/aqvn_intel_suite/webui/canvas.html
- [x] C22-004: Alt+S and #toggle-sidebar-btn toggle #sidebar collapsed state with localStorage persistence
- [x] C22-005: Alt+C and #toggle-canvas-btn toggle #canvas-pane collapsed state with localStorage persistence
- [x] C22-006: Splitters support smooth horizontal drag resizing within defined bounds

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C22-001 | #sidebar contains session context list, New Chat button, and Server Switcher | aqvn-opsx-doit | `surf:22-frontend-3col-layout-and-splitters` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-LAYOUT-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-LAYOUT-001 pass. | — | P2 | 1 | native-subagents | planned |
| C22-002 | #canvas-pane contains tab header with 'Intel Suite', #close-canvas-btn, and iframe #canvas-frame | aqvn-opsx-doit | `surf:22-frontend-3col-layout-and-splitters` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-LAYOUT-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-LAYOUT-002 pass. | `C22-001` | P2 | 1 | native-subagents | planned |
| C22-003 | #canvas-frame loads /plugins/aqvn_intel_suite/webui/canvas.html | aqvn-opsx-doit | `surf:22-frontend-3col-layout-and-splitters` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-LAYOUT-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-LAYOUT-003 pass. | `C22-002` | P2 | 1 | native-subagents | planned |
| C22-004 | Alt+S and #toggle-sidebar-btn toggle #sidebar collapsed state with localStorage persistence | aqvn-opsx-doit | `surf:22-frontend-3col-layout-and-splitters` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-LAYOUT-004. Seeded RED: behavior unmet. GREEN: behavior met and VAL-LAYOUT-004 pass. | `C22-003` | P2 | 1 | native-subagents | planned |
| C22-005 | Alt+C and #toggle-canvas-btn toggle #canvas-pane collapsed state with localStorage persistence | aqvn-opsx-doit | `surf:22-frontend-3col-layout-and-splitters` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-LAYOUT-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-LAYOUT-001 pass. | `C22-004` | P2 | 1 | native-subagents | planned |
| C22-006 | Splitters support smooth horizontal drag resizing within defined bounds | aqvn-opsx-doit | `surf:22-frontend-3col-layout-and-splitters` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-LAYOUT-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-LAYOUT-002 pass. | `C22-005` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:22-frontend-3col-layout-and-splitters` | `C22-001`, `C22-002`, `C22-003`, `C22-004`, `C22-005`, `C22-006` | covered |

## Dependency graph

- `C22-001` -> `C22-002`
- `C22-002` -> `C22-003`
- `C22-003` -> `C22-004`
- `C22-004` -> `C22-005`
- `C22-005` -> `C22-006`

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

- Task ID: `C22-001`
- Reason: first atom on the critical path
