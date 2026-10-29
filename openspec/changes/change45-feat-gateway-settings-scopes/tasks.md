# Tasks: change45-feat-gateway-settings-scopes

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [x] C45-001: Settings UI exposes checkboxes for each scope and master connector switch
- [x] C45-002: Toggling master off immediately terminates the outbound connection
- [x] C45-003: connector_hello scopes match active configuration

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C45-001 | Settings UI exposes checkboxes for each scope and master connector switch | aqvn-opsx-doit | `surf:45-feat-gateway-settings-scopes` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-GATEWAY-006. Seeded RED: behavior unmet. GREEN: behavior met and VAL-GATEWAY-006 pass. | — | P2 | 1 | native-subagents | complete |
| C45-002 | Toggling master off immediately terminates the outbound connection | aqvn-opsx-doit | `surf:45-feat-gateway-settings-scopes` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-GATEWAY-006. Seeded RED: behavior unmet. GREEN: behavior met and VAL-GATEWAY-006 pass. | `C45-001` | P2 | 1 | native-subagents | complete |
| C45-003 | connector_hello scopes match active configuration | aqvn-opsx-doit | `surf:45-feat-gateway-settings-scopes` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-GATEWAY-006. Seeded RED: behavior unmet. GREEN: behavior met and VAL-GATEWAY-006 pass. | `C45-002` | P2 | 1 | native-subagents | complete |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:45-feat-gateway-settings-scopes` | `C45-001`, `C45-002`, `C45-003` | covered |

## Dependency graph

- `C45-001` -> `C45-002`
- `C45-002` -> `C45-003`

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

- Task ID: `C45-001`
- Reason: first atom on the critical path
