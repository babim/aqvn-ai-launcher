# Tasks: change15-rfc-server-core-fix-1

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [ ] C15-001: M15-T1: RFC binds 0.0.0.0:55080 when enabled AND rfc_password non-empty
- [ ] C15-002: M15-T2: Empty rfc_password → no bind, fail-closed with log
- [ ] C15-003: M15-T3: Empty allowed_paths → sandbox deny-all; multi-entry allowed_paths all honored
- [ ] C15-004: M15-T4: Full cargo test + clippy + fmt green; whitelabel gate clean

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C15-001 | M15-T1: RFC binds 0.0.0.0:55080 when enabled AND rfc_password non-empty | aqvn-opsx-doit | `surf:15-rfc-server-core-fix-1` | `(proposals-only-scaffold)` | script | No VAL-* mapped (fulfills[] empty). Behavior: "RFC binds 0.0.0.0:55080 when enabled AND rfc_password non-empty". Seeded RED: behavior unmet. GREEN: behavior met. | — | P2 | 1 | native-subagents | planned |
| C15-002 | M15-T2: Empty rfc_password → no bind, fail-closed with log | aqvn-opsx-doit | `surf:15-rfc-server-core-fix-1` | `(proposals-only-scaffold)` | script | No VAL-* mapped (fulfills[] empty). Behavior: "Empty rfc_password → no bind, fail-closed with log". Seeded RED: behavior unmet. GREEN: behavior met. | `C15-001` | P2 | 1 | native-subagents | planned |
| C15-003 | M15-T3: Empty allowed_paths → sandbox deny-all; multi-entry allowed_paths all honored | aqvn-opsx-doit | `surf:15-rfc-server-core-fix-1` | `(proposals-only-scaffold)` | script | No VAL-* mapped (fulfills[] empty). Behavior: "Empty allowed_paths → sandbox deny-all; multi-entry allowed_paths all honored". Seeded RED: behavior unmet. GREEN: behavior met. | `C15-002` | P2 | 1 | native-subagents | planned |
| C15-004 | M15-T4: Full cargo test + clippy + fmt green; whitelabel gate clean | aqvn-opsx-doit | `surf:15-rfc-server-core-fix-1` | `(proposals-only-scaffold)` | script | No VAL-* mapped (fulfills[] empty). Behavior: "Full cargo test + clippy + fmt green; whitelabel gate clean". Seeded RED: behavior unmet. GREEN: behavior met. | `C15-003` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:15-rfc-server-core-fix-1` | `C15-001`, `C15-002`, `C15-003`, `C15-004` | covered |

## Dependency graph

- `C15-001` -> `C15-002`
- `C15-002` -> `C15-003`
- `C15-003` -> `C15-004`

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

- Task ID: `C15-001`
- Reason: first atom on the critical path
