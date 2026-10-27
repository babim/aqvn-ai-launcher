#!/usr/bin/env bash
# Deterministic chat E2E through the loopback bridge proxy against the local
# chat harness:
#   UI-shaped login -> CSRF -> Socket.IO /ws -> state_request ->
#   /api/message_async -> state_push records (client GUID merged) -> /api/stop.
#
# Harness lifecycle is owned by its own scripts; the subject app and Xvfb are
# owned by tools/_e2e_lib.sh. Nothing else on this host is touched.
#
# The local harness ships without a model, so no assistant record is emitted.
# A bounded OpenAI-compatible stub (tools/model_stub_server.py) is started on a
# loopback free port and the harness model presets are pointed at it for the
# duration of the run. Harness sources are never touched; the preset file is
# backed up and restored on exit.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
AQVN_E2E_SERVER_URL="${AQVN_E2E_SERVER_URL:-http://127.0.0.1:5000}"
export AQVN_E2E_SERVER_URL
# The harness resolves the "other" provider key from the environment; the stub
# ignores it. Preset files are not allowed to carry API keys (the harness strips
# them at startup), so the dummy value travels as an inherited env var.
export OTHER_API_KEY="${OTHER_API_KEY:-stub-key}"
HARNESS_DIR="${AQVN_E2E_HARNESS:-}"

if [ -z "$HARNESS_DIR" ]; then
  start_script="$(find "$HOME/Desktop/dev/dev.aqvn" -maxdepth 3 -name start_a0.sh -print -quit 2>/dev/null || true)"
  [ -n "$start_script" ] && HARNESS_DIR="$(dirname "$start_script")"
fi
HARNESS_PID_FILE="$HARNESS_DIR/a0-ui.pid"

# shellcheck source=tools/_e2e_lib.sh
source "$REPO_ROOT/tools/_e2e_lib.sh"

PRESETS_FILE="$HARNESS_DIR/agent-zero/usr/plugins/_model_config/presets.yaml"
PRESETS_BACKUP="$STATE_DIR/presets.yaml.bak"
MODEL_STUB_PID_FILE="$STATE_DIR/model-stub.pid"
MODEL_STUB_PORT_FILE="$STATE_DIR/model-stub.port"
MODEL_STUB_LOG="$STATE_DIR/model-stub.log"
MODEL_STUB_PORT=""
model_stub_started=0
presets_backed_up=0
# The promptinclude extension calls the harness's own development RFC endpoint
# at prompt-build time. Our RFC server only implements the file/text-editor
# surface, so the extension is disabled for the duration of the flow via the
# harness plugin toggle file, then restored.
PROMPTINCLUDE_DIR="$HARNESS_DIR/agent-zero/usr/plugins/_promptinclude"
PROMPTINCLUDE_TOGGLE_ON="$STATE_DIR/promptinclude.toggle-1"
PROMPTINCLUDE_TOGGLE_OFF="$STATE_DIR/promptinclude.toggle-0"
promptinclude_touched=0

fail() {
  echo "FAIL $*" >&2
  exit 1
}

app_started=0

stop_harness() {
  local pid
  pid="$(cat "$HARNESS_PID_FILE" 2>/dev/null || true)"

  if [ -n "$pid" ] && pid_alive "$pid"; then
    if ! pid_matches "$pid" "run_ui.py"; then
      echo "warning: pid $pid in $HARNESS_PID_FILE is not the harness; refusing to kill" >&2
      rm -f "$HARNESS_PID_FILE"
    else
      "$HARNESS_DIR/stop_a0.sh" >/dev/null 2>&1 || true
      if ! terminate_pid "$pid" 10; then
        echo "error: harness pid=$pid survived; keeping $HARNESS_PID_FILE" >&2
        printf '%s\n' "$pid" >"$HARNESS_PID_FILE"
        return 1
      fi
      echo "stopped harness pid=$pid"
      rm -f "$HARNESS_PID_FILE"
    fi
  else
    rm -f "$HARNESS_PID_FILE"
  fi

  wait_port_closed 5000 10 || {
    echo "error: port 5000 still open after harness stop" >&2
    return 1
  }
  return 0
}

