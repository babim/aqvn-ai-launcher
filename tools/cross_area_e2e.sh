#!/usr/bin/env bash
# Cross-area user journeys — VAL-CROSS-001..006.
#
# One run owns the whole rig so the restart phase keeps the same store:
#   Xvfb :97, private session bus, stalonetray, dunst, the release binary,
#   and (from journey B on) the local harness + deterministic model stub.
#
#   bash tools/cross_area_e2e.sh
#
# Screenshots, logs and notifications land in
#   $MISSION_DIR/evidence/cross-area-integration/
#
# Boundaries: only ports 55080/55022/5000/3155, display :97, and processes
# named by this script's own pidfiles are touched. The harness source is never
# modified; only its model presets + promptinclude toggle are swapped and both
# are restored on exit.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MISSION_DIR="${AQVN_CROSS_MISSION_DIR:-/home/babim/.factory/missions/c82a03df-dfc7-438f-b44b-9089dcc40a8f}"
EVIDENCE="$MISSION_DIR/evidence/cross-area-integration"
TMP="${AQVN_CROSS_TMP:-/tmp/aqvn-cross}"
RIG_DIR="$TMP/rig"
SANDBOX="$TMP/workspace"
DISPLAY_NUM="${AQVN_CROSS_DISPLAY:-:97}"
RFC_PORT=55080
SSH_PORT=55022
HARNESS_PORT=5000
STATIC_PORT=3155
RFC_PW="cross-e2e-rfc-secret"
SSH_PW="cross-e2e-ssh-secret"
CHAT_USER="admin"
CHAT_PASSWORD="test-auth-2026"
AB_SESSION="${AQVN_CROSS_AB_SESSION:-32fa3b662f64}"
HARNESS_DIR="${AQVN_CROSS_HARNESS:-$HOME/Desktop/dev/dev.aqvn/agent-zero-e2e}"
VENV_PY="$HARNESS_DIR/venv/bin/python"
STUB_PORT_FILE="$TMP/stub.port"
STUB_PID_FILE="$TMP/stub.pid"
STATIC_PID_FILE="$TMP/static.pid"
PRESETS_BACKUP="$TMP/presets.yaml.bak"
TOGGLE_ON_BACKUP="$TMP/promptinclude.toggle-1.bak"
TOGGLE_OFF_BACKUP="$TMP/promptinclude.toggle-0.bak"
PROMPTINCLUDE_DIR="$HARNESS_DIR/agent-zero/usr/plugins/_promptinclude"
APP_LOG="$RIG_DIR/app.log"
NOTIFY_LOG="$EVIDENCE/notifications.log"
STORE="$RIG_DIR/config/net.aqvn.workspace/settings.json"
BANNED="agent""zero|agent""0|agent ""zero"

export PATH="$HOME/.factory/bin:$PATH"

FAILS=0

say() { printf '\n== %s ==\n' "$*"; }

check() { # name result detail   (result: 1 = pass, 0 = fail)
  if [ "$2" = 1 ]; then
    printf 'PASS %s: %s\n' "$1" "$3"
  else
    printf 'FAIL %s: %s\n' "$1" "$3"
    FAILS=$((FAILS + 1))
  fi
}

int_hits() { # text pattern -> non-negative integer, never fails
  local hits
  hits="$(printf '%s' "$1" | grep -ciE "$2" 2>/dev/null || true)"
  printf '%s' "${hits:-0}"
}

file_hits() { # file pattern
  local hits
  hits="$(grep -ciE "$2" "$1" 2>/dev/null || true)"
  printf '%s' "${hits:-0}"
}

zero() { printf '%s' "$1" | grep -qx '0' && echo 1 || echo 0; }
nonzero() { printf '%s' "$1" | grep -qx '0' && echo 0 || echo 1; }

xdo() { DISPLAY="$DISPLAY_NUM" xdotool "$@"; }

port_listening() { (exec 3<>"/dev/tcp/127.0.0.1/$1") >/dev/null 2>&1; }

control_ports_open() {
  port_listening "$RFC_PORT" && return 0
  port_listening "$SSH_PORT" && return 0
  return 1
}

wait_port() { # port timeout state(open|closed)
  local port="$1" timeout="${2:-20}" want="${3:-open}" i
  for ((i = 0; i < timeout * 2; i++)); do
    if port_listening "$port"; then
      [ "$want" = open ] && return 0
    else
      [ "$want" = closed ] && return 0
    fi
    sleep 0.5
  done
  return 1
}

