#!/usr/bin/env bash
# Regression coverage for the journey rig capabilities that the cross-area
# run depends on:
#   VAL-CROSS-001  the rig starts from contract defaults supplied by env
#                  (agent control OFF, no allowed paths) and writes them to
#                  the isolated settings store.
#   VAL-CROSS-003  `tray_rig.sh restart` relaunches the app on the same
#                  display+config (same store, new pid) without touching
#                  Xvfb / D-Bus / tray / notification hosts.
set -euo pipefail

RIG=/home/babim/Desktop/dev/dev.aqvn/ai-launcher/tools/tray_rig.sh
RIG_DIR="${AQVN_TEST_RIG_DIR:-/tmp/aqvn-rig-restart-test}"
DISPLAY_NUM="${AQVN_TEST_RIG_DISPLAY:-:99}"

cleanup() {
  AQVN_RIG_DIR="$RIG_DIR" AQVN_RIG_DISPLAY="$DISPLAY_NUM" bash "$RIG" stop >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

fail() {
  echo "FAIL $*" >&2
  exit 1
}
pass() { echo "PASS $*"; }

export AQVN_RIG_DIR="$RIG_DIR"
export AQVN_RIG_DISPLAY="$DISPLAY_NUM"
export AQVN_RIG_RFC_ENABLED=false
export AQVN_RIG_ALLOWED_PATHS=""
export AQVN_RIG_SERVER_URL="http://127.0.0.1:5000"

bash "$RIG" stop >/dev/null 2>&1 || true
rm -rf "$RIG_DIR"

setsid --fork bash "$RIG" start >/dev/null 2>&1
for _ in $(seq 1 60); do
  [ -f "$RIG_DIR/env.sh" ] && break
  sleep 0.5
done
[ -f "$RIG_DIR/env.sh" ] || fail "VAL-CROSS-001 rig did not start"

SETTINGS="$RIG_DIR/config/net.aqvn.workspace/settings.json"
[ -f "$SETTINGS" ] || fail "VAL-CROSS-001 settings store missing"
python3 - "$SETTINGS" <<'PY' || exit 1
import json
import sys

path = sys.argv[1]
data = json.load(open(path, encoding="utf-8"))
assert data["server_url"] == "http://127.0.0.1:5000", data
assert data["rfc_enabled"] is False, data
assert data["allowed_paths"] == [], data
assert data["rfc_port"] == 55080 and data["ssh_port"] == 55022, data
print("PASS VAL-CROSS-001 rig store reflects env defaults (toggle off, no paths)")
PY

pid_before="$(cat "$RIG_DIR/pids/app.pid")"
kill -0 "$pid_before" 2>/dev/null || fail "VAL-CROSS-003 app not running before restart"
bash "$RIG" restart >/dev/null 2>&1 || fail "VAL-CROSS-003 restart command missing or failed"
pid_after="$(cat "$RIG_DIR/pids/app.pid")"
[ "$pid_after" != "$pid_before" ] || fail "VAL-CROSS-003 restart reused pid $pid_before"
kill -0 "$pid_after" 2>/dev/null || fail "VAL-CROSS-003 relaunched app not alive"

for _ in $(seq 1 40); do
  if DISPLAY="$DISPLAY_NUM" xdotool search --onlyvisible --name 'AQVN AI Workspace' >/dev/null 2>&1; then
    break
  fi
  sleep 0.5
done
DISPLAY="$DISPLAY_NUM" xdotool search --onlyvisible --name 'AQVN AI Workspace' >/dev/null 2>&1 ||
  fail "VAL-CROSS-003 window did not come back after restart"

if ss -tln | grep -qE ':(55080|55022)\b'; then
  fail "VAL-CROSS-003 toggle-off store must not open control ports"
fi
pass "VAL-CROSS-003 restart relaunched pid $pid_after on the same store, ports stay closed"

bash "$RIG" stop >/dev/null 2>&1 || true
echo "TRAY RIG JOURNEY TEST PASS"
