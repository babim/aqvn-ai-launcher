# Tasks: change11-cross-area-integration

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [ ] C11-001: Journeys A+B pass end-to-end with screenshots/logs
- [ ] C11-002: Restart → settings + listeners restored from saved config
- [ ] C11-003: Whitelabel sweep clean on all six named surfaces
- [ ] C11-004: Toggle OFF closes both control ports, chat unaffected; chat stable under concurrent RFC load

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C11-001 | Journeys A+B pass end-to-end with screenshots/logs | aqvn-opsx-doit | `surf:11-cross-area-integration` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CROSS-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CROSS-001 pass. | — | P2 | 1 | native-subagents | planned |
| C11-002 | Restart → settings + listeners restored from saved config | aqvn-opsx-doit | `surf:11-cross-area-integration` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CROSS-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CROSS-002 pass. | `C11-001` | P2 | 1 | native-subagents | planned |
| C11-003 | Whitelabel sweep clean on all six named surfaces | aqvn-opsx-doit | `surf:11-cross-area-integration` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CROSS-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CROSS-003 pass. | `C11-002` | P2 | 1 | native-subagents | planned |
| C11-004 | Toggle OFF closes both control ports, chat unaffected; chat stable under concurrent RFC load | aqvn-opsx-doit | `surf:11-cross-area-integration` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CROSS-004, VAL-CROSS-005, VAL-CROSS-006. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CROSS-004, VAL-CROSS-005, VAL-CROSS-006 pass. | `C11-003` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:11-cross-area-integration` | `C11-001`, `C11-002`, `C11-003`, `C11-004` | covered |

## Dependency graph

- `C11-001` -> `C11-002`
- `C11-002` -> `C11-003`
- `C11-003` -> `C11-004`

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

- Task ID: `C11-001`
- Reason: first atom on the critical path
