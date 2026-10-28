# Tasks: change38-feat-web-container-shell

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [x] C38-001: Webview displays the complete upstream web UI (Sidebar, Chat, DAG Canvas, Settings)
- [x] C38-002: Internal app routes and frames navigate inside the webview without opening external browser
- [x] C38-003: External hyperlinks open in system default browser via tauri-plugin-shell
- [x] C38-004: os_integration navigation tests pass cleanly

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C38-001 | Webview displays the complete upstream web UI (Sidebar, Chat, DAG Canvas, Settings) | aqvn-opsx-doit | `surf:38-feat-web-container-shell` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-WEB-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-WEB-003 pass. | — | P2 | 1 | native-subagents | complete |
| C38-002 | Internal app routes and frames navigate inside the webview without opening external browser | aqvn-opsx-doit | `surf:38-feat-web-container-shell` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-WEB-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-WEB-003 pass. | `C38-001` | P2 | 1 | native-subagents | complete |
| C38-003 | External hyperlinks open in system default browser via tauri-plugin-shell | aqvn-opsx-doit | `surf:38-feat-web-container-shell` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-WEB-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-WEB-003 pass. | `C38-002` | P2 | 1 | native-subagents | complete |
| C38-004 | os_integration navigation tests pass cleanly | aqvn-opsx-doit | `surf:38-feat-web-container-shell` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-WEB-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-WEB-003 pass. | `C38-003` | P2 | 1 | native-subagents | complete |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:38-feat-web-container-shell` | `C38-001`, `C38-002`, `C38-003`, `C38-004` | covered |

## Dependency graph

- `C38-001` -> `C38-002`
- `C38-002` -> `C38-003`
- `C38-003` -> `C38-004`

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

- Task ID: `C38-001`
- Reason: first atom on the critical path