harness_rfc_password() {
  python3 - "$HARNESS_DIR/agent-zero/usr/.env" <<'PY'
import sys

key = "RFC" + "_PASSWORD"
try:
    lines = open(sys.argv[1], encoding="utf-8", errors="ignore").read().splitlines()
except OSError:
    sys.exit(0)
for line in lines:
    line = line.strip()
    if not line or line.startswith("#") or "=" not in line:
        continue
    name, value = line.split("=", 1)
    if name.strip() == key:
        sys.stdout.write(value.strip().strip('"').strip("'"))
        break
PY
}

disable_promptinclude() {
  mkdir -p "$PROMPTINCLUDE_DIR"
  rm -f "$PROMPTINCLUDE_TOGGLE_ON" "$PROMPTINCLUDE_TOGGLE_OFF"
  [ -f "$PROMPTINCLUDE_DIR/.toggle-1" ] && cp -p "$PROMPTINCLUDE_DIR/.toggle-1" "$PROMPTINCLUDE_TOGGLE_ON"
  [ -f "$PROMPTINCLUDE_DIR/.toggle-0" ] && cp -p "$PROMPTINCLUDE_DIR/.toggle-0" "$PROMPTINCLUDE_TOGGLE_OFF"
  promptinclude_touched=1
  : >"$PROMPTINCLUDE_DIR/.toggle-0"
}

restore_promptinclude() {
  [ "$promptinclude_touched" = 1 ] || return 0
  rm -f "$PROMPTINCLUDE_DIR/.toggle-0" "$PROMPTINCLUDE_DIR/.toggle-1"
  if [ -f "$PROMPTINCLUDE_TOGGLE_ON" ]; then
    cp -p "$PROMPTINCLUDE_TOGGLE_ON" "$PROMPTINCLUDE_DIR/.toggle-1" || return 1
  fi
  if [ -f "$PROMPTINCLUDE_TOGGLE_OFF" ]; then
    cp -p "$PROMPTINCLUDE_TOGGLE_OFF" "$PROMPTINCLUDE_DIR/.toggle-0" || return 1
  fi
  rm -f "$PROMPTINCLUDE_TOGGLE_ON" "$PROMPTINCLUDE_TOGGLE_OFF"
  promptinclude_touched=0
  echo "restored harness plugin toggles"
  return 0
}

start_model_stub() {
  rm -f "$MODEL_STUB_PORT_FILE"
  python3 "$REPO_ROOT/tools/model_stub_server.py" \
    --port 0 --port-file "$MODEL_STUB_PORT_FILE" >"$MODEL_STUB_LOG" 2>&1 &
  local pid=$!
  printf '%s\n' "$pid" >"$MODEL_STUB_PID_FILE"

  local i
  for ((i = 0; i < 100; i++)); do
    [ -s "$MODEL_STUB_PORT_FILE" ] && break
    pid_alive "$pid" || break
    sleep 0.2
  done
  if [ ! -s "$MODEL_STUB_PORT_FILE" ]; then
    echo "error: model stub did not publish a port; log: $MODEL_STUB_LOG" >&2
    return 1
  fi
  MODEL_STUB_PORT="$(cat "$MODEL_STUB_PORT_FILE")"
  for ((i = 0; i < 100; i++)); do
    if curl -sf -o /dev/null "http://127.0.0.1:$MODEL_STUB_PORT/health"; then
      model_stub_started=1
      echo "model stub pid=$pid port=$MODEL_STUB_PORT"
      return 0
    fi
    pid_alive "$pid" || break
    sleep 0.2
  done
  echo "error: model stub health check failed; log: $MODEL_STUB_LOG" >&2
  return 1
}

