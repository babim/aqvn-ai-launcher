# Tasks: change17-rfc-ssh-e2e-fix-1

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [ ] C17-001: M17-T1: a0_e2e.sh runs full harness E2E, exits 0, leaves zero orphans
- [ ] C17-002: M17-T2: Kill paths provenance-checked; pidfile retained on kill failure
- [ ] C17-003: M17-T3: Port 5000 waits to close like 55080/55022

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C17-001 | M17-T1: a0_e2e.sh runs full harness E2E, exits 0, leaves zero orphans | aqvn-opsx-doit | `surf:17-rfc-ssh-e2e-fix-1` | `(proposals-only-scaffold)` | script | No VAL-* mapped (fulfills[] empty). Behavior: "a0_e2e.sh runs full harness E2E, exits 0, leaves zero orphans". Seeded RED: behavior unmet. GREEN: behavior met. | — | P2 | 1 | native-subagents | planned |
| C17-002 | M17-T2: Kill paths provenance-checked; pidfile retained on kill failure | aqvn-opsx-doit | `surf:17-rfc-ssh-e2e-fix-1` | `(proposals-only-scaffold)` | script | No VAL-* mapped (fulfills[] empty). Behavior: "Kill paths provenance-checked; pidfile retained on kill failure". Seeded RED: behavior unmet. GREEN: behavior met. | `C17-001` | P2 | 1 | native-subagents | planned |
| C17-003 | M17-T3: Port 5000 waits to close like 55080/55022 | aqvn-opsx-doit | `surf:17-rfc-ssh-e2e-fix-1` | `(proposals-only-scaffold)` | script | No VAL-* mapped (fulfills[] empty). Behavior: "Port 5000 waits to close like 55080/55022". Seeded RED: behavior unmet. GREEN: behavior met. | `C17-002` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:17-rfc-ssh-e2e-fix-1` | `C17-001`, `C17-002`, `C17-003` | covered |

## Dependency graph

- `C17-001` -> `C17-002`
- `C17-002` -> `C17-003`

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

- Task ID: `C17-001`
- Reason: first atom on the critical path
