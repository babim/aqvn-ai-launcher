# Proposal: change45-feat-gateway-settings-scopes

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `feat-gateway-settings-scopes` (milestone `gateway-hub-phase2`) to an openspec change. The feature description:

> Add user-configurable security scopes (files, code_execution, browser, computer_use) and master gateway switch in settings and AppConfig with hot-reload support.

Mission: `d398e923-10d9-46b0-af7c-32a71e95a347`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **45-001**: Settings UI exposes checkboxes for each scope and master connector switch
- **45-002**: Toggling master off immediately terminates the outbound connection
- **45-003**: connector_hello scopes match active configuration

## Goals

- Bind feature `feat-gateway-settings-scopes` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change45-feat-gateway-settings-scopes/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-GATEWAY-006`
