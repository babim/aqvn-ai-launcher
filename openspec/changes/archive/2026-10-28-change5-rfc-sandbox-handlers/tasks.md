# Tasks: change5-rfc-sandbox-handlers

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [ ] C5-001: Every whitelist fn returns exact spec shape (integration tests per family)
- [ ] C5-002: Sandbox: escape read/write/delete, ../ traversal, symlink escape → 403; /a0/ prefix maps into sandbox root
- [ ] C5-003: _provide_root_password round-trips: python harness crypto decrypts to exact ssh_password
- [ ] C5-004: make_dir jailed like all path ops
- [ ] C5-005: Settings hot-reload works (password/paths/port/toggle), process PID unchanged

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C5-001 | Every whitelist fn returns exact spec shape (integration tests per family) | aqvn-opsx-doit | `surf:5-rfc-sandbox-handlers` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-RFC-009. Seeded RED: behavior unmet. GREEN: behavior met and VAL-RFC-009 pass. | — | P2 | 1 | native-subagents | planned |
| C5-002 | Sandbox: escape read/write/delete, ../ traversal, symlink escape → 403; /a0/ prefix maps into sandbox root | aqvn-opsx-doit | `surf:5-rfc-sandbox-handlers` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-RFC-010. Seeded RED: behavior unmet. GREEN: behavior met and VAL-RFC-010 pass. | `C5-001` | P2 | 1 | native-subagents | planned |
| C5-003 | _provide_root_password round-trips: python harness crypto decrypts to exact ssh_password | aqvn-opsx-doit | `surf:5-rfc-sandbox-handlers` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-RFC-011. Seeded RED: behavior unmet. GREEN: behavior met and VAL-RFC-011 pass. | `C5-002` | P2 | 1 | native-subagents | planned |
| C5-004 | make_dir jailed like all path ops | aqvn-opsx-doit | `surf:5-rfc-sandbox-handlers` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-RFC-012. Seeded RED: behavior unmet. GREEN: behavior met and VAL-RFC-012 pass. | `C5-003` | P2 | 1 | native-subagents | planned |
| C5-005 | Settings hot-reload works (password/paths/port/toggle), process PID unchanged | aqvn-opsx-doit | `surf:5-rfc-sandbox-handlers` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-RFC-013, VAL-RFC-014, VAL-RFC-015, VAL-RFC-016, VAL-RFC-017, VAL-RFC-018, VAL-RFC-019, VAL-RFC-020, VAL-RFC-021, VAL-RFC-022, VAL-RFC-023, VAL-RFC-024, VAL-RFC-026. Seeded RED: behavior unmet. GREEN: behavior met and VAL-RFC-013, VAL-RFC-014, VAL-RFC-015, VAL-RFC-016, VAL-RFC-017, VAL-RFC-018, VAL-RFC-019, VAL-RFC-020, VAL-RFC-021, VAL-RFC-022, VAL-RFC-023, VAL-RFC-024, VAL-RFC-026 pass. | `C5-004` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:5-rfc-sandbox-handlers` | `C5-001`, `C5-002`, `C5-003`, `C5-004`, `C5-005` | covered |

## Dependency graph

- `C5-001` -> `C5-002`
- `C5-002` -> `C5-003`
- `C5-003` -> `C5-004`
- `C5-004` -> `C5-005`

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

- Task ID: `C5-001`
- Reason: first atom on the critical path