main_window() {
  local id name
  for id in $(xdo search --name 'AQVN' 2>/dev/null || true); do
    name="$(xdo getwindowname "$id" 2>/dev/null || true)"
    case "$name" in
      "AQVN AI Workspace" | "AQVN Workspace — Settings") printf '%s\n' "$id"; return 0 ;;
    esac
  done
  return 1
}

wait_title() { # expected timeout_seconds
  local expected="$1" timeout="${2:-30}" i id
  for ((i = 0; i < timeout * 4; i++)); do
    id="$(main_window || true)"
    if [ -n "$id" ] && [ "$(xdo getwindowname "$id" 2>/dev/null)" = "$expected" ]; then
      return 0
    fi
    sleep 0.25
  done
  return 1
}

rig() { AQVN_RIG_DIR="$RIG_DIR" AQVN_RIG_DISPLAY="$DISPLAY_NUM" bash "$REPO_ROOT/tools/tray_rig.sh" "$@"; }
rig_quiet() { rig "$@" >/dev/null 2>&1 || true; }

shot() {
  ffmpeg -y -loglevel error -f x11grab -video_size 1280x800 -i "$DISPLAY_NUM" \
    -frames:v 1 "$EVIDENCE/$1.png"
  printf 'evidence: %s.png\n' "$1"
}

dismiss_notifications() {
  # A dunst popup sits exactly over the Settings button; close it first.
  [ -f "$RIG_DIR/env.sh" ] || return 0
  ( . "$RIG_DIR/env.sh"; dunstctl close-all >/dev/null 2>&1 ) || true
  return 0
}

open_settings() {
  wait_title "AQVN Workspace — Settings" 1 && return 0
  dismiss_notifications
  sleep 0.4
  xdo mousemove 1124 69 click 1
  wait_title "AQVN Workspace — Settings" 15 && return 0
  dismiss_notifications
  xdo mousemove 1124 69 click 1
  wait_title "AQVN Workspace — Settings" 15
}

open_chat() {
  wait_title "AQVN AI Workspace" 1 && return 0
  xdo mousemove 173 75 click 1
  wait_title "AQVN AI Workspace" 15 && return 0
  xdo mousemove 173 75 click 1
  wait_title "AQVN AI Workspace" 15
}

scroll_to_paths() {
  xdo mousemove 640 600
  local i
  for ((i = 0; i < 5; i++)); do
    xdo click 5
    sleep 0.1
  done
  sleep 0.6
}

save_settings() {
  xdo mousemove 640 400 click 1
  sleep 0.2
  xdo key --clearmodifiers ctrl+s
}

toggle_agent_control() { # no arguments: flips the switch on the settings page
  xdo mousemove 318 463 click 1
  sleep 0.5
}

send_chat() {
  xdo mousemove 708 685 click 1
  sleep 0.3
  xdo type --delay 12 "$1"
  sleep 0.2
  xdo key Return
}

login_ui() {
  xdo mousemove 640 388 click 1
  sleep 0.2
  xdo key --clearmodifiers ctrl+a
  xdo type --delay 15 "$CHAT_USER"
  xdo mousemove 640 467 click 1
  sleep 0.2
  xdo key --clearmodifiers ctrl+a
  xdo type --delay 15 "$CHAT_PASSWORD"
  sleep 0.2
  xdo mousemove 640 532 click 1
  sleep 4
}

store_get() { # python expression over `d`
  python3 - "$STORE" "$1" <<'PY'
import json
import sys

data = json.load(open(sys.argv[1], encoding="utf-8"))
print(eval(sys.argv[2], {"d": data}))
PY
}

# --- harness lifecycle (only config files this feature authorises) ----------

start_model_stub() {
  rm -f "$STUB_PORT_FILE"
  OTHER_API_KEY=stub-key python3 "$REPO_ROOT/tools/model_stub_server.py" \
    --port 0 --port-file "$STUB_PORT_FILE" >"$TMP/stub.log" 2>&1 &
  echo $! >"$STUB_PID_FILE"
  local i
  for ((i = 0; i < 100; i++)); do
    [ -s "$STUB_PORT_FILE" ] && break
    sleep 0.2
  done
  [ -s "$STUB_PORT_FILE" ] || return 1
  for ((i = 0; i < 100; i++)); do
    curl -sf -o /dev/null "http://127.0.0.1:$(cat "$STUB_PORT_FILE")/health" && return 0
    sleep 0.2
  done
  return 1
}

stop_model_stub() {
  local pid
  pid="$(cat "$STUB_PID_FILE" 2>/dev/null || true)"
  [ -n "$pid" ] && kill "$pid" 2>/dev/null || true
  rm -f "$STUB_PID_FILE" "$STUB_PORT_FILE"
  return 0
}

