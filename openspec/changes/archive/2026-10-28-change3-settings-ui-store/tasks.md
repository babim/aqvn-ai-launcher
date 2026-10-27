# Tasks: change3-settings-ui-store

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [ ] C3-001: All settings controls present in DOM with correct defaults (URL prefilled, toggle OFF)
- [ ] C3-002: Values persist across app restart via store plugin; store JSON contains changed keys
- [ ] C3-003: Store file path derived from identifier net.aqvn.workspace
- [ ] C3-004: Settings page loads and works in browser-mode fallback (agent-browser, zero console errors)

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C3-001 | All settings controls present in DOM with correct defaults (URL prefilled, toggle OFF) | aqvn-opsx-doit | `surf:3-settings-ui-store` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-SCAF-010. Seeded RED: behavior unmet. GREEN: behavior met and VAL-SCAF-010 pass. | — | P2 | 1 | native-subagents | planned |
| C3-002 | Values persist across app restart via store plugin; store JSON contains changed keys | aqvn-opsx-doit | `surf:3-settings-ui-store` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-SCAF-011. Seeded RED: behavior unmet. GREEN: behavior met and VAL-SCAF-011 pass. | `C3-001` | P2 | 1 | native-subagents | planned |
| C3-003 | Store file path derived from identifier net.aqvn.workspace | aqvn-opsx-doit | `surf:3-settings-ui-store` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-SCAF-012. Seeded RED: behavior unmet. GREEN: behavior met and VAL-SCAF-012 pass. | `C3-002` | P2 | 1 | native-subagents | planned |
| C3-004 | Settings page loads and works in browser-mode fallback (agent-browser, zero console errors) | aqvn-opsx-doit | `surf:3-settings-ui-store` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-SCAF-010. Seeded RED: behavior unmet. GREEN: behavior met and VAL-SCAF-010 pass. | `C3-003` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:3-settings-ui-store` | `C3-001`, `C3-002`, `C3-003`, `C3-004` | covered |

## Dependency graph

- `C3-001` -> `C3-002`
- `C3-002` -> `C3-003`
- `C3-003` -> `C3-004`

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

- Task ID: `C3-001`
- Reason: first atom on the critical path
