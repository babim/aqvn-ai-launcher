# RFC / Host-Bridge Protocol Notes

This directory is the home for protocol notes about the upstream cloud workspace
server we stay wire-compatible with (upstream project: Agent Zero v2.14).

## Whitelabel exclusion

The repo-wide whitelabel gate (`rg -i "agentzero|agent0|agent zero"`) excludes
`docs/rfc-spec/**`. Naming the upstream project is allowed **only here**. Code,
UI strings, bundle metadata, comments, and commit content stay AQVN-branded.

## Planned notes

- `security-notes.md` — exposure model, real security boundary, SSH denylist
  blind spots (the guard is best-effort, not a boundary)
- `rfc-wire.md` — `POST /api/rfc` envelope, HMAC-SHA256 hex, response shapes
- `ssh-bridge.md` — password exchange (`_provide_root_password`), PTY behavior
- `chat-wire.md` — login/CSRF, socket.io state push, message_async

Authoritative working specs live in the mission research directory; final notes
move here as the implementation lands.