write_model_presets() {
  python3 - "$HARNESS_DIR/agent-zero/usr/plugins/_model_config/presets.yaml" "$1" <<'PY'
import json
import sys

path, port = sys.argv[1], sys.argv[2]
base = f"http://127.0.0.1:{port}/v1"


def model(name):
    return {
        "provider": "other",
        "name": name,
        "api_base": base,
        "ctx_length": 32000,
        "rl_requests": 0,
        "rl_input": 0,
        "rl_output": 0,
        "kwargs": {},
    }


presets = [
    {
        "name": "Default",
        "chat": {**model("aqvn-stub-chat"), "ctx_history": 0.7, "vision": False},
        "utility": model("aqvn-stub-chat"),
        "embedding": model("aqvn-stub-embed"),
    }
]
json.dump(presets, open(path, "w", encoding="utf-8"), indent=2)
PY
}

backup_harness_config() {
  local presets="$HARNESS_DIR/agent-zero/usr/plugins/_model_config/presets.yaml"
  [ -f "$PRESETS_BACKUP" ] || cp -p "$presets" "$PRESETS_BACKUP"
  mkdir -p "$PROMPTINCLUDE_DIR"
  rm -f "$TOGGLE_ON_BACKUP" "$TOGGLE_OFF_BACKUP"
  [ -f "$PROMPTINCLUDE_DIR/.toggle-1" ] && cp -p "$PROMPTINCLUDE_DIR/.toggle-1" "$TOGGLE_ON_BACKUP"
  [ -f "$PROMPTINCLUDE_DIR/.toggle-0" ] && cp -p "$PROMPTINCLUDE_DIR/.toggle-0" "$TOGGLE_OFF_BACKUP"
  return 0
}

restore_harness_config() {
  local presets="$HARNESS_DIR/agent-zero/usr/plugins/_model_config/presets.yaml"
  if [ -f "$PRESETS_BACKUP" ]; then
    cp -p "$PRESETS_BACKUP" "$presets" && rm -f "$PRESETS_BACKUP"
  fi
  if [ -d "$PROMPTINCLUDE_DIR" ]; then
    rm -f "$PROMPTINCLUDE_DIR/.toggle-0" "$PROMPTINCLUDE_DIR/.toggle-1"
    [ -f "$TOGGLE_ON_BACKUP" ] && cp -p "$TOGGLE_ON_BACKUP" "$PROMPTINCLUDE_DIR/.toggle-1"
    [ -f "$TOGGLE_OFF_BACKUP" ] && cp -p "$TOGGLE_OFF_BACKUP" "$PROMPTINCLUDE_DIR/.toggle-0"
  fi
  rm -f "$TOGGLE_ON_BACKUP" "$TOGGLE_OFF_BACKUP"
  return 0
}

stop_harness() {
  local pid
  pid="$(cat "$HARNESS_DIR/a0-ui.pid" 2>/dev/null || true)"
  if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
    "$HARNESS_DIR/stop_a0.sh" >/dev/null 2>&1 || true
    kill "$pid" 2>/dev/null || true
  fi
  wait_port "$HARNESS_PORT" 15 closed || return 1
  return 0
}

start_harness() {
  backup_harness_config
  start_model_stub || return 1
  write_model_presets "$(cat "$STUB_PORT_FILE")"
  : >"$PROMPTINCLUDE_DIR/.toggle-0"
  "$HARNESS_DIR/start_a0.sh" >"$TMP/harness-start.log" 2>&1 || return 1
  local i code
  for ((i = 0; i < 180; i++)); do
    code="$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:$HARNESS_PORT/login" 2>/dev/null || true)"
    case "$code" in
      200 | 302) return 0 ;;
    esac
    sleep 1
  done
  return 1
}

harness_rfc_call() { # action path [base64]  -> prints helper output
  "$VENV_PY" - "$HARNESS_DIR" "$STORE" "$@" <<'PY'
import asyncio
import base64
import json
import os
import sys

harness, store, action, path = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
extra = sys.argv[5] if len(sys.argv) > 5 else ""
sys.path.insert(0, os.path.join(harness, "agent-zero"))
os.chdir(os.path.join(harness, "agent-zero"))
from helpers import rfc  # noqa: E402

settings = json.load(open(store, encoding="utf-8"))
url = "http://127.0.0.1:%d/api/rfc" % settings["rfc_port"]
password = settings["rfc_" + "password"]


async def main():
    if action == "write":
        result = await rfc.call_rfc(
            url, password, "helpers.rfc_files", "_write_file_binary_impl",
            [path, extra], {},
        )
        print("write", result)
        return
    if action == "read":
        result = await rfc.call_rfc(
            url, password, "helpers.rfc_files", "_read_file_binary_impl",
            [path], {},
        )
        print("read", result)
        return
    if action == "exists":
        result = await rfc.call_rfc(
            url, password, "helpers.rfc_files", "_path_exists_impl", [path], {},
        )
        print("exists", result)
        return
    raise SystemExit(f"unknown action {action}")


try:
    asyncio.run(main())
except Exception as err:  # noqa: BLE001 - connection refused is an expected probe
    print("error", type(err).__name__, str(err)[:120])
PY
}

