# Spec delta: change23-frontend-markdown-and-code-render

## ADDED Requirements

### Requirement: change23-frontend-markdown-and-code-render binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `frontend-markdown-and-code-render`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change23-frontend-markdown-and-code-render-rendercontent-parses-markdown-using-mark
- GIVEN the bridge script is invoked with `--feature-id frontend-markdown-and-code-render`
- WHEN the change scaffold is produced
- THEN the behavior "renderContent parses Markdown using marked.parse" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change23-frontend-markdown-and-code-render-code-blocks-are-highlighted-via-hljs-and
- GIVEN the bridge script is invoked with `--feature-id frontend-markdown-and-code-render`
- WHEN the change scaffold is produced
- THEN the behavior "Code blocks are highlighted via hljs and decorated with language badge and Copy button" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change23-frontend-markdown-and-code-render-copy-button-copies-code-to-clipboard-usi
- GIVEN the bridge script is invoked with `--feature-id frontend-markdown-and-code-render`
- WHEN the change scaffold is produced
- THEN the behavior "Copy button copies code to clipboard using navigator.clipboard.writeText with feedback state" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change23-frontend-markdown-and-code-render-msg-code-exe-and-msg-tool-styled-with-0d
- GIVEN the bridge script is invoked with `--feature-id frontend-markdown-and-code-render`
- WHEN the change scaffold is produced
- THEN the behavior "msg-code_exe and msg-tool styled with #0d1117 background and cyan $ EXECUTE branding" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change23-frontend-markdown-and-code-render-thought-content-renders-in-details-class
- GIVEN the bridge script is invoked with `--feature-id frontend-markdown-and-code-render`
- WHEN the change scaffold is produced
- THEN the behavior "Thought content renders in <details class='thought-step'>" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
