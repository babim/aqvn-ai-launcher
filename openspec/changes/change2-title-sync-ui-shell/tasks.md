# Tasks: change2-title-sync-ui-shell

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [ ] C2-001: rg finds literal document.title assignment in ui/
- [ ] C2-002: Runtime native window title == AQVN AI Workspace under Xvfb
- [ ] C2-003: Title survives navigation index→settings→index round-trip

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C2-001 | rg finds literal document.title assignment in ui/ | aqvn-opsx-doit | `surf:2-title-sync-ui-shell` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-SCAF-007. Seeded RED: behavior unmet. GREEN: behavior met and VAL-SCAF-007 pass. | — | P2 | 1 | native-subagents | planned |
| C2-002 | Runtime native window title == AQVN AI Workspace under Xvfb | aqvn-opsx-doit | `surf:2-title-sync-ui-shell` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-SCAF-008. Seeded RED: behavior unmet. GREEN: behavior met and VAL-SCAF-008 pass. | `C2-001` | P2 | 1 | native-subagents | planned |
| C2-003 | Title survives navigation index→settings→index round-trip | aqvn-opsx-doit | `surf:2-title-sync-ui-shell` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-SCAF-009. Seeded RED: behavior unmet. GREEN: behavior met and VAL-SCAF-009 pass. | `C2-002` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:2-title-sync-ui-shell` | `C2-001`, `C2-002`, `C2-003` | covered |

## Dependency graph

- `C2-001` -> `C2-002`
- `C2-002` -> `C2-003`

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

- Task ID: `C2-001`
- Reason: first atom on the critical path