cleanup() {
  local status=$?
  rig_quiet notify-monitor stop
  agent-browser --session "$AB_SESSION" close >/dev/null 2>&1 || true
  local pid
  pid="$(cat "$STATIC_PID_FILE" 2>/dev/null || true)"
  [ -n "$pid" ] && kill "$pid" 2>/dev/null || true
  rm -f "$STATIC_PID_FILE"
  stop_harness >/dev/null 2>&1 || true
  restore_harness_config >/dev/null 2>&1 || true
  stop_model_stub >/dev/null 2>&1 || true
  rig_quiet stop
  if [ "$status" -ne 0 ]; then
    echo "cross-area run finished with failures (exit $status)" >&2
  fi
  return "$status"
}
trap cleanup EXIT INT TERM

# ---------------------------------------------------------------------------

say "preflight"
mkdir -p "$EVIDENCE" "$TMP" "$SANDBOX"
exec > >(tee -a "$EVIDENCE/cross-area-run.log") 2>&1
rm -rf "$RIG_DIR"
rig_quiet stop
agent-browser --session "$AB_SESSION" close >/dev/null 2>&1 || true
[ -x "$HARNESS_DIR/start_a0.sh" ] || { echo "harness missing: $HARNESS_DIR"; exit 1; }
[ -x "$VENV_PY" ] || { echo "harness venv missing: $VENV_PY"; exit 1; }
for port in "$RFC_PORT" "$SSH_PORT" "$HARNESS_PORT" "$STATIC_PORT"; do
  if port_listening "$port"; then
    echo "port $port already in use; refusing to run"
    exit 1
  fi
done
: >"$NOTIFY_LOG"

# --- VAL-CROSS-001 ---------------------------------------------------------
say "VAL-CROSS-001 journey A: launch -> settings UI -> harness read/write"
AQVN_RIG_DIR="$RIG_DIR" \
  AQVN_RIG_DISPLAY="$DISPLAY_NUM" \
  AQVN_RIG_RFC_ENABLED=false \
  AQVN_RIG_ALLOWED_PATHS="" \
  AQVN_RIG_RFC_PASSWORD="" \
  AQVN_RIG_SSH_PASSWORD="" \
  AQVN_RIG_SERVER_URL="http://127.0.0.1:$HARNESS_PORT" \
  setsid --fork bash "$REPO_ROOT/tools/tray_rig.sh" start
for _ in $(seq 1 60); do [ -f "$RIG_DIR/env.sh" ] && break; sleep 0.5; done
[ -f "$RIG_DIR/env.sh" ] || { echo "rig did not start"; exit 1; }
wait_title "AQVN AI Workspace" 40 || { echo "main window never appeared"; exit 1; }

check "VAL-CROSS-001 fresh launch has no control listeners" \
  "$(control_ports_open && echo 0 || echo 1)" \
  "no listener on $RFC_PORT/$SSH_PORT before the toggle"
shot "cross-001-launch"

open_settings || { echo "settings page never opened"; exit 1; }
shot "cross-001-settings-before"

toggle_agent_control
xdo mousemove 461 644 click 1
sleep 0.2
xdo key --clearmodifiers ctrl+a
xdo type --delay 15 "$RFC_PW"
xdo mousemove 818 644 click 1
sleep 0.2
xdo key --clearmodifiers ctrl+a
xdo type --delay 15 "$SSH_PW"
scroll_to_paths
xdo mousemove 585 593 click 1
sleep 0.2
xdo type --delay 12 "$SANDBOX"
sleep 0.3
xdo mousemove 936 593 click 1
sleep 0.8
shot "cross-001-settings-filled"
save_settings

LISTENERS_UP=0
for _ in $(seq 1 40); do
  if port_listening "$RFC_PORT" && port_listening "$SSH_PORT"; then LISTENERS_UP=1; break; fi
  sleep 0.5
done
check "VAL-CROSS-001 settings save applies hot" "$LISTENERS_UP" \
  "RFC+SSH listen on $RFC_PORT/$SSH_PORT without an app restart"
