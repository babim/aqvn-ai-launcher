# Proposal: change5-rfc-sandbox-handlers

## Why

This change is auto-scaffolded by `opsx-mission-bind.mjs` (AQVN Round 3 M2-F1) to bind Factory mission feature `rfc-sandbox-handlers` (milestone `rfc-ssh-bridge`) to an openspec change. The feature description:

> Sandbox + full whitelist handler surface per research/rfc-protocol.md §5. sandbox.rs: jail every path arg to allowed_paths — expand ~, map /a0/... prefix → sandbox root, canonicalize, symlink-escape containment, default-deny → 403. Handlers with EXACT response shapes: helpers.rfc_files (14 fns: binary read/write base64, delete file/folder, _list_folder_impl entries {name,path,is_file,is_dir,size,modified float epoch} sorted by name, make_dirs, exists trio, get_subdirectories fnmatch, zip_dir (system temp, return existing valid zip path), move_file, read_directory base64-zip); plugins._text_editor.helpers.file_ops (read_file numbered-lines {content,total_lines,warnings,error}, write_file {total_lines,error}, file_info, apply_patch, apply_context_patch_file, apply_exact_replace_file, validate_edits → [list,str]); api.get_work_dir_files.get_files; api.file_info.get_file_info; api.edit_work_dir_file load_file/save_file; api.delete_work_dir_file(s); api.rename_work_dir_file rename_item/create_folder/move_items; api.upload_work_dir_files.upload_file; helpers.rfc_exchange._provide_root_password (hex PEM RSA-2048 in → hex RSA-OAEP MGF1-SHA256 no-label ciphertext of ssh_password out, rsa crate oaep feature); plugins._code_execution...make_dir (jailed). Config hot-reload: password/paths/ports/toggle changes apply without app restart (tokio watch → listener restart, same process PID).

Mission: `c82a03df-dfc7-438f-b44b-9089dcc40a8f`. The change is proposals-only scaffolding; implementation is deferred to the doit drain (aqvn-opsx-doit).

## What Changes

- **5-001**: Every whitelist fn returns exact spec shape (integration tests per family)
- **5-002**: Sandbox: escape read/write/delete, ../ traversal, symlink escape → 403; /a0/ prefix maps into sandbox root
- **5-003**: _provide_root_password round-trips: python harness crypto decrypts to exact ssh_password
- **5-004**: make_dir jailed like all path ops
- **5-005**: Settings hot-reload works (password/paths/port/toggle), process PID unchanged

## Goals

- Bind feature `rfc-sandbox-handlers` to a 1:1 openspec change (idempotent).
- Map every expected behavior to a task atom with a VAL-* test_strategy.
- Populate `mission_lineage` (schema v1.1) linking change ↔ feature ↔ mission.

## Non-Goals

- Do not implement the feature behavior in this scaffold; implementation is deferred to the doit drain.
- Do not edit source files outside `openspec/changes/change5-rfc-sandbox-handlers/` during authoring.

## Validation Contract Assertions Fulfilled

- `VAL-RFC-009`
- `VAL-RFC-010`
- `VAL-RFC-011`
- `VAL-RFC-012`
- `VAL-RFC-013`
- `VAL-RFC-014`
- `VAL-RFC-015`
- `VAL-RFC-016`
- `VAL-RFC-017`
- `VAL-RFC-018`
- `VAL-RFC-019`
- `VAL-RFC-020`
- `VAL-RFC-021`
- `VAL-RFC-022`
- `VAL-RFC-023`
- `VAL-RFC-024`
- `VAL-RFC-026`
