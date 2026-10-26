# Host-bridge security notes

Scope: the two agent-facing listeners in the Rust core — RFC HTTP (`:55080`)
and SSH (`:55022`). Both are gated by the single master toggle
("Allow agent control", `rfc_enabled`, default **OFF**).

## Real security boundary

| Layer | Role |
|---|---|
| Master toggle OFF (default) | No socket exists at all: neither port listens. |
| Password auth (`rfc_password` HMAC / `ssh_password`) | The actual boundary once the toggle is ON. |
| Wildcard bind (contract ruling R2) | Both listeners bind the wildcard address so the cloud agent can reach the host; exposure is accepted because auth is mandatory. |
| Fail-closed bind | An enabled listener with an empty/whitespace password refuses to bind and logs an error. A wildcard, passwordless listener must never exist. |
| Command denylist (`ssh/guard.rs`) | **Best-effort secondary layer only.** Not a security boundary. |

The SSH guard exists so a typo or a prompt-injected instruction does not wipe
the host. It cannot stop a determined command — the authenticated shell is
full shell access **by design**. Do not treat a denylist pass as a security
guarantee, and do not widen the denylist without a security review.

## SSH denylist blind spots (known, accepted)

The guard vets the interactive line buffer and the `exec_request` string: it
splits on `;`, `&&`, `||`, `|`, resolves the program name, and matches the
named destructive population (`rm -rf /` variants, `mkfs*`, `dd of=/dev/*`,
fork bombs, `shutdown`/`reboot`/`poweroff`/`halt`, `init 0|6`). Everything
below bypasses it:

1. **Backslash variants** — `\rm -rf /`: the escaped command name does not
   match the `rm` program token.
2. **Wrapper prefixes** — `busybox rm -rf /`, `nohup rm -rf /`,
   `env rm -rf /`, `timeout 5 rm -rf /`, `xargs rm`, `setsid …`: the first
   token is the wrapper, so the real command is only seen as an argument.
3. **Shell re-entry** — `bash -c 'rm -rf /'`, `sh -c …`, `zsh -c …`: the
   inner script is opaque to the line-oriented parser.
4. **`eval`** — `eval "$CMD"` with the payload built at runtime.
5. **Command substitution** — `echo $(rm -rf /)`, `` `rm -rf /` ``,
   `$(printf 'rm -rf /')`: substitution bodies are not parsed.
6. **Control-key line-buffer desync** — the server mirrors the typed line to
   vet it, while the tty/readline owns the real buffer. `Ctrl-C` (abort line)
   and `Ctrl-U` (kill line) are forwarded to the pty but the mirror is not
   cleared, and `Ctrl-W`/`Ctrl-K`/arrow-key edits are not tracked, so the
   vetted string can differ from what the shell actually executes. Binary or
   pasted control sequences have the same effect.
7. **Multi-line constructs** — heredocs, `for` loops, and function bodies are
   vetted per input line, not as one program.
8. **Non-shell paths** — anything that is not typed into the PTY (e.g. a
   script written through the RFC file APIs and then executed) never passes
   the guard.

Residual risk is accepted: the design contract says an enabled bridge is full
remote control, and the mitigation is the master toggle (default OFF) plus a
required password, not the denylist.

## RFC server invariants

- HMAC-SHA256 over the exact received `rfc_input` string bytes, lowercase hex,
  constant-time compare (`hmac::Mac::verify_slice`); mismatch → 403.
- Hard dispatch whitelist on `(module, function_name)`; unknown → 403.
- Path jail: every path argument is canonicalized and must stay inside a
  configured `allowed_paths` root; symlink escape, `..` traversal, and an
  empty root list all deny (403).
- `_provide_root_password` returns RSA-OAEP-SHA256 ciphertext of the configured
  SSH password; it never returns plaintext.

## Operational guidance

- Keep the toggle OFF unless the agent needs host control for that session.
- Set a distinct, non-trivial password before enabling; an empty password
  keeps the listeners closed.
- The wildcard bind is intentional (cloud reachability). Front the host with
  a firewall if exposure must be limited.