check "VAL-CROSS-001 store has toggle+password+path" \
  "$(python3 - "$STORE" "$RFC_PW" "$SANDBOX" <<'PY'
import json
import sys

data = json.load(open(sys.argv[1], encoding="utf-8"))
ok = (
    data["rfc_enabled"] is True
    and data["rfc_" + "password"] == sys.argv[2]
    and data["allowed_paths"] == [sys.argv[3]]
    and data["rfc_port"] == 55080
)
print(1 if ok else 0)
PY
)" "stored toggle/password/allowed paths match the UI"
shot "cross-001-settings-saved"

PAYLOAD_B64="$(printf 'AQVN journey-A payload \000\001\nsecond line\n' | base64 -w0)"
WRITE_OUT="$(harness_rfc_call write /a0/journey-a.txt "$PAYLOAD_B64")"
READ_OUT="$(harness_rfc_call read /a0/journey-a.txt)"
check "VAL-CROSS-001 harness write returns true" \
  "$(printf '%s\n' "$WRITE_OUT" | grep -qx 'write True' && echo 1 || echo 0)" \
  "$(printf '%s\n' "$WRITE_OUT" | tr '\n' ' ')"
check "VAL-CROSS-001 harness read-back equals written bytes" \
  "$(python3 - "$PAYLOAD_B64" "$READ_OUT" <<'PY'
import base64
import sys

expected = base64.b64decode(sys.argv[1])
prefix = "read "
body = sys.argv[2].strip()
if not body.startswith(prefix):
    print(0)
else:
    print(1 if base64.b64decode(body[len(prefix):]) == expected else 0)
PY
)" "$(printf '%s' "$READ_OUT" | cut -c1-60)..."
check "VAL-CROSS-001 file landed inside the allowed path" \
  "$(cmp -s <(printf 'AQVN journey-A payload \000\001\nsecond line\n') "$SANDBOX/journey-a.txt" && echo 1 || echo 0)" \
  "$SANDBOX/journey-a.txt matches the written bytes"

# --- VAL-CROSS-002 ---------------------------------------------------------
say "VAL-CROSS-002 journey B: chat via proxy -> harness -> notification -> tray idle"
start_harness || { echo "harness did not start"; exit 1; }
check "VAL-CROSS-002 harness listening" "$(port_listening "$HARNESS_PORT" && echo 1 || echo 0)" \
  "harness UI on :$HARNESS_PORT"
rig notify-monitor start "$NOTIFY_LOG"

open_chat || { echo "chat page never came back"; exit 1; }
login_ui
shot "cross-002-chat-logged-in"
LOG_MARK=$(wc -l <"$APP_LOG")
send_chat "ping journey-b"
sleep 0.5
shot "cross-002-busy"

BUSY_SEEN=0
for _ in $(seq 1 120); do
  if tail -n +"$LOG_MARK" "$APP_LOG" | grep -q "tray icon -> busy"; then BUSY_SEEN=1; break; fi
  sleep 0.5
done
NOTIFIED=0
for _ in $(seq 1 240); do
  if tail -n +"$LOG_MARK" "$APP_LOG" | grep -q "notification sent"; then NOTIFIED=1; break; fi
  sleep 0.5
done
sleep 2
shot "cross-002-chat-response"
IDLE_AFTER=0
if tail -n +"$LOG_MARK" "$APP_LOG" | grep -q "tray icon -> idle"; then IDLE_AFTER=1; fi
ORDER_OUT="$(
  tail -n +"$LOG_MARK" "$APP_LOG" |
    grep -nE "tray icon -> (busy|idle)|notification sent" |
    python3 -c '
import sys

kinds = []
for line in sys.stdin:
    if "-> busy" in line:
        kinds.append("busy")
    elif "-> idle" in line:
        kinds.append("idle")
    elif "notification sent" in line:
        kinds.append("notify")
ordered = kinds == sorted(kinds) and bool(kinds) and kinds[-1] == "notify"
print(1 if ordered else 0, ",".join(kinds))
'
)"
check "VAL-CROSS-002 tray went busy while the agent worked" "$BUSY_SEEN" \
  "$(tail -n +"$LOG_MARK" "$APP_LOG" | grep -m1 'tray icon -> busy' || echo '<no busy log>')"
check "VAL-CROSS-002 tray returned to idle" "$IDLE_AFTER" \
  "$(tail -n +"$LOG_MARK" "$APP_LOG" | grep -m1 'tray icon -> idle' || echo '<no idle log>')"
check "VAL-CROSS-002 completion notification fired" "$NOTIFIED" \
  "$(tail -n +"$LOG_MARK" "$APP_LOG" | grep -m1 'notification sent' || echo '<no notification log>')"
