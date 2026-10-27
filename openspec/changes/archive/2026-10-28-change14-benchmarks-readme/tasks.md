# Tasks: change14-benchmarks-readme

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [ ] C14-001: benchmark.sh runs all 5 gates, prints table, correct exit code, no orphans
- [ ] C14-002: Real measured numbers recorded; all gates PASS (or honest failure report to orchestrator)
- [ ] C14-003: README complete for all platforms + security model + limitations; whitelabel grep clean

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C14-001 | benchmark.sh runs all 5 gates, prints table, correct exit code, no orphans | aqvn-opsx-doit | `surf:14-benchmarks-readme` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-BENCH-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-BENCH-001 pass. | — | P2 | 1 | native-subagents | planned |
| C14-002 | Real measured numbers recorded; all gates PASS (or honest failure report to orchestrator) | aqvn-opsx-doit | `surf:14-benchmarks-readme` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-BENCH-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-BENCH-002 pass. | `C14-001` | P2 | 1 | native-subagents | planned |
| C14-003 | README complete for all platforms + security model + limitations; whitelabel grep clean | aqvn-opsx-doit | `surf:14-benchmarks-readme` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-BENCH-003, VAL-BENCH-004, VAL-BENCH-005, VAL-BENCH-006, VAL-CI-005. Seeded RED: behavior unmet. GREEN: behavior met and VAL-BENCH-003, VAL-BENCH-004, VAL-BENCH-005, VAL-BENCH-006, VAL-CI-005 pass. | `C14-002` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:14-benchmarks-readme` | `C14-001`, `C14-002`, `C14-003` | covered |

## Dependency graph

- `C14-001` -> `C14-002`
- `C14-002` -> `C14-003`

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

- Task ID: `C14-001`
- Reason: first atom on the critical path
