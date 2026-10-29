# Tasks: change36-feat-web-proxy-passthrough

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [x] C36-001: Requests to /, /login.css, /vendor/*, /css/*, /js/* are forwarded to upstream
- [x] C36-002: Content-Type, status codes, and headers are preserved
- [x] C36-003: Session cookies from CookieJar are replayed and upstream Set-Cookie headers are absorbed
- [x] C36-004: Integration tests in src-tauri/tests/proxy_server.rs pass cleanly

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C36-001 | Requests to /, /login.css, /vendor/*, /css/*, /js/* are forwarded to upstream | aqvn-opsx-doit | `surf:36-feat-web-proxy-passthrough` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-WEB-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-WEB-001 pass. | — | P2 | 1 | native-subagents | planned |
| C36-002 | Content-Type, status codes, and headers are preserved | aqvn-opsx-doit | `surf:36-feat-web-proxy-passthrough` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-WEB-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-WEB-001 pass. | `C36-001` | P2 | 1 | native-subagents | planned |
| C36-003 | Session cookies from CookieJar are replayed and upstream Set-Cookie headers are absorbed | aqvn-opsx-doit | `surf:36-feat-web-proxy-passthrough` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-WEB-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-WEB-001 pass. | `C36-002` | P2 | 1 | native-subagents | planned |
| C36-004 | Integration tests in src-tauri/tests/proxy_server.rs pass cleanly | aqvn-opsx-doit | `surf:36-feat-web-proxy-passthrough` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-WEB-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-WEB-001 pass. | `C36-003` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:36-feat-web-proxy-passthrough` | `C36-001`, `C36-002`, `C36-003`, `C36-004` | covered |

## Dependency graph

- `C36-001` -> `C36-002`
- `C36-002` -> `C36-003`
- `C36-003` -> `C36-004`

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

- Task ID: `C36-001`
- Reason: first atom on the critical path