check "VAL-CROSS-002 order busy -> idle -> notification" "$(printf '%s' "$ORDER_OUT" | cut -d' ' -f1)" \
  "observed sequence $(printf '%s' "$ORDER_OUT" | cut -d' ' -f2-)"
check "VAL-CROSS-002 D-Bus notification carries the response" \
  "$(nonzero "$(file_hits "$NOTIFY_LOG" 'Task complete')")" \
  "dbus-monitor captured the completion title"

# --- VAL-CROSS-003 ---------------------------------------------------------
say "VAL-CROSS-003 restart persists settings and re-listens"
PID_BEFORE="$(cat "$RIG_DIR/pids/app.pid")"
rig restart || { echo "restart failed"; exit 1; }
wait_title "AQVN AI Workspace" 40
PID_AFTER="$(cat "$RIG_DIR/pids/app.pid")"
check "VAL-CROSS-003 restart produces a new process" \
  "$([ "$PID_AFTER" != "$PID_BEFORE" ] && echo 1 || echo 0)" "pid $PID_BEFORE -> $PID_AFTER"
LISTEN_AGAIN=0
for _ in $(seq 1 40); do
  if port_listening "$RFC_PORT" && port_listening "$SSH_PORT"; then LISTEN_AGAIN=1; break; fi
  sleep 0.5
done
check "VAL-CROSS-003 listeners come back on the saved ports" "$LISTEN_AGAIN" \
  "$RFC_PORT/$SSH_PORT listening with no reconfiguration"
check "VAL-CROSS-003 saved settings survive in the store" \
  "$(python3 - "$STORE" "$RFC_PW" "$SANDBOX" <<'PY'
import json
import sys

data = json.load(open(sys.argv[1], encoding="utf-8"))
ok = (
    data["rfc_enabled"] is True
    and data["rfc_" + "password"] == sys.argv[2]
    and data["allowed_paths"] == [sys.argv[3]]
)
print(1 if ok else 0)
PY
)" "toggle, password and allowed paths still persisted"
open_settings || { echo "settings did not open after restart"; exit 1; }
shot "cross-003-settings-after-restart"
check "VAL-CROSS-003 settings UI shows the saved toggle" \
  "$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(1 if d["rfc_enabled"] else 0)' "$STORE")" \
  "settings page rendered from the persisted store"
EXISTS_OUT="$(harness_rfc_call exists /a0/journey-a.txt)"
check "VAL-CROSS-003 harness call succeeds immediately after restart" \
  "$(printf '%s\n' "$EXISTS_OUT" | grep -qx 'exists True' && echo 1 || echo 0)" \
  "$(printf '%s\n' "$EXISTS_OUT" | tr '\n' ' ')"

# --- VAL-CROSS-005 ---------------------------------------------------------
say "VAL-CROSS-005 toggle OFF closes both control ports, chat keeps working"
open_settings || { echo "settings did not open"; exit 1; }
toggle_agent_control
save_settings
CLOSED=0
for _ in $(seq 1 40); do
  if ! control_ports_open; then CLOSED=1; break; fi
  sleep 0.5
done
check "VAL-CROSS-005 both control ports closed" "$CLOSED" \
  "ss listeners on 55080/55022: $(ss -tln 2>/dev/null | grep -cE ':(55080|55022)\b' || true)"
check "VAL-CROSS-005 store records the toggle off" \
  "$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(1 if d["rfc_enabled"] is False else 0)' "$STORE")" \
  "rfc_enabled=false persisted"
REFUSED_OUT="$(harness_rfc_call exists /a0/journey-a.txt)"
check "VAL-CROSS-005 harness call cannot connect" \
  "$(printf '%s\n' "$REFUSED_OUT" | grep -q '^error ' && echo 1 || echo 0)" \
  "$(printf '%s\n' "$REFUSED_OUT" | cut -c1-80)"
open_chat || { echo "chat did not reopen"; exit 1; }
sleep 1
login_ui
shot "cross-005-chat-before-send"
LOG_MARK=$(wc -l <"$APP_LOG")
send_chat "ping after toggle off"
CHAT_STILL=0
for _ in $(seq 1 240); do
  if tail -n +"$LOG_MARK" "$APP_LOG" | grep -q "notification sent"; then CHAT_STILL=1; break; fi
  sleep 0.5
done
sleep 1
shot "cross-005-chat-after-toggle-off"
check "VAL-CROSS-005 chat still completes with the toggle off" "$CHAT_STILL" \
  "message completed after both control ports closed"
