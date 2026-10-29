# Proposal: change28-frontend-markdown-line-breaks-fix

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `frontend-markdown-line-breaks-fix` (milestone `frontend-chat-experience`) to an openspec change. The feature description:

> Scrutiny fix (frontend-markdown-and-code-render): multi-line messages collapse to a single run-on line because marked.parse is called without breaks:true while .msg-body no longer sets white-space:pre-wrap. Add breaks:true to the lib.parse options in renderMarkdown (ui/chat.js) and add a test asserting a two-line plain message keeps its line break.

Mission: `d398e923-10d9-46b0-af7c-32a71e95a347`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **28-001**: marked.parse is invoked with breaks:true so a single newline becomes a <br> in rendered output
- **28-002**: A two-line plain message renders with its line break preserved
- **28-003**: node tools/tests/test_chat_handlers.mjs passes with the new assertion
- **28-004**: cargo test, clippy -D warnings and cargo fmt --check remain clean

## Goals

- Bind feature `frontend-markdown-line-breaks-fix` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change28-frontend-markdown-line-breaks-fix/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-MD-002`
