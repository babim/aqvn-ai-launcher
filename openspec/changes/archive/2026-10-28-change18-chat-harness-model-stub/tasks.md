# Tasks: change18-chat-harness-model-stub

<!-- generated from roadmap-audit.json by render-tasks-md; do not edit by hand -->

## Execution tasks

- [ ] C18-001: M18-T1: tools/model_stub_server.py serves OpenAI-compatible completions on loopback
- [ ] C18-002: M18-T2: Harness config points chat_model at stub (config files only, no source edits)
- [ ] C18-003: M18-T3: chat_e2e.sh flow yields an assistant record with non-empty content (exit 0)
- [ ] C18-004: M18-T4: Clean shutdown: stub + harness + app stopped, ports free, no orphans

## Task inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C18-001 | M18-T1: tools/model_stub_server.py serves OpenAI-compatible completions on loopback | aqvn-opsx-doit | `surf:18-chat-harness-model-stub` | `(proposals-only-scaffold)` | script | No VAL-* mapped (fulfills[] empty). Behavior: "tools/model_stub_server.py serves OpenAI-compatible completions on loopback". Seeded RED: behavior unmet. GREEN: behavior met. | — | P2 | 1 | native-subagents | planned |
| C18-002 | M18-T2: Harness config points chat_model at stub (config files only, no source edits) | aqvn-opsx-doit | `surf:18-chat-harness-model-stub` | `(proposals-only-scaffold)` | script | No VAL-* mapped (fulfills[] empty). Behavior: "Harness config points chat_model at stub (config files only, no source edits)". Seeded RED: behavior unmet. GREEN: behavior met. | `C18-001` | P2 | 1 | native-subagents | planned |
| C18-003 | M18-T3: chat_e2e.sh flow yields an assistant record with non-empty content (exit 0) | aqvn-opsx-doit | `surf:18-chat-harness-model-stub` | `(proposals-only-scaffold)` | script | No VAL-* mapped (fulfills[] empty). Behavior: "chat_e2e.sh flow yields an assistant record with non-empty content (exit 0)". Seeded RED: behavior unmet. GREEN: behavior met. | `C18-002` | P2 | 1 | native-subagents | planned |
| C18-004 | M18-T4: Clean shutdown: stub + harness + app stopped, ports free, no orphans | aqvn-opsx-doit | `surf:18-chat-harness-model-stub` | `(proposals-only-scaffold)` | script | No VAL-* mapped (fulfills[] empty). Behavior: "Clean shutdown: stub + harness + app stopped, ports free, no orphans". Seeded RED: behavior unmet. GREEN: behavior met. | `C18-003` | P2 | 1 | native-subagents | planned |

## Surface to task coverage

| Surface ID | Task IDs | Coverage status |
| --- | --- | --- |
| `surf:18-chat-harness-model-stub` | `C18-001`, `C18-002`, `C18-003`, `C18-004` | covered |

## Dependency graph

- `C18-001` -> `C18-002`
- `C18-002` -> `C18-003`
- `C18-003` -> `C18-004`

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

- Task ID: `C18-001`
- Reason: first atom on the critical path
