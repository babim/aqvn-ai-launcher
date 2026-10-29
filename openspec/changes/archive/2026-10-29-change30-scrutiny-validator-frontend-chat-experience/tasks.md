# Tasks: change30-scrutiny-validator-frontend-chat-experience

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [ ] C30-001: M30-T1: Validators pass (test, typecheck, lint)
- [ ] C30-002: M30-T2: Review subagents spawned for each feature
- [ ] C30-003: M30-T3: Findings synthesized into scrutiny report

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C30-001 | M30-T1: Validators pass (test, typecheck, lint) | aqvn-opsx-doit | `surf:30-scrutiny-validator-frontend-chat-experience` | `(proposals-only-scaffold)` | script | No VAL-* mapped (fulfills[] empty). Behavior: "Validators pass (test, typecheck, lint)". Seeded RED: behavior unmet. GREEN: behavior met. | — | P2 | 1 | native-subagents | planned |
| C30-002 | M30-T2: Review subagents spawned for each feature | aqvn-opsx-doit | `surf:30-scrutiny-validator-frontend-chat-experience` | `(proposals-only-scaffold)` | script | No VAL-* mapped (fulfills[] empty). Behavior: "Review subagents spawned for each feature". Seeded RED: behavior unmet. GREEN: behavior met. | `C30-001` | P2 | 1 | native-subagents | planned |
| C30-003 | M30-T3: Findings synthesized into scrutiny report | aqvn-opsx-doit | `surf:30-scrutiny-validator-frontend-chat-experience` | `(proposals-only-scaffold)` | script | No VAL-* mapped (fulfills[] empty). Behavior: "Findings synthesized into scrutiny report". Seeded RED: behavior unmet. GREEN: behavior met. | `C30-002` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:30-scrutiny-validator-frontend-chat-experience` | `C30-001`, `C30-002`, `C30-003` | covered |

## Dependency graph

- `C30-001` -> `C30-002`
- `C30-002` -> `C30-003`

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

- Task ID: `C30-001`
- Reason: first atom on the critical path
