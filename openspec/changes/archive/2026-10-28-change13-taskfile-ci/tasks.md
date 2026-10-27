# Tasks: change13-taskfile-ci

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [ ] C13-001: task --list shows build:linux/build:windows/build:android; task build:linux exits 0 with deb+rpm
- [ ] C13-002: release.yml: tag trigger, 3 jobs, windows size gate 10MB step present
- [ ] C13-003: actionlint + YAML parse clean
- [ ] C13-004: CI-runtime deferral logged

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C13-001 | task --list shows build:linux/build:windows/build:android; task build:linux exits 0 with deb+rpm | aqvn-opsx-doit | `surf:13-taskfile-ci` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-MOB-005. Seeded RED: behavior unmet. GREEN: behavior met and VAL-MOB-005 pass. | — | P2 | 1 | native-subagents | planned |
| C13-002 | release.yml: tag trigger, 3 jobs, windows size gate 10MB step present | aqvn-opsx-doit | `surf:13-taskfile-ci` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CI-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CI-001 pass. | `C13-001` | P2 | 1 | native-subagents | planned |
| C13-003 | actionlint + YAML parse clean | aqvn-opsx-doit | `surf:13-taskfile-ci` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CI-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CI-002 pass. | `C13-002` | P2 | 1 | native-subagents | planned |
| C13-004 | CI-runtime deferral logged | aqvn-opsx-doit | `surf:13-taskfile-ci` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-CI-003, VAL-CI-004. Seeded RED: behavior unmet. GREEN: behavior met and VAL-CI-003, VAL-CI-004 pass. | `C13-003` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:13-taskfile-ci` | `C13-001`, `C13-002`, `C13-003`, `C13-004` | covered |

## Dependency graph

- `C13-001` -> `C13-002`
- `C13-002` -> `C13-003`
- `C13-003` -> `C13-004`

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

- Task ID: `C13-001`
- Reason: first atom on the critical path
