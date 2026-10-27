# Tasks: change19-multi-server-url-switcher

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [x] C19-001: Login screen displays editable Server URL field and saved profile options
- [x] C19-002: Changing Server URL persists to store and saves to server_profiles list
- [x] C19-003: Settings page displays and allows managing saved server profiles
- [x] C19-004: Chat header displays the active connected Server URL
- [x] C19-005: agent-browser tests verify profile selection, custom URL entry, and persistence without console errors
- [x] C19-006: cargo check, clippy -D warnings, fmt clean, whitelabel 0 matches

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C19-001 | Login screen displays editable Server URL field and saved profile options | aqvn-opsx-doit | `surf:19-multi-server-url-switcher` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CHAT-016. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CHAT-016 pass. | — | P2 | 1 | native-subagents | planned |
| C19-002 | Changing Server URL persists to store and saves to server_profiles list | aqvn-opsx-doit | `surf:19-multi-server-url-switcher` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CHAT-016. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CHAT-016 pass. | `C19-001` | P2 | 1 | native-subagents | planned |
| C19-003 | Settings page displays and allows managing saved server profiles | aqvn-opsx-doit | `surf:19-multi-server-url-switcher` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CHAT-016. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CHAT-016 pass. | `C19-002` | P2 | 1 | native-subagents | planned |
| C19-004 | Chat header displays the active connected Server URL | aqvn-opsx-doit | `surf:19-multi-server-url-switcher` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CHAT-016. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CHAT-016 pass. | `C19-003` | P2 | 1 | native-subagents | planned |
| C19-005 | agent-browser tests verify profile selection, custom URL entry, and persistence without console errors | aqvn-opsx-doit | `surf:19-multi-server-url-switcher` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CHAT-016. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CHAT-016 pass. | `C19-004` | P2 | 1 | native-subagents | planned |
| C19-006 | cargo check, clippy -D warnings, fmt clean, whitelabel 0 matches | aqvn-opsx-doit | `surf:19-multi-server-url-switcher` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CHAT-016. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CHAT-016 pass. | `C19-005` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:19-multi-server-url-switcher` | `C19-001`, `C19-002`, `C19-003`, `C19-004`, `C19-005`, `C19-006` | covered |

## Dependency graph

- `C19-001` -> `C19-002`
- `C19-002` -> `C19-003`
- `C19-003` -> `C19-004`
- `C19-004` -> `C19-005`
- `C19-005` -> `C19-006`

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

- Task ID: `C19-001`
- Reason: first atom on the critical path
