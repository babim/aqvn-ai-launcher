# Tasks: change37-feat-web-asset-cache

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [x] C37-001: Static asset requests (.ttf, .woff2, .css, .js) are cached locally under app cache dir
- [x] C37-002: Subsequent requests are served instantly from disk cache without network roundtrips
- [x] C37-003: Cache hit header or indicator verified via integration test

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C37-001 | Static asset requests (.ttf, .woff2, .css, .js) are cached locally under app cache dir | aqvn-opsx-doit | `surf:37-feat-web-asset-cache` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-WEB-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-WEB-002 pass. | — | P2 | 1 | native-subagents | complete |
| C37-002 | Subsequent requests are served instantly from disk cache without network roundtrips | aqvn-opsx-doit | `surf:37-feat-web-asset-cache` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-WEB-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-WEB-002 pass. | `C37-001` | P2 | 1 | native-subagents | complete |
| C37-003 | Cache hit header or indicator verified via integration test | aqvn-opsx-doit | `surf:37-feat-web-asset-cache` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-WEB-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-WEB-002 pass. | `C37-002` | P2 | 1 | native-subagents | complete |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:37-feat-web-asset-cache` | `C37-001`, `C37-002`, `C37-003` | covered |

## Dependency graph

- `C37-001` -> `C37-002`
- `C37-002` -> `C37-003`

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

- Task ID: `C37-001`
- Reason: first atom on the critical path
