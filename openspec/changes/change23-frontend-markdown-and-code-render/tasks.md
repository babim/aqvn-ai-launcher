# Tasks: change23-frontend-markdown-and-code-render

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [x] C23-001: renderContent parses Markdown using marked.parse
- [x] C23-002: Code blocks are highlighted via hljs and decorated with language badge and Copy button
- [x] C23-003: Copy button copies code to clipboard using navigator.clipboard.writeText with feedback state
- [x] C23-004: msg-code_exe and msg-tool styled with #0d1117 background and cyan $ EXECUTE branding
- [x] C23-005: Thought content renders in <details class='thought-step'>

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C23-001 | renderContent parses Markdown using marked.parse | aqvn-opsx-doit | `surf:23-frontend-markdown-and-code-render` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-MD-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-MD-002 pass. | — | P2 | 1 | native-subagents | planned |
| C23-002 | Code blocks are highlighted via hljs and decorated with language badge and Copy button | aqvn-opsx-doit | `surf:23-frontend-markdown-and-code-render` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-MD-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-MD-003 pass. | `C23-001` | P2 | 1 | native-subagents | planned |
| C23-003 | Copy button copies code to clipboard using navigator.clipboard.writeText with feedback state | aqvn-opsx-doit | `surf:23-frontend-markdown-and-code-render` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-MD-004. Seeded RED: behavior unmet. GREEN: behavior met and VAL-MD-004 pass. | `C23-002` | P2 | 1 | native-subagents | planned |
| C23-004 | msg-code_exe and msg-tool styled with #0d1117 background and cyan $ EXECUTE branding | aqvn-opsx-doit | `surf:23-frontend-markdown-and-code-render` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-MD-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-MD-002 pass. | `C23-003` | P2 | 1 | native-subagents | planned |
| C23-005 | Thought content renders in <details class='thought-step'> | aqvn-opsx-doit | `surf:23-frontend-markdown-and-code-render` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-MD-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-MD-003 pass. | `C23-004` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:23-frontend-markdown-and-code-render` | `C23-001`, `C23-002`, `C23-003`, `C23-004`, `C23-005` | covered |

## Dependency graph

- `C23-001` -> `C23-002`
- `C23-002` -> `C23-003`
- `C23-003` -> `C23-004`
- `C23-004` -> `C23-005`

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

- Task ID: `C23-001`
- Reason: first atom on the critical path
