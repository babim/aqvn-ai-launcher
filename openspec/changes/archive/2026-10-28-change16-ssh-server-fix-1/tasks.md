# Tasks: change16-ssh-server-fix-1

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [ ] C16-001: M16-T1: SSH binds 0.0.0.0:55022 when enabled AND ssh_password non-empty
- [ ] C16-002: M16-T2: Empty ssh_password → no bind, fail-closed with log
- [ ] C16-003: M16-T3: Denylist limitations documented in repo docs + missionDir library
- [ ] C16-004: M16-T4: Full cargo test + clippy + fmt green; whitelabel gate clean

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C16-001 | M16-T1: SSH binds 0.0.0.0:55022 when enabled AND ssh_password non-empty | aqvn-opsx-doit | `surf:16-ssh-server-fix-1` | `(proposals-only-scaffold)` | script | No VAL-* mapped (fulfills[] empty). Behavior: "SSH binds 0.0.0.0:55022 when enabled AND ssh_password non-empty". Seeded RED: behavior unmet. GREEN: behavior met. | — | P2 | 1 | native-subagents | planned |
| C16-002 | M16-T2: Empty ssh_password → no bind, fail-closed with log | aqvn-opsx-doit | `surf:16-ssh-server-fix-1` | `(proposals-only-scaffold)` | script | No VAL-* mapped (fulfills[] empty). Behavior: "Empty ssh_password → no bind, fail-closed with log". Seeded RED: behavior unmet. GREEN: behavior met. | `C16-001` | P2 | 1 | native-subagents | planned |
| C16-003 | M16-T3: Denylist limitations documented in repo docs + missionDir library | aqvn-opsx-doit | `surf:16-ssh-server-fix-1` | `(proposals-only-scaffold)` | script | No VAL-* mapped (fulfills[] empty). Behavior: "Denylist limitations documented in repo docs + missionDir library". Seeded RED: behavior unmet. GREEN: behavior met. | `C16-002` | P2 | 1 | native-subagents | planned |
| C16-004 | M16-T4: Full cargo test + clippy + fmt green; whitelabel gate clean | aqvn-opsx-doit | `surf:16-ssh-server-fix-1` | `(proposals-only-scaffold)` | script | No VAL-* mapped (fulfills[] empty). Behavior: "Full cargo test + clippy + fmt green; whitelabel gate clean". Seeded RED: behavior unmet. GREEN: behavior met. | `C16-003` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:16-ssh-server-fix-1` | `C16-001`, `C16-002`, `C16-003`, `C16-004` | covered |

## Dependency graph

- `C16-001` -> `C16-002`
- `C16-002` -> `C16-003`
- `C16-003` -> `C16-004`

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

- Task ID: `C16-001`
- Reason: first atom on the critical path
