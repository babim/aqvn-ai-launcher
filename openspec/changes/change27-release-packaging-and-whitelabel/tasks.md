# Tasks: change27-release-packaging-and-whitelabel

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [x] C27-001: Cargo.toml and tauri.conf.json specify version = '0.1.6'
- [x] C27-002: CHANGELOG.md documents [0.1.6] release features
- [x] C27-003: Zero occurrences of 'Agent Zero' or 'agent-zero' across the repository
- [x] C27-004: cd src-tauri && cargo test passes 100% (125+ tests)
- [x] C27-005: cargo clippy --all-targets -- -D warnings and cargo fmt --check pass cleanly

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C27-001 | Cargo.toml and tauri.conf.json specify version = '0.1.6' | aqvn-opsx-doit | `surf:27-release-packaging-and-whitelabel` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-REL-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-REL-001 pass. | — | P2 | 1 | native-subagents | planned |
| C27-002 | CHANGELOG.md documents [0.1.6] release features | aqvn-opsx-doit | `surf:27-release-packaging-and-whitelabel` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-REL-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-REL-002 pass. | `C27-001` | P2 | 1 | native-subagents | planned |
| C27-003 | Zero occurrences of 'Agent Zero' or 'agent-zero' across the repository | aqvn-opsx-doit | `surf:27-release-packaging-and-whitelabel` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-REL-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-REL-003 pass. | `C27-002` | P2 | 1 | native-subagents | planned |
| C27-004 | cd src-tauri && cargo test passes 100% (125+ tests) | aqvn-opsx-doit | `surf:27-release-packaging-and-whitelabel` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-REL-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-REL-001 pass. | `C27-003` | P2 | 1 | native-subagents | planned |
| C27-005 | cargo clippy --all-targets -- -D warnings and cargo fmt --check pass cleanly | aqvn-opsx-doit | `surf:27-release-packaging-and-whitelabel` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-REL-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-REL-002 pass. | `C27-004` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:27-release-packaging-and-whitelabel` | `C27-001`, `C27-002`, `C27-003`, `C27-004`, `C27-005` | covered |

## Dependency graph

- `C27-001` -> `C27-002`
- `C27-002` -> `C27-003`
- `C27-003` -> `C27-004`
- `C27-004` -> `C27-005`

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

- Task ID: `C27-001`
- Reason: first atom on the critical path
