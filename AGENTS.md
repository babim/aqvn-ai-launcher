# Agent Guidance

Verified environment facts and guardrails for this repository.

## Spec-Lock Gate Rule

- Source edits under `src/` and `src-tauri/src/` require an active OpenSpec task.
- An active (non-archive) `openspec/changes/*/tasks.md` must contain a checkbox or table row naming the target file path.
- The gate hook is always-on and not bypassable: `AQVN_HOOK_MODE=relax` does not disable it, and no allowlist or environment variable override exists.
- Docs (`*.md`), `docs/`, and tests under `tests/` are exempt.

## Serena Rust LSP Configuration

- Rust LSP (`rust-analyzer 1.97.1` at `~/.cargo/bin/rust-analyzer`) is configured in `.serena/project.yml` (`language_servers: [rust]`).
- If an existing Serena MCP daemon process was started prior to configuration update, restart the session/MCP server to reload `rust-analyzer`.
- For reading Rust source files directly, use `desktop-commander read_file` or builtin `Read`.

## Test Timings

- Quick: `cargo test --lib` and `cargo test --bin aqvn-agent` complete in seconds.
- Node suite: `node --test tools/tests/test_chat_handlers.mjs` completes in ~3s (170 tests).
- Full suite: `cargo test --all-targets` takes ~75s and exceeds standard 30s tool timeouts; run with an extended timeout.

## Environment Test Hygiene (`TEST_ENV_MUTEX`)

- Tests that mutate environment variables must serialize with `static TEST_ENV_MUTEX: Mutex<()>`.
- Concurrent environment mutation races and causes flaky failures across parallel test threads.
