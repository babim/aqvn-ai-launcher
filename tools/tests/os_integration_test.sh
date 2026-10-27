#!/usr/bin/env bash
# Static gates for the OS integration feature (tray glyphs, UI wiring, docs).
# Run from the repo root: bash tools/tests/os_integration_test.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fail() { echo "FAIL: $*" >&2; exit 1; }

cd "$ROOT"

[ -f src-tauri/icons/tray-idle.png ] || fail "missing src-tauri/icons/tray-idle.png"
[ -f src-tauri/icons/tray-busy.png ] || fail "missing src-tauri/icons/tray-busy.png"
cmp -s src-tauri/icons/tray-idle.png src-tauri/icons/tray-busy.png \
  && fail "tray icons must differ between idle and busy states"

[ -x tools/tray_rig.sh ] || fail "tools/tray_rig.sh missing or not executable"

rg -q "set_tray_busy" ui/chat.js || fail "chat.js must report busy state to Rust"
rg -q "notify_task_complete" ui/chat.js || fail "chat.js must fire the completion notification"

rg -qi "wayland" docs/os-integration.md || fail "docs must document the Wayland limitation"
rg -qi "tray menu" docs/os-integration.md || fail "docs must name the tray-menu fallback"

echo "os_integration_test: PASS"
