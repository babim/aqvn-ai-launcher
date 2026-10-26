# Tasks: change4-rfc-server-core

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [ ] C4-001: Worked-example request (spec §7) returns 200 + bare JSON
- [ ] C4-002: Wrong hash / missing hash → 403; malformed rfc_input → 500; GET → 405; unknown route → 404 (exact bodies)
- [ ] C4-003: Unknown module/function → 403 (whitelist enforced)
- [ ] C4-004: Toggle OFF → port 55080 closed (ss -tln evidence)
- [ ] C4-005: HMAC constant-time (code review criterion)

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C4-001 | Worked-example request (spec §7) returns 200 + bare JSON | aqvn-opsx-doit | `surf:4-rfc-server-core` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-RFC-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-RFC-001 pass. | — | P2 | 1 | native-subagents | planned |
| C4-002 | Wrong hash / missing hash → 403; malformed rfc_input → 500; GET → 405; unknown route → 404 (exact bodies) | aqvn-opsx-doit | `surf:4-rfc-server-core` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-RFC-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-RFC-002 pass. | `C4-001` | P2 | 1 | native-subagents | planned |
| C4-003 | Unknown module/function → 403 (whitelist enforced) | aqvn-opsx-doit | `surf:4-rfc-server-core` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-RFC-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-RFC-003 pass. | `C4-002` | P2 | 1 | native-subagents | planned |
| C4-004 | Toggle OFF → port 55080 closed (ss -tln evidence) | aqvn-opsx-doit | `surf:4-rfc-server-core` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-RFC-004. Seeded RED: behavior unmet. GREEN: behavior met and VAL-RFC-004 pass. | `C4-003` | P2 | 1 | native-subagents | planned |
| C4-005 | HMAC constant-time (code review criterion) | aqvn-opsx-doit | `surf:4-rfc-server-core` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-RFC-005, VAL-RFC-006, VAL-RFC-007, VAL-RFC-008, VAL-RFC-025. Seeded RED: behavior unmet. GREEN: behavior met and VAL-RFC-005, VAL-RFC-006, VAL-RFC-007, VAL-RFC-008, VAL-RFC-025 pass. | `C4-004` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:4-rfc-server-core` | `C4-001`, `C4-002`, `C4-003`, `C4-004`, `C4-005` | covered |

## Dependency graph

- `C4-001` -> `C4-002`
- `C4-002` -> `C4-003`
- `C4-003` -> `C4-004`
- `C4-004` -> `C4-005`

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

- Task ID: `C4-001`
- Reason: first atom on the critical path
