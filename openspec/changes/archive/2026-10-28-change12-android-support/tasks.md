# Tasks: change12-android-support

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [ ] C12-001: gen/android committed and clean; APK builds exit 0 with arm64 libs
- [ ] C12-002: Manifest: net.aqvn.workspace + AQVN label + INTERNET-only
- [ ] C12-003: RFC/SSH/tray/hotkey absent from Android build (source gating + .so symbol check)
- [ ] C12-004: Deferral of runtime test explicitly logged

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C12-001 | gen/android committed and clean; APK builds exit 0 with arm64 libs | aqvn-opsx-doit | `surf:12-android-support` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-MOB-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-MOB-001 pass. | — | P2 | 1 | native-subagents | planned |
| C12-002 | Manifest: net.aqvn.workspace + AQVN label + INTERNET-only | aqvn-opsx-doit | `surf:12-android-support` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-MOB-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-MOB-002 pass. | `C12-001` | P2 | 1 | native-subagents | planned |
| C12-003 | RFC/SSH/tray/hotkey absent from Android build (source gating + .so symbol check) | aqvn-opsx-doit | `surf:12-android-support` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-MOB-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-MOB-003 pass. | `C12-002` | P2 | 1 | native-subagents | planned |
| C12-004 | Deferral of runtime test explicitly logged | aqvn-opsx-doit | `surf:12-android-support` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-MOB-004. Seeded RED: behavior unmet. GREEN: behavior met and VAL-MOB-004 pass. | `C12-003` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:12-android-support` | `C12-001`, `C12-002`, `C12-003`, `C12-004` | covered |

## Dependency graph

- `C12-001` -> `C12-002`
- `C12-002` -> `C12-003`
- `C12-003` -> `C12-004`

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

- Task ID: `C12-001`
- Reason: first atom on the critical path