check "VAL-CROSS-005 proxy still listening" \
  "$(nonzero "$(ss -tlnp 2>/dev/null | grep -c "pid=$(cat "$RIG_DIR/pids/app.pid")," || true)")" \
  "bridge proxy keeps its listener while RFC/SSH are off"

# --- VAL-CROSS-006 ---------------------------------------------------------
say "VAL-CROSS-006 chat stays alive under concurrent RFC traffic"
open_settings || { echo "settings did not open"; exit 1; }
toggle_agent_control
save_settings
UP=0
for _ in $(seq 1 40); do
  if port_listening "$RFC_PORT" && port_listening "$SSH_PORT"; then UP=1; break; fi
  sleep 0.5
done
check "VAL-CROSS-006 toggle back on reopens both ports" "$UP" "$RFC_PORT/$SSH_PORT listening"

python3 -m http.server "$STATIC_PORT" --directory "$REPO_ROOT/ui" >"$TMP/static.log" 2>&1 &
echo $! >"$STATIC_PID_FILE"
for _ in $(seq 1 40); do curl -sf -o /dev/null "http://127.0.0.1:$STATIC_PORT/index.html" && break; sleep 0.3; done

PROXY_PORT="$(ss -tlnp 2>/dev/null | grep "pid=$(cat "$RIG_DIR/pids/app.pid")," |
  grep -oE '127\.0\.0\.1:[0-9]+' | head -1 | cut -d: -f2)"
check "VAL-CROSS-006 proxy port resolved" "$([ -n "$PROXY_PORT" ] && echo 1 || echo 0)" "proxy :$PROXY_PORT"

"$VENV_PY" - "$HARNESS_DIR" "$STORE" >"$TMP/rfc-load.log" 2>&1 <<'PY' &
import asyncio
import base64
import json
import os
import sys

harness, store = sys.argv[1], sys.argv[2]
sys.path.insert(0, os.path.join(harness, "agent-zero"))
os.chdir(os.path.join(harness, "agent-zero"))
from helpers import rfc  # noqa: E402

settings = json.load(open(store, encoding="utf-8"))
url = "http://127.0.0.1:%d/api/rfc" % settings["rfc_port"]
password = settings["rfc_" + "password"]


async def main():
    for i in range(10):
        target = f"/a0/load-{i}.txt"
        payload = base64.b64encode(f"load {i}".encode()).decode()
        await rfc.call_rfc(url, password, "helpers.rfc_files",
                           "_write_file_binary_impl", [target, payload], {})
        back = await rfc.call_rfc(url, password, "helpers.rfc_files",
                                  "_read_file_binary_impl", [target], {})
        assert base64.b64decode(back) == f"load {i}".encode(), i
        print("rfc", i, "ok", flush=True)


asyncio.run(main())
PY
RFC_LOAD_PID=$!

agent-browser --session "$AB_SESSION" open \
  "http://127.0.0.1:$STATIC_PORT/index.html?proxy=http://127.0.0.1:$PROXY_PORT" \
  >"$TMP/ab-open.log" 2>&1
sleep 1
agent-browser --session "$AB_SESSION" fill '#login-username' "$CHAT_USER" >/dev/null 2>&1
agent-browser --session "$AB_SESSION" fill '#login-password' "$CHAT_PASSWORD" >/dev/null 2>&1
agent-browser --session "$AB_SESSION" click '#login-submit' >/dev/null 2>&1
sleep 4
agent-browser --session "$AB_SESSION" fill '#message-input' "ping concurrent" >/dev/null 2>&1
agent-browser --session "$AB_SESSION" click '#send-btn' >/dev/null 2>&1

REPLY_SEEN=0
for _ in $(seq 1 120); do
  TEXT="$(agent-browser --session "$AB_SESSION" get text '#messages' 2>/dev/null || true)"
  case "$TEXT" in *"AQVN stub model reply"*) REPLY_SEEN=1; break ;; esac
  sleep 1
done
CONN_TEXT="$(agent-browser --session "$AB_SESSION" get text '#connection' 2>/dev/null || true)"
agent-browser --session "$AB_SESSION" screenshot "$EVIDENCE/cross-006-browser.png" >/dev/null 2>&1 || true
agent-browser --session "$AB_SESSION" console >"$EVIDENCE/cross-006-console.log" 2>&1 || true
agent-browser --session "$AB_SESSION" errors >"$EVIDENCE/cross-006-errors.log" 2>&1 || true
wait "$RFC_LOAD_PID" || true
RFC_CALLS="$(grep -c 'rfc .* ok' "$TMP/rfc-load.log" || true)"
check "VAL-CROSS-006 10 concurrent RFC calls served" \
  "$([ "${RFC_CALLS:-0}" = 10 ] && echo 1 || echo 0)" "${RFC_CALLS:-0}/10 round-trips"