write_model_config() { # port
  local port="$1"
  if [ "$presets_backed_up" = 0 ]; then
    if [ ! -f "$PRESETS_FILE" ]; then
      echo "error: harness presets not found: $PRESETS_FILE" >&2
      return 1
    fi
    cp -p "$PRESETS_FILE" "$PRESETS_BACKUP" || return 1
    presets_backed_up=1
  fi
  python3 - "$PRESETS_FILE" "$port" <<'PY'
import sys

path, port = sys.argv[1], sys.argv[2]
base = f"http://127.0.0.1:{port}/v1"
presets = [
    {
        "name": "Default",
        "chat": {
            "provider": "other",
            "name": "aqvn-stub-chat",
            "api_base": base,
            "ctx_length": 32000,
            "ctx_history": 0.7,
            "vision": False,
            "rl_requests": 0,
            "rl_input": 0,
            "rl_output": 0,
            "kwargs": {},
        },
        "utility": {
            "provider": "other",
            "name": "aqvn-stub-chat",
            "api_base": base,
            "ctx_length": 32000,
            "rl_requests": 0,
            "rl_input": 0,
            "rl_output": 0,
            "kwargs": {},
        },
        "embedding": {
            "provider": "other",
            "name": "aqvn-stub-embed",
            "api_base": base,
            "rl_requests": 0,
            "rl_input": 0,
            "rl_output": 0,
            "kwargs": {},
        },
    }
]

import json

with open(path, "w", encoding="utf-8") as handle:
    json.dump(presets, handle, indent=2)
    handle.write("\n")
PY
}

restore_model_config() {
  [ "$presets_backed_up" = 1 ] || return 0
  if [ ! -f "$PRESETS_BACKUP" ]; then
    presets_backed_up=0
    return 0
  fi
  if cp -p "$PRESETS_BACKUP" "$PRESETS_FILE"; then
    rm -f "$PRESETS_BACKUP"
    presets_backed_up=0
    echo "restored harness model presets"
    return 0
  fi
  echo "error: could not restore $PRESETS_FILE; backup kept at $PRESETS_BACKUP" >&2
  return 1
}

stop_model_stub() {
  local pid
  pid="$(cat "$MODEL_STUB_PID_FILE" 2>/dev/null || true)"
  if [ -n "$pid" ] && pid_alive "$pid"; then
    if ! pid_matches "$pid" "model_stub_server.py"; then
      echo "warning: pid $pid in $MODEL_STUB_PID_FILE is not the model stub; refusing to kill" >&2
      rm -f "$MODEL_STUB_PID_FILE"
    else
      terminate_pid "$pid" 10 || {
        echo "error: model stub pid=$pid survived; keeping $MODEL_STUB_PID_FILE" >&2
        return 1
      }
      echo "stopped model stub pid=$pid"
      rm -f "$MODEL_STUB_PID_FILE"
    fi
  else
    rm -f "$MODEL_STUB_PID_FILE"
  fi
  if [ -n "$MODEL_STUB_PORT" ]; then
    wait_port_closed "$MODEL_STUB_PORT" 10 || {
      echo "error: model stub port $MODEL_STUB_PORT still open" >&2
      return 1
    }
  fi
  rm -f "$MODEL_STUB_PORT_FILE"
  model_stub_started=0
  return 0
}

cleanup() {
  local status=$?
  if ! stop_harness; then
    echo "cleanup: harness did not stop cleanly" >&2
    if [ "$status" -eq 0 ]; then status=1; fi
  fi
  if ! restore_promptinclude; then
    echo "cleanup: harness plugin toggles were not restored" >&2
    if [ "$status" -eq 0 ]; then status=1; fi
  fi
  if ! restore_model_config; then
    echo "cleanup: harness model presets were not restored" >&2
    if [ "$status" -eq 0 ]; then status=1; fi
  fi
  if [ "$model_stub_started" = 1 ] || [ -f "$MODEL_STUB_PID_FILE" ]; then
    if ! stop_model_stub; then
      echo "cleanup: model stub did not stop cleanly" >&2
      if [ "$status" -eq 0 ]; then status=1; fi
    fi
  fi
  if [ "$app_started" = 1 ]; then
    if ! bash "$REPO_ROOT/tools/_e2e_lib.sh" stop; then
      echo "cleanup: subject app or Xvfb did not stop cleanly" >&2
      if [ "$status" -eq 0 ]; then status=1; fi
    fi
    app_started=0
  fi
  if [ "$status" -ne 0 ]; then
    echo "cleanup complete (exit $status)" >&2
  fi
  return "$status"
}
trap cleanup EXIT INT TERM

