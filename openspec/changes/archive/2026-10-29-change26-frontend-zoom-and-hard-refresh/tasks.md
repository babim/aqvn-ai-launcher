# Tasks: change26-frontend-zoom-and-hard-refresh

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [x] C26-001: Ctrl + increases zoom, Ctrl - decreases zoom, Ctrl 0 resets zoom to 1.0
- [x] C26-002: Zoom setting is stored in localStorage['aqvn_zoom'] and applied on init
- [x] C26-003: Hard refresh clears message RAM state, re-fetches CSRF, reconnects Socket.IO, and reloads canvas iframe

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C26-001 | Ctrl + increases zoom, Ctrl - decreases zoom, Ctrl 0 resets zoom to 1.0 | aqvn-opsx-doit | `surf:26-frontend-zoom-and-hard-refresh` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-ZOOM-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-ZOOM-001 pass. | — | P2 | 1 | native-subagents | planned |
| C26-002 | Zoom setting is stored in localStorage['aqvn_zoom'] and applied on init | aqvn-opsx-doit | `surf:26-frontend-zoom-and-hard-refresh` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-ZOOM-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-ZOOM-002 pass. | `C26-001` | P2 | 1 | native-subagents | planned |
| C26-003 | Hard refresh clears message RAM state, re-fetches CSRF, reconnects Socket.IO, and reloads canvas iframe | aqvn-opsx-doit | `surf:26-frontend-zoom-and-hard-refresh` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-ZOOM-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-ZOOM-001 pass. | `C26-002` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:26-frontend-zoom-and-hard-refresh` | `C26-001`, `C26-002`, `C26-003` | covered |

## Dependency graph

- `C26-001` -> `C26-002`
- `C26-002` -> `C26-003`

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

- Task ID: `C26-001`
- Reason: first atom on the critical path
