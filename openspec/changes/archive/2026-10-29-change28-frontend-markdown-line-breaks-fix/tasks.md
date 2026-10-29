# Tasks: change28-frontend-markdown-line-breaks-fix

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [x] C28-001: marked.parse is invoked with breaks:true so a single newline becomes a <br> in rendered output
- [x] C28-002: A two-line plain message renders with its line break preserved
- [x] C28-003: node tools/tests/test_chat_handlers.mjs passes with the new assertion
- [x] C28-004: cargo test, clippy -D warnings and cargo fmt --check remain clean

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C28-001 | marked.parse is invoked with breaks:true so a single newline becomes a <br> in rendered output | aqvn-opsx-doit | `surf:28-frontend-markdown-line-breaks-fix` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-MD-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-MD-002 pass. | — | P2 | 1 | native-subagents | planned |
| C28-002 | A two-line plain message renders with its line break preserved | aqvn-opsx-doit | `surf:28-frontend-markdown-line-breaks-fix` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-MD-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-MD-002 pass. | `C28-001` | P2 | 1 | native-subagents | planned |
| C28-003 | node tools/tests/test_chat_handlers.mjs passes with the new assertion | aqvn-opsx-doit | `surf:28-frontend-markdown-line-breaks-fix` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-MD-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-MD-002 pass. | `C28-002` | P2 | 1 | native-subagents | planned |
| C28-004 | cargo test, clippy -D warnings and cargo fmt --check remain clean | aqvn-opsx-doit | `surf:28-frontend-markdown-line-breaks-fix` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-MD-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-MD-002 pass. | `C28-003` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:28-frontend-markdown-line-breaks-fix` | `C28-001`, `C28-002`, `C28-003`, `C28-004` | covered |

## Dependency graph

- `C28-001` -> `C28-002`
- `C28-002` -> `C28-003`
- `C28-003` -> `C28-004`

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

- Task ID: `C28-001`
- Reason: first atom on the critical path
