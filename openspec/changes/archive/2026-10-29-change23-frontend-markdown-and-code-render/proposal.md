# Proposal: change23-frontend-markdown-and-code-render

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `frontend-markdown-and-code-render` (milestone `frontend-chat-experience`) to an openspec change. The feature description:

> Implement safe Markdown parsing in ui/chat.js replacing plain-text renderContent, with highlight.js syntax highlighting, 1-click Copy code buttons, terminal theme #0d1117 for code/tool messages, and collapsible <details class='thought-step'>.

Mission: `d398e923-10d9-46b0-af7c-32a71e95a347`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **23-001**: renderContent parses Markdown using marked.parse
- **23-002**: Code blocks are highlighted via hljs and decorated with language badge and Copy button
- **23-003**: Copy button copies code to clipboard using navigator.clipboard.writeText with feedback state
- **23-004**: msg-code_exe and msg-tool styled with #0d1117 background and cyan $ EXECUTE branding
- **23-005**: Thought content renders in <details class='thought-step'>

## Goals

- Bind feature `frontend-markdown-and-code-render` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change23-frontend-markdown-and-code-render/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-MD-002`
- `VAL-MD-003`
- `VAL-MD-004`
