#!/usr/bin/env bash
# Shared lifecycle for the E2E subject app: isolated settings, private Xvfb
# display, and our own binary. Used by spec_client.py and a0_e2e.sh so both
# drive the same deterministic configuration.
#
# Subcommands: start | stop | status
# Everything lives under $AQVN_E2E_STATE_DIR (default /tmp/aqvn-e2e-<uid>);
# pidfiles are the only processes this script is allowed to kill.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STATE_DIR="${AQVN_E2E_STATE_DIR:-/tmp/aqvn-e2e-$(id -u)}"
SANDBOX="${AQVN_E2E_SANDBOX:-$STATE_DIR/sandbox}"
CONFIG_HOME="$STATE_DIR/config"
RFC_PORT="${AQVN_E2E_RFC_PORT:-55080}"
SSH_PORT="${AQVN_E2E_SSH_PORT:-55022}"
RFC_PASSWORD="${AQVN_E2E_RFC_PASSWORD:-rfc-e2e-test-password}"
SSH_PASSWORD="${AQVN_E2E_SSH_PASSWORD:-ssh-e2e-test-password}"
APP_PID_FILE="$STATE_DIR/app.pid"
XVFB_PID_FILE="$STATE_DIR/xvfb.pid"
DISPLAY_FILE="$STATE_DIR/display"
APP_LOG="$STATE_DIR/app.log"
XVFB_LOG="$STATE_DIR/xvfb.log"

port_listening() { (exec 3<>"/dev/tcp/127.0.0.1/$1") >/dev/null 2>&1; }

wait_port() { # port timeout_seconds
  local i
  for ((i = 0; i < ${2:-60}; i++)); do
    port_listening "$1" && return 0
    sleep 1
  done
  return 1
}

wait_port_closed() { # port timeout_seconds
  local i
  for ((i = 0; i < ${2:-10}; i++)); do
    port_listening "$1" || return 0
    sleep 1
  done
  return 1
}

pid_alive() { [ -n "${1:-}" ] && kill -0 "$1" 2>/dev/null; }

app_binary() {
  if [ -n "${AQVN_APP_BIN:-}" ]; then
    printf '%s\n' "$AQVN_APP_BIN"
    return 0
  fi
  local release="$REPO_ROOT/src-tauri/target/release/aqvn-workspace"
  local debug="$REPO_ROOT/src-tauri/target/debug/aqvn-workspace"
  if [ -x "$release" ] &&
    [ -z "$(find "$REPO_ROOT/src-tauri/src" "$REPO_ROOT/src-tauri/Cargo.toml" -newer "$release" -print -quit 2>/dev/null)" ]; then
    printf '%s\n' "$release"
    return 0
  fi
  # Debug build tracks the current tree more closely than a stale release.
  if [ -x "$debug" ]; then
    printf '%s\n' "$debug"
    return 0
  fi
  printf '%s\n' "$release"
}

pick_display() {
  local n
  for n in 97 98 99; do
    if [ ! -e "/tmp/.X11-unix/X$n" ]; then
      printf ':%s\n' "$n"
      return 0
    fi
  done
  return 1
}

write_settings() {
  mkdir -p "$CONFIG_HOME/net.aqvn.workspace" "$SANDBOX"
  python3 - "$CONFIG_HOME/net.aqvn.workspace/settings.json" \
    "$SANDBOX" "$RFC_PASSWORD" "$SSH_PASSWORD" "$RFC_PORT" "$SSH_PORT" <<'PY'
import json
import sys

path, sandbox, rfc_password, ssh_password, rfc_port, ssh_port = sys.argv[1:7]
settings = {
    "server_url": "https://a0.matmagoc.com",
    "rfc_enabled": True,
    "rfc_password": rfc_password,
    "ssh_password": ssh_password,
    "allowed_paths": [sandbox],
    "rfc_port": int(rfc_port),
    "ssh_port": int(ssh_port),
}
with open(path, "w", encoding="utf-8") as handle:
    json.dump(settings, handle, indent=2)
PY
}

