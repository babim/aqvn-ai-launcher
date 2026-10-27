#!/usr/bin/env bash
# Deterministic chat E2E through the loopback bridge proxy against the local
# chat harness:
#   UI-shaped login -> CSRF -> Socket.IO /ws -> state_request ->
#   /api/message_async -> state_push records (client GUID merged) -> /api/stop.
#
# Harness lifecycle is owned by its own scripts; the subject app and Xvfb are
# owned by tools/_e2e_lib.sh. Nothing else on this host is touched.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
AQVN_E2E_SERVER_URL="${AQVN_E2E_SERVER_URL:-http://127.0.0.1:5000}"
export AQVN_E2E_SERVER_URL
HARNESS_DIR="${AQVN_E2E_HARNESS:-}"

if [ -z "$HARNESS_DIR" ]; then
  start_script="$(find "$HOME/Desktop/dev/dev.aqvn" -maxdepth 3 -name start_a0.sh -print -quit 2>/dev/null || true)"
  [ -n "$start_script" ] && HARNESS_DIR="$(dirname "$start_script")"
fi
HARNESS_PID_FILE="$HARNESS_DIR/a0-ui.pid"

# shellcheck source=tools/_e2e_lib.sh
source "$REPO_ROOT/tools/_e2e_lib.sh"

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

cleanup() {
  local status=$?
  if ! stop_harness; then
    echo "cleanup: harness did not stop cleanly" >&2
    if [ "$status" -eq 0 ]; then status=1; fi
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

echo "== stopping subject app =="
bash "$REPO_ROOT/tools/_e2e_lib.sh" stop || fail "subject app or Xvfb did not stop cleanly"
app_started=0

wait_port_closed "$proxy_port" 10 || fail "proxy port $proxy_port still open after app stop"
for port in "$RFC_PORT" "$SSH_PORT" 5000; do
  wait_port_closed "$port" 10 || fail "port $port still open after cleanup"
done

echo "CHAT E2E PASS: harness + proxy stopped, ports free"