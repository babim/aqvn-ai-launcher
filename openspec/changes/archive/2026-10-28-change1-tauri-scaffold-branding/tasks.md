# Tasks: change1-tauri-scaffold-branding

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [ ] C1-001: cargo check/test/clippy -D warnings/fmt all exit 0, zero warnings
- [ ] C1-002: Release binary builds; size recorded and < 8MB
- [ ] C1-003: App launches under Xvfb :97 (WEBKIT_DISABLE_DMABUF_RENDERER=1) and renders the placeholder UI (ffmpeg screenshot evidence)
- [ ] C1-004: tauri.conf.json identity fields exactly net.aqvn.workspace / AQVN Workspace
- [ ] C1-005: rg whitelabel gate: zero matches outside docs/rfc-spec/

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C1-001 | cargo check/test/clippy -D warnings/fmt all exit 0, zero warnings | aqvn-opsx-doit | `surf:1-tauri-scaffold-branding` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-SCAF-001. Seeded RED: behavior unmet. GREEN: behavior met and VAL-SCAF-001 pass. | — | P2 | 1 | native-subagents | planned |
| C1-002 | Release binary builds; size recorded and < 8MB | aqvn-opsx-doit | `surf:1-tauri-scaffold-branding` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-SCAF-002. Seeded RED: behavior unmet. GREEN: behavior met and VAL-SCAF-002 pass. | `C1-001` | P2 | 1 | native-subagents | planned |
| C1-003 | App launches under Xvfb :97 (WEBKIT_DISABLE_DMABUF_RENDERER=1) and renders the placeholder UI (ffmpeg screenshot evidence) | aqvn-opsx-doit | `surf:1-tauri-scaffold-branding` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-SCAF-003. Seeded RED: behavior unmet. GREEN: behavior met and VAL-SCAF-003 pass. | `C1-002` | P2 | 1 | native-subagents | planned |
| C1-004 | tauri.conf.json identity fields exactly net.aqvn.workspace / AQVN Workspace | aqvn-opsx-doit | `surf:1-tauri-scaffold-branding` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-SCAF-004. Seeded RED: behavior unmet. GREEN: behavior met and VAL-SCAF-004 pass. | `C1-003` | P2 | 1 | native-subagents | planned |
| C1-005 | rg whitelabel gate: zero matches outside docs/rfc-spec/ | aqvn-opsx-doit | `surf:1-tauri-scaffold-branding` | `(proposals-only-scaffold)` | script | Mapped VAL-*: VAL-SCAF-005, VAL-SCAF-006, VAL-SCAF-013, VAL-SCAF-014. Seeded RED: behavior unmet. GREEN: behavior met and VAL-SCAF-005, VAL-SCAF-006, VAL-SCAF-013, VAL-SCAF-014 pass. | `C1-004` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:1-tauri-scaffold-branding` | `C1-001`, `C1-002`, `C1-003`, `C1-004`, `C1-005` | covered |

## Dependency graph

- `C1-001` -> `C1-002`
- `C1-002` -> `C1-003`
- `C1-003` -> `C1-004`
- `C1-004` -> `C1-005`

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

- Task ID: `C1-001`
- Reason: first atom on the critical path