start() {
  mkdir -p "$STATE_DIR" "$SANDBOX"

  if [ -f "$APP_PID_FILE" ] && pid_alive "$(cat "$APP_PID_FILE")"; then
    echo "subject app already running (pid $(cat "$APP_PID_FILE"))"
    return 0
  fi
  rm -f "$APP_PID_FILE"

  local port
  for port in "$RFC_PORT" "$SSH_PORT"; do
    if port_listening "$port"; then
      echo "port $port is already in use; refusing to reuse an unknown server" >&2
      return 1
    fi
  done

  local display=""
  if [ -n "${DISPLAY:-}" ] && [ -e "/tmp/.X11-unix/X${DISPLAY#:}" ]; then
    display="$DISPLAY"
    rm -f "$XVFB_PID_FILE"
  else
    display="$(pick_display)" || {
      echo "no free X display in :97-:99" >&2
      return 1
    }
    Xvfb "$display" -screen 0 1280x800x24 >"$XVFB_LOG" 2>&1 &
    echo "$!" >"$XVFB_PID_FILE"
    local i
    for ((i = 0; i < 10; i++)); do
      [ -e "/tmp/.X11-unix/X${display#:}" ] && break
      sleep 1
    done
    if [ ! -e "/tmp/.X11-unix/X${display#:}" ]; then
      echo "Xvfb failed to start on $display (log: $XVFB_LOG)" >&2
      return 1
    fi
  fi
  printf '%s\n' "$display" >"$DISPLAY_FILE"
  export DISPLAY="$display"

  write_settings

  local binary
  binary="$(app_binary)"
  if [ ! -x "$binary" ]; then
    echo "subject binary not found: $binary" >&2
    return 1
  fi

  XDG_CONFIG_HOME="$CONFIG_HOME" \
    DISPLAY="$display" \
    WEBKIT_DISABLE_DMABUF_RENDERER=1 \
    setsid "$binary" >"$APP_LOG" 2>&1 &
  echo "$!" >"$APP_PID_FILE"

  if ! wait_port "$RFC_PORT" 90 || ! wait_port "$SSH_PORT" 90; then
    echo "subject app did not open $RFC_PORT/$SSH_PORT; log: $APP_LOG" >&2
    tail -n 20 "$APP_LOG" >&2 || true
    stop || true
    return 1
  fi

  echo "subject app pid=$(cat "$APP_PID_FILE") display=$display sandbox=$SANDBOX rfc=$RFC_PORT ssh=$SSH_PORT"
}

stop() {
  if [ -f "$APP_PID_FILE" ]; then
    local pid
    pid="$(cat "$APP_PID_FILE")"
    if pid_alive "$pid"; then
      kill "$pid" 2>/dev/null || true
      local i
      for ((i = 0; i < 10; i++)); do
        pid_alive "$pid" || break
        sleep 1
      done
      pid_alive "$pid" && kill -9 "$pid" 2>/dev/null || true
      echo "stopped subject app pid=$pid"
    fi
    rm -f "$APP_PID_FILE"
  fi

  if [ -f "$XVFB_PID_FILE" ]; then
    local xpid
    xpid="$(cat "$XVFB_PID_FILE")"
    pid_alive "$xpid" && kill "$xpid" 2>/dev/null || true
    rm -f "$XVFB_PID_FILE"
  fi

  wait_port_closed "$RFC_PORT" 10 || echo "warning: port $RFC_PORT still open" >&2
  wait_port_closed "$SSH_PORT" 10 || echo "warning: port $SSH_PORT still open" >&2
  return 0
}

status() {
  local port
  for port in "$RFC_PORT" "$SSH_PORT"; do
    if port_listening "$port"; then
      echo "port $port LISTEN"
    else
      echo "port $port closed"
    fi
  done
}

case "${1:-}" in
  start) start ;;
  stop) stop ;;
  status) status ;;
  *)
    echo "usage: $0 {start|stop|status}" >&2
    exit 2
    ;;
esac