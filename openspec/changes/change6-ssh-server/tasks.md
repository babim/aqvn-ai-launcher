# Tasks: change6-ssh-server

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [ ] C6-001: Listener only when toggle ON; port configurable + hot-applied
- [ ] C6-002: Correct password → PTY shell (requested geometry, cwd = first allowed path); wrong password → rejected
- [ ] C6-003: Denylist blocks named destructive population; benign passes; session survives refusal
- [ ] C6-004: Security invariant: denylist is best-effort secondary — never weakens auth/toggle

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C6-001 | Listener only when toggle ON; port configurable + hot-applied | aqvn-opsx-doit | `surf:6-ssh-server` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-SSH-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-SSH-001 pass. | — | P2 | 1 | native-subagents | planned |
| C6-002 | Correct password → PTY shell (requested geometry, cwd = first allowed path); wrong password → rejected | aqvn-opsx-doit | `surf:6-ssh-server` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-SSH-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-SSH-002 pass. | `C6-001` | P2 | 1 | native-subagents | planned |
| C6-003 | Denylist blocks named destructive population; benign passes; session survives refusal | aqvn-opsx-doit | `surf:6-ssh-server` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-SSH-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-SSH-003 pass. | `C6-002` | P2 | 1 | native-subagents | planned |
| C6-004 | Security invariant: denylist is best-effort secondary — never weakens auth/toggle | aqvn-opsx-doit | `surf:6-ssh-server` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-SSH-004, VAL-SSH-005, VAL-SSH-006, VAL-SSH-009. Seeded RED: behavior unmet. GREEN: behavior met and VAL-SSH-004, VAL-SSH-005, VAL-SSH-006, VAL-SSH-009 pass. | `C6-003` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:6-ssh-server` | `C6-001`, `C6-002`, `C6-003`, `C6-004` | covered |

## Dependency graph

- `C6-001` -> `C6-002`
- `C6-002` -> `C6-003`
- `C6-003` -> `C6-004`

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

- Task ID: `C6-001`
- Reason: first atom on the critical path
