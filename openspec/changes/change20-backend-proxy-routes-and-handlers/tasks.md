# Tasks: change20-backend-proxy-routes-and-handlers

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [x] C20-001: Requests to /plugins/*, /extensions/*, and /public/* are routed to handle_forward_raw
- [x] C20-002: handle_forward_raw forwards requests without requiring CSRF header
- [x] C20-003: Upstream session cookies are attached and response Set-Cookie headers update the CookieJar
- [x] C20-004: Response Content-Type and raw binary body bytes are preserved
- [x] C20-005: Integration tests in src-tauri/tests/proxy_server.rs pass cleanly

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C20-001 | Requests to /plugins/*, /extensions/*, and /public/* are routed to handle_forward_raw | aqvn-opsx-doit | `surf:20-backend-proxy-routes-and-handlers` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-PROXY-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-PROXY-001 pass. | — | P2 | 1 | native-subagents | planned |
| C20-002 | handle_forward_raw forwards requests without requiring CSRF header | aqvn-opsx-doit | `surf:20-backend-proxy-routes-and-handlers` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-PROXY-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-PROXY-002 pass. | `C20-001` | P2 | 1 | native-subagents | planned |
| C20-003 | Upstream session cookies are attached and response Set-Cookie headers update the CookieJar | aqvn-opsx-doit | `surf:20-backend-proxy-routes-and-handlers` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-PROXY-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-PROXY-003 pass. | `C20-002` | P2 | 1 | native-subagents | planned |
| C20-004 | Response Content-Type and raw binary body bytes are preserved | aqvn-opsx-doit | `surf:20-backend-proxy-routes-and-handlers` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-PROXY-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-PROXY-001 pass. | `C20-003` | P2 | 1 | native-subagents | planned |
| C20-005 | Integration tests in src-tauri/tests/proxy_server.rs pass cleanly | aqvn-opsx-doit | `surf:20-backend-proxy-routes-and-handlers` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-PROXY-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-PROXY-002 pass. | `C20-004` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:20-backend-proxy-routes-and-handlers` | `C20-001`, `C20-002`, `C20-003`, `C20-004`, `C20-005` | covered |

## Dependency graph

- `C20-001` -> `C20-002`
- `C20-002` -> `C20-003`
- `C20-003` -> `C20-004`
- `C20-004` -> `C20-005`

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

- Task ID: `C20-001`
- Reason: first atom on the critical path
