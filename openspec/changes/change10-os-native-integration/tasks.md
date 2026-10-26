# Tasks: change10-os-native-integration

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [ ] C10-001: Tray visible in rig screenshot with Show/Hide + Quit menu; menu toggles window; Quit closes app + ports
- [ ] C10-002: Busy/idle icon switch driven by chat activity
- [ ] C10-003: Alt+Space toggles window under X11/Xvfb; grab logged; fallback documented
- [ ] C10-004: Notification fires on task completion (evidence path recorded)
- [ ] C10-005: External link opens system browser; webview stays on app pages

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C10-001 | Tray visible in rig screenshot with Show/Hide + Quit menu; menu toggles window; Quit closes app + ports | aqvn-opsx-doit | `surf:10-os-native-integration` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-OS-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-OS-001 pass. | — | P2 | 1 | native-subagents | planned |
| C10-002 | Busy/idle icon switch driven by chat activity | aqvn-opsx-doit | `surf:10-os-native-integration` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-OS-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-OS-002 pass. | `C10-001` | P2 | 1 | native-subagents | planned |
| C10-003 | Alt+Space toggles window under X11/Xvfb; grab logged; fallback documented | aqvn-opsx-doit | `surf:10-os-native-integration` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-OS-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-OS-003 pass. | `C10-002` | P2 | 1 | native-subagents | planned |
| C10-004 | Notification fires on task completion (evidence path recorded) | aqvn-opsx-doit | `surf:10-os-native-integration` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-OS-004. Seeded RED: behavior unmet. GREEN: behavior met and VAL-OS-004 pass. | `C10-003` | P2 | 1 | native-subagents | planned |
| C10-005 | External link opens system browser; webview stays on app pages | aqvn-opsx-doit | `surf:10-os-native-integration` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-OS-005, VAL-OS-006, VAL-OS-007, VAL-OS-008. Seeded RED: behavior unmet. GREEN: behavior met and VAL-OS-005, VAL-OS-006, VAL-OS-007, VAL-OS-008 pass. | `C10-004` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:10-os-native-integration` | `C10-001`, `C10-002`, `C10-003`, `C10-004`, `C10-005` | covered |

## Dependency graph

- `C10-001` -> `C10-002`
- `C10-002` -> `C10-003`
- `C10-003` -> `C10-004`
- `C10-004` -> `C10-005`

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

- Task ID: `C10-001`
- Reason: first atom on the critical path
