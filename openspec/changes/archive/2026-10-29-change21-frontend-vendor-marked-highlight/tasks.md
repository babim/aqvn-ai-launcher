# Tasks: change21-frontend-vendor-marked-highlight

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [x] C21-001: ui/vendor/marked.min.js is present and valid JS
- [x] C21-002: ui/vendor/highlight.min.js is present and valid JS
- [x] C21-003: ui/index.html loads marked.min.js and highlight.min.js before chat.js
- [x] C21-004: window.marked and window.hljs are defined in the webview window scope

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C21-001 | ui/vendor/marked.min.js is present and valid JS | aqvn-opsx-doit | `surf:21-frontend-vendor-marked-highlight` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-MD-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-MD-001 pass. | — | P2 | 1 | native-subagents | planned |
| C21-002 | ui/vendor/highlight.min.js is present and valid JS | aqvn-opsx-doit | `surf:21-frontend-vendor-marked-highlight` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-MD-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-MD-001 pass. | `C21-001` | P2 | 1 | native-subagents | planned |
| C21-003 | ui/index.html loads marked.min.js and highlight.min.js before chat.js | aqvn-opsx-doit | `surf:21-frontend-vendor-marked-highlight` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-MD-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-MD-001 pass. | `C21-002` | P2 | 1 | native-subagents | planned |
| C21-004 | window.marked and window.hljs are defined in the webview window scope | aqvn-opsx-doit | `surf:21-frontend-vendor-marked-highlight` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-MD-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-MD-001 pass. | `C21-003` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:21-frontend-vendor-marked-highlight` | `C21-001`, `C21-002`, `C21-003`, `C21-004` | covered |

## Dependency graph

- `C21-001` -> `C21-002`
- `C21-002` -> `C21-003`
- `C21-003` -> `C21-004`

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

- Task ID: `C21-001`
- Reason: first atom on the critical path