[ -n "$HARNESS_DIR" ] && [ -x "$HARNESS_DIR/start_a0.sh" ] ||
  fail "harness not found (set AQVN_E2E_HARNESS to the directory holding start_a0.sh)"

if port_listening 5000; then
  fail "port 5000 already in use; refusing to touch an unknown harness"
fi

if [ -z "${AQVN_E2E_RFC_PASSWORD:-}" ]; then
  harness_password="$(harness_rfc_password)"
  if [ -n "$harness_password" ]; then
    AQVN_E2E_RFC_PASSWORD="$harness_password"
    export AQVN_E2E_RFC_PASSWORD
    echo "== subject app RFC password aligned with harness config =="
  fi
fi

echo "== starting subject app (server_url=$AQVN_E2E_SERVER_URL) =="
bash "$REPO_ROOT/tools/_e2e_lib.sh" start
app_started=1

proxy_port=""
for _ in $(seq 1 30); do
  proxy_port="$(grep -oE 'Bridge proxy listening on http://127\.0\.0\.1:[0-9]+' "$APP_LOG" 2>/dev/null |
    tail -n 1 | grep -oE '[0-9]+$' || true)"
  [ -n "$proxy_port" ] && break
  sleep 1
done
[ -n "$proxy_port" ] || fail "proxy port not found in $APP_LOG"
echo "== proxy listener at http://127.0.0.1:$proxy_port =="

echo "== starting model stub =="
start_model_stub || fail "model stub did not become ready (log: $MODEL_STUB_LOG)"

echo "== pointing harness chat model at http://127.0.0.1:$MODEL_STUB_PORT/v1 =="
write_model_config "$MODEL_STUB_PORT" ||
  fail "could not write harness model presets: $PRESETS_FILE"

disable_promptinclude || fail "could not disable the harness promptinclude plugin"

echo "== starting harness =="
"$HARNESS_DIR/start_a0.sh"

code="000"
for ((i = 0; i < 180; i++)); do
  code="$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:5000/login 2>/dev/null || true)"
  if [ "$code" = "200" ] || [ "$code" = "302" ]; then
    break
  fi
  sleep 1
done
[ "$code" = "200" ] || [ "$code" = "302" ] ||
  fail "harness UI did not become ready (last HTTP $code; log: $HARNESS_DIR/a0-ui.log)"

echo "== running chat flow through the proxy =="
python3 "$REPO_ROOT/tools/chat_e2e.py" \
  --mode proxy \
  --base "http://127.0.0.1:$proxy_port" \
  --username admin \
  --password test-auth-2026

echo "== stopping harness =="
stop_harness || fail "harness did not stop cleanly (pidfile: $HARNESS_PID_FILE)"

restore_promptinclude || fail "harness plugin toggles were not restored"
restore_model_config || fail "harness model presets were not restored"

echo "== stopping model stub =="
stop_model_stub || fail "model stub did not stop cleanly"

echo "== stopping subject app =="
bash "$REPO_ROOT/tools/_e2e_lib.sh" stop || fail "subject app or Xvfb did not stop cleanly"
app_started=0

wait_port_closed "$proxy_port" 10 || fail "proxy port $proxy_port still open after app stop"
for port in "$RFC_PORT" "$SSH_PORT" 5000 "$MODEL_STUB_PORT"; do
  [ -n "$port" ] || continue
  wait_port_closed "$port" 10 || fail "port $port still open after cleanup"
done

echo "CHAT E2E PASS: harness + model stub + proxy stopped, ports free"