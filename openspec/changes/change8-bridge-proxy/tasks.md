# Tasks: change8-bridge-proxy

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [ ] C8-001: ss shows 127.0.0.1 bind only; port random per launch; UI receives it at init
- [ ] C8-002: CSRF bootstrap + login-on-behalf work against harness :5000 (admin/test-auth-2026)
- [ ] C8-003: WS handshake through proxy passes validate_ws_origin (Origin spoofed) — state_request ACK received
- [ ] C8-004: Session survives ≥50s (ping interval); 403 → single refresh+retry; runtime rotation → auto re-bootstrap

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C8-001 | ss shows 127.0.0.1 bind only; port random per launch; UI receives it at init | aqvn-opsx-doit | `surf:8-bridge-proxy` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-PROXY-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-PROXY-001 pass. | — | P2 | 1 | native-subagents | planned |
| C8-002 | CSRF bootstrap + login-on-behalf work against harness :5000 (admin/test-auth-2026) | aqvn-opsx-doit | `surf:8-bridge-proxy` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-PROXY-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-PROXY-002 pass. | `C8-001` | P2 | 1 | native-subagents | planned |
| C8-003 | WS handshake through proxy passes validate_ws_origin (Origin spoofed) — state_request ACK received | aqvn-opsx-doit | `surf:8-bridge-proxy` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-PROXY-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-PROXY-003 pass. | `C8-002` | P2 | 1 | native-subagents | planned |
| C8-004 | Session survives ≥50s (ping interval); 403 → single refresh+retry; runtime rotation → auto re-bootstrap | aqvn-opsx-doit | `surf:8-bridge-proxy` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-PROXY-004, VAL-PROXY-005, VAL-PROXY-006, VAL-PROXY-007, VAL-PROXY-008. Seeded RED: behavior unmet. GREEN: behavior met and VAL-PROXY-004, VAL-PROXY-005, VAL-PROXY-006, VAL-PROXY-007, VAL-PROXY-008 pass. | `C8-003` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:8-bridge-proxy` | `C8-001`, `C8-002`, `C8-003`, `C8-004` | covered |

## Dependency graph

- `C8-001` -> `C8-002`
- `C8-002` -> `C8-003`
- `C8-003` -> `C8-004`

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

- Task ID: `C8-001`
- Reason: first atom on the critical path
