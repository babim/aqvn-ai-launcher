# Tasks: change29-frontend-outbox-durability-fix

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [x] C29-001: Two overlapping flushOutbox calls send exactly one message_async per queued entry
- [x] C29-002: A queued entry is claimed before sending so a reconnect cannot re-send it
- [x] C29-003: The outbox is capped by entry count and/or total bytes
- [x] C29-004: When localStorage.setItem throws, a user-visible error is surfaced and the row is not left claiming delivery
- [x] C29-005: node tools/tests/test_chat_handlers.mjs passes with the new assertions
- [x] C29-006: cargo test, clippy -D warnings and cargo fmt --check remain clean

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C29-001 | Two overlapping flushOutbox calls send exactly one message_async per queued entry | aqvn-opsx-doit | `surf:29-frontend-outbox-durability-fix` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CONN-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CONN-003 pass. | — | P2 | 1 | native-subagents | planned |
| C29-002 | A queued entry is claimed before sending so a reconnect cannot re-send it | aqvn-opsx-doit | `surf:29-frontend-outbox-durability-fix` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CONN-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CONN-003 pass. | `C29-001` | P2 | 1 | native-subagents | planned |
| C29-003 | The outbox is capped by entry count and/or total bytes | aqvn-opsx-doit | `surf:29-frontend-outbox-durability-fix` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CONN-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CONN-003 pass. | `C29-002` | P2 | 1 | native-subagents | planned |
| C29-004 | When localStorage.setItem throws, a user-visible error is surfaced and the row is not left claiming delivery | aqvn-opsx-doit | `surf:29-frontend-outbox-durability-fix` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CONN-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CONN-003 pass. | `C29-003` | P2 | 1 | native-subagents | planned |
| C29-005 | node tools/tests/test_chat_handlers.mjs passes with the new assertions | aqvn-opsx-doit | `surf:29-frontend-outbox-durability-fix` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CONN-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CONN-003 pass. | `C29-004` | P2 | 1 | native-subagents | planned |
| C29-006 | cargo test, clippy -D warnings and cargo fmt --check remain clean | aqvn-opsx-doit | `surf:29-frontend-outbox-durability-fix` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CONN-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CONN-003 pass. | `C29-005` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:29-frontend-outbox-durability-fix` | `C29-001`, `C29-002`, `C29-003`, `C29-004`, `C29-005`, `C29-006` | covered |

## Dependency graph

- `C29-001` -> `C29-002`
- `C29-002` -> `C29-003`
- `C29-003` -> `C29-004`
- `C29-004` -> `C29-005`
- `C29-005` -> `C29-006`

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

- Task ID: `C29-001`
- Reason: first atom on the critical path
