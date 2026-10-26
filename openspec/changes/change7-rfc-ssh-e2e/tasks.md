# Tasks: change7-rfc-ssh-e2e

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [ ] C7-001: spec_client Gate 3: correct hash → file content; wrong hash → 403; outside sandbox → 403
- [ ] C7-002: Real A0 call_rfc (harness venv) succeeds against our server — wire-compatible proof
- [ ] C7-003: SSH exchange decrypts to exact ssh_password; paramiko session runs command end-to-end
- [ ] C7-004: Harness left stopped, no orphans, ports free

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C7-001 | spec_client Gate 3: correct hash → file content; wrong hash → 403; outside sandbox → 403 | aqvn-opsx-doit | `surf:7-rfc-ssh-e2e` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-RFC-027. Seeded RED: behavior unmet. GREEN: behavior met and VAL-RFC-027 pass. | — | P2 | 1 | native-subagents | planned |
| C7-002 | Real A0 call_rfc (harness venv) succeeds against our server — wire-compatible proof | aqvn-opsx-doit | `surf:7-rfc-ssh-e2e` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-RFC-028. Seeded RED: behavior unmet. GREEN: behavior met and VAL-RFC-028 pass. | `C7-001` | P2 | 1 | native-subagents | planned |
| C7-003 | SSH exchange decrypts to exact ssh_password; paramiko session runs command end-to-end | aqvn-opsx-doit | `surf:7-rfc-ssh-e2e` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-SSH-007. Seeded RED: behavior unmet. GREEN: behavior met and VAL-SSH-007 pass. | `C7-002` | P2 | 1 | native-subagents | planned |
| C7-004 | Harness left stopped, no orphans, ports free | aqvn-opsx-doit | `surf:7-rfc-ssh-e2e` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-SSH-008. Seeded RED: behavior unmet. GREEN: behavior met and VAL-SSH-008 pass. | `C7-003` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:7-rfc-ssh-e2e` | `C7-001`, `C7-002`, `C7-003`, `C7-004` | covered |

## Dependency graph

- `C7-001` -> `C7-002`
- `C7-002` -> `C7-003`
- `C7-003` -> `C7-004`

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

- Task ID: `C7-001`
- Reason: first atom on the critical path
