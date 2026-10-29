# Tasks: change32-scrutiny-validator-backend-proxy

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [ ] C32-001: M32-T1: Validators pass (test, typecheck, lint)
- [ ] C32-002: M32-T2: Review subagents spawned for each feature
- [ ] C32-003: M32-T3: Findings synthesized into scrutiny report

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C32-001 | M32-T1: Validators pass (test, typecheck, lint) | aqvn-opsx-doit | `surf:32-scrutiny-validator-backend-proxy` | `(proposals-only-scaffold)` | script | No VAL-* mapped (fulfills[] empty). Behavior: "Validators pass (test, typecheck, lint)". Seeded RED: behavior unmet. GREEN: behavior met. | — | P2 | 1 | native-subagents | planned |
| C32-002 | M32-T2: Review subagents spawned for each feature | aqvn-opsx-doit | `surf:32-scrutiny-validator-backend-proxy` | `(proposals-only-scaffold)` | script | No VAL-* mapped (fulfills[] empty). Behavior: "Review subagents spawned for each feature". Seeded RED: behavior unmet. GREEN: behavior met. | `C32-001` | P2 | 1 | native-subagents | planned |
| C32-003 | M32-T3: Findings synthesized into scrutiny report | aqvn-opsx-doit | `surf:32-scrutiny-validator-backend-proxy` | `(proposals-only-scaffold)` | script | No VAL-* mapped (fulfills[] empty). Behavior: "Findings synthesized into scrutiny report". Seeded RED: behavior unmet. GREEN: behavior met. | `C32-002` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:32-scrutiny-validator-backend-proxy` | `C32-001`, `C32-002`, `C32-003` | covered |

## Dependency graph

- `C32-001` -> `C32-002`
- `C32-002` -> `C32-003`

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

- Task ID: `C32-001`
- Reason: first atom on the critical path
