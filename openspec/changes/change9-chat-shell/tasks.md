# Tasks: change9-chat-shell

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [ ] C9-001: agent-browser: login → send → streaming render (merge by id) → stop → new chat, zero console errors
- [ ] C9-002: Harness E2E: full chat round-trip through proxy against :5000
- [ ] C9-003: Real-server E2E: login + state_request + message_async + response records vs a0.matmagoc.com (creds redacted)
- [ ] C9-004: Reconnect re-handshakes; session expiry surfaces re-login UI; browser-mode and real app consistent

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C9-001 | agent-browser: login → send → streaming render (merge by id) → stop → new chat, zero console errors | aqvn-opsx-doit | `surf:9-chat-shell` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CHAT-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CHAT-001 pass. | — | P2 | 1 | native-subagents | planned |
| C9-002 | Harness E2E: full chat round-trip through proxy against :5000 | aqvn-opsx-doit | `surf:9-chat-shell` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CHAT-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CHAT-002 pass. | `C9-001` | P2 | 1 | native-subagents | planned |
| C9-003 | Real-server E2E: login + state_request + message_async + response records vs a0.matmagoc.com (creds redacted) | aqvn-opsx-doit | `surf:9-chat-shell` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CHAT-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CHAT-003 pass. | `C9-002` | P2 | 1 | native-subagents | planned |
| C9-004 | Reconnect re-handshakes; session expiry surfaces re-login UI; browser-mode and real app consistent | aqvn-opsx-doit | `surf:9-chat-shell` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CHAT-004, VAL-CHAT-005, VAL-CHAT-006, VAL-CHAT-007, VAL-CHAT-008, VAL-CHAT-009, VAL-CHAT-010, VAL-CHAT-011, VAL-CHAT-012, VAL-CHAT-013, VAL-CHAT-014, VAL-CHAT-015. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CHAT-004, VAL-CHAT-005, VAL-CHAT-006, VAL-CHAT-007, VAL-CHAT-008, VAL-CHAT-009, VAL-CHAT-010, VAL-CHAT-011, VAL-CHAT-012, VAL-CHAT-013, VAL-CHAT-014, VAL-CHAT-015 pass. | `C9-003` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:9-chat-shell` | `C9-001`, `C9-002`, `C9-003`, `C9-004` | covered |

## Dependency graph

- `C9-001` -> `C9-002`
- `C9-002` -> `C9-003`
- `C9-003` -> `C9-004`

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

- Task ID: `C9-001`
- Reason: first atom on the critical path