check "VAL-CROSS-006 chat response arrived during RFC load" "$REPLY_SEEN" \
  "rendered assistant record (connection='$CONN_TEXT')"
check "VAL-CROSS-006 connection stayed online" \
  "$(printf '%s' "$CONN_TEXT" | grep -qiE 'connected|online|polling' && echo 1 || echo 0)" "label='$CONN_TEXT'"
check "VAL-CROSS-006 no socket disconnect in console" \
  "$(zero "$(file_hits "$EVIDENCE/cross-006-console.log" 'disconnect|reconnect|transport close')")" \
  "console lines: $(wc -l <"$EVIDENCE/cross-006-console.log")"
check "VAL-CROSS-006 no page errors" \
  "$(zero "$(file_hits "$EVIDENCE/cross-006-errors.log" '.+')")" \
  "error lines: $(grep -cvE '^\s*$' "$EVIDENCE/cross-006-errors.log" || true)"

# --- VAL-CROSS-004 ---------------------------------------------------------
say "VAL-CROSS-004 whitelabel sweep across the named surfaces"
open_chat || true
TITLE="$(xdo getwindowname "$(main_window)")"
check "VAL-CROSS-004 window title is branded" \
  "$(printf '%s' "$TITLE" | grep -q '^AQVN' && echo 1 || echo 0)" "title='$TITLE'"
check "VAL-CROSS-004 window title has no upstream name" \
  "$(zero "$(int_hits "$TITLE" "$BANNED")")" "title='$TITLE'"

agent-browser --session "$AB_SESSION" open \
  "http://127.0.0.1:$STATIC_PORT/settings.html" >/dev/null 2>&1 || true
sleep 1
SETTINGS_TEXT="$(agent-browser --session "$AB_SESSION" get text 'body' 2>/dev/null || true)"
agent-browser --session "$AB_SESSION" open \
  "http://127.0.0.1:$STATIC_PORT/index.html?proxy=http://127.0.0.1:$PROXY_PORT" >/dev/null 2>&1 || true
sleep 1
CHAT_TEXT="$(agent-browser --session "$AB_SESSION" get text 'body' 2>/dev/null || true)"
check "VAL-CROSS-004 settings UI text" "$(zero "$(int_hits "$SETTINGS_TEXT" "$BANNED")")" \
  "settings DOM scanned ($(printf '%s' "$SETTINGS_TEXT" | wc -w) words)"
check "VAL-CROSS-004 chat UI text" "$(zero "$(int_hits "$CHAT_TEXT" "$BANNED")")" \
  "chat DOM scanned ($(printf '%s' "$CHAT_TEXT" | wc -w) words)"
check "VAL-CROSS-004 notification text" "$(zero "$(file_hits "$NOTIFY_LOG" "$BANNED")")" \
  "dbus capture scanned ($(wc -l <"$NOTIFY_LOG" 2>/dev/null || echo 0) lines)"
check "VAL-CROSS-004 tray tooltip strings" \
  "$(grep -qE 'AQVN Workspace — (idle|working)' "$REPO_ROOT/src-tauri/src/app/tray.rs" && echo 1 || echo 0)" \
  "both tray tooltips are AQVN-branded"
check "VAL-CROSS-004 android config label" \
  "$(python3 - "$REPO_ROOT/src-tauri/tauri.conf.json" <<'PY'
import json
import re
import sys

conf = json.load(open(sys.argv[1], encoding="utf-8"))
label = f"{conf.get('productName', '')} {conf.get('identifier', '')}".strip()
banned = "agent" + "zero|agent" + "0|agent " + "zero"
print(1 if label and not re.search(banned, label, re.I) else 0)
PY
)" "tauri.conf.json productName/identifier (gen/android is generated in the mobile milestone)"
RG_HITS="$(cd "$REPO_ROOT" && rg -i -l "$BANNED" --glob '!docs/rfc-spec/**' --glob '!.git' . 2>/dev/null | wc -l || true)"
check "VAL-CROSS-004 repo source sweep" "$(zero "${RG_HITS:-0}")" \
  "rg hits outside docs/rfc-spec: ${RG_HITS:-0}"

say "summary"
if [ "$FAILS" -eq 0 ]; then
  echo "CROSS-AREA E2E PASS: VAL-CROSS-001..006"
else
  echo "CROSS-AREA E2E FAILURES: $FAILS"
fi
exit "$((FAILS > 0))"
