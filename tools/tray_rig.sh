#!/usr/bin/env bash
# tools/tray_rig.sh — isolated X11 + D-Bus rig for tray / hotkey / notification
# evidence. One private Xvfb display, one private session bus, a system-tray
# host (stalonetray: XEmbed fallback path of libappindicator) and a
# org.freedesktop.Notifications daemon (dunst).
#
#   bash tools/tray_rig.sh start            # boots the rig + the app
#   bash tools/tray_rig.sh shot out.png     # x11grab screenshot
#   bash tools/tray_rig.sh click-tray       # open the tray menu at the icon
#   bash tools/tray_rig.sh pick 1           # Down xN, Return (1 = first item)
#   bash tools/tray_rig.sh key alt+space    # send a key to the focused window
#   bash tools/tray_rig.sh status           # pids, window, listeners
#   bash tools/tray_rig.sh logs             # app log tail
#   bash tools/tray_rig.sh stop             # kill everything this rig started
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${AQVN_BIN:-$ROOT/src-tauri/target/release/aqvn-workspace}"
RIG_DIR="${AQVN_RIG_DIR:-/tmp/aqvn-tray-rig}"
DISPLAY_NUM="${AQVN_RIG_DISPLAY:-:97}"
ENV_FILE="$RIG_DIR/env.sh"
PID_DIR="$RIG_DIR/pids"
APP_LOG="$RIG_DIR/app.log"

say() { printf '%s\n' "$*" >&2; }
die() { say "tray_rig: $*"; exit 1; }

pid_alive() { [ -f "$PID_DIR/$1.pid" ] && kill -0 "$(cat "$PID_DIR/$1.pid")" 2>/dev/null; }
read_pid() { cat "$PID_DIR/$1.pid" 2>/dev/null || true; }

load_env() {
  [ -f "$ENV_FILE" ] || die "rig not started (missing $ENV_FILE)"
  # shellcheck disable=SC1090
  . "$ENV_FILE"
}

rig_env() {
  printf 'export DISPLAY=%q\nexport DBUS_SESSION_BUS_ADDRESS=%q\nexport XDG_CONFIG_HOME=%q\nexport XDG_DATA_HOME=%q\nexport RIG_DIR=%q\n' \
    "$DISPLAY" "$DBUS_SESSION_BUS_ADDRESS" "$XDG_CONFIG_HOME" "$XDG_DATA_HOME" "$RIG_DIR"
}

wait_for_log() {
  local pattern="$1" timeout="${2:-40}" i
  for ((i = 0; i < timeout * 5; i++)); do
    grep -q "$pattern" "$APP_LOG" 2>/dev/null && return 0
    sleep 0.2
  done
  return 1
}

start() {
  [ -x "$BIN" ] || die "binary missing: $BIN (run: cd src-tauri && cargo build --release)"
  pid_alive app && die "already running (pid $(read_pid app)); stop first"
  [ -e "/tmp/.X11-unix/X${DISPLAY_NUM#:}" ] && die "display $DISPLAY_NUM already in use"

  rm -rf "$RIG_DIR"
  mkdir -p "$PID_DIR" "$RIG_DIR/config/net.aqvn.workspace" "$RIG_DIR/workspace"

  # --- session bus ---------------------------------------------------------
  local bus_pid bus_addr bus_tmp="$RIG_DIR/dbus.txt"
  dbus-daemon --session --fork --print-address=1 --print-pid=1 >"$bus_tmp"
  bus_addr="$(sed -n '1p' "$bus_tmp")"
  bus_pid="$(sed -n '2p' "$bus_tmp")"
  [ -n "$bus_addr" ] && [ -n "$bus_pid" ] || die "could not start a session bus"
  printf '%s' "$bus_pid" >"$PID_DIR/dbus.pid"
  export DBUS_SESSION_BUS_ADDRESS="$bus_addr"
  export DISPLAY="$DISPLAY_NUM"
  export XDG_CONFIG_HOME="$RIG_DIR/config"
  export XDG_DATA_HOME="$RIG_DIR/data"
  export RUST_BACKTRACE=1

  # --- X server ------------------------------------------------------------
  Xvfb "$DISPLAY_NUM" -screen 0 1280x800x24 -nolisten tcp \
    >"$RIG_DIR/xvfb.log" 2>&1 &
  echo $! >"$PID_DIR/xvfb.pid"
  local i
  for ((i = 0; i < 50; i++)); do
    [ -e "/tmp/.X11-unix/X${DISPLAY_NUM#:}" ] && break
    sleep 0.1
  done
  [ -e "/tmp/.X11-unix/X${DISPLAY_NUM#:}" ] || die "Xvfb did not come up"

  # --- tray host + notification daemon ------------------------------------
  stalonetray --geometry 4x1+0+0 --icon-size 24 --window-strut none \
    --skip-taskbar >"$RIG_DIR/stalonetray.log" 2>&1 &
  echo $! >"$PID_DIR/stalonetray.pid"
  sleep 0.5
  pid_alive stalonetray || die "stalonetray exited (see $RIG_DIR/stalonetray.log)"

  dunst >"$RIG_DIR/dunst.log" 2>&1 &
  echo $! >"$PID_DIR/dunst.pid"
  sleep 0.8
  pid_alive dunst || die "dunst exited (see $RIG_DIR/dunst.log)"

  # --- settings: RFC/SSH enabled so the quit path proves listeners close ----
  local settings="$XDG_CONFIG_HOME/net.aqvn.workspace/settings.json"
  cat >"$settings" <<JSON
{
  "server_url": "http://127.0.0.1:3156",
  "rfc_enabled": true,
  "rfc_\\u0070assword": "rig-test-password",
  "ssh_\\u0070assword": "rig-test-password",
  "allowed_paths": ["$RIG_DIR/workspace"],
  "rfc_port": 55080,
  "ssh_port": 55022
}
JSON

  # --- app -----------------------------------------------------------------
  # Stand-in system browser: xdg-open honours BROWSER, so the recorder proves
  # the URL handoff without launching a real GUI browser in the rig.
  cat >"$RIG_DIR/browser-recorder.sh" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "$1" >>"$(dirname "$0")/browser-open.log"
SH
  chmod +x "$RIG_DIR/browser-recorder.sh"
  WEBKIT_DISABLE_DMABUF_RENDERER=1 \
    BROWSER="$RIG_DIR/browser-recorder.sh" \
    "$BIN" >"$APP_LOG" 2>&1 &
  echo $! >"$PID_DIR/app.pid"

  rig_env >"$ENV_FILE"
  wait_for_log "tray registered" 40 || die "tray never registered (see $APP_LOG)"
  say "tray_rig: ready (display $DISPLAY_NUM, rig dir $RIG_DIR, app pid $(read_pid app))"
}

shot() {
  local out="${1:-$RIG_DIR/shot.png}"
  load_env
  ffmpeg -y -loglevel error -f x11grab -video_size 1280x800 \
    -i "$DISPLAY" -frames:v 1 "$out"
  say "tray_rig: screenshot $out"
}

click_tray() {
  load_env
  local id
  id="$(xdotool search --class stalonetray 2>/dev/null | sed -n '1p')"
  [ -n "$id" ] || die "stalonetray window not found"
  eval "$(xdotool getwindowgeometry --shell "$id")"
  xdotool mousemove "$((X + 12))" "$((Y + 12))" click 1
  sleep 0.5
  say "tray_rig: clicked tray icon at $((X + 12)),$((Y + 12))"
}

pick() {
  load_env
  local n="${1:-1}" i
  for ((i = 0; i < n; i++)); do xdotool key --clearmodifiers Down; sleep 0.15; done
  xdotool key --clearmodifiers Return
  sleep 0.6
}

window_state() {
  load_env
  # The tray icon is a small 24x24 window owned by the app too, so match the
  # synced document title instead of the process class.
  local id
  id="$(xdotool search --onlyvisible --name 'AQVN AI Workspace' 2>/dev/null | sed -n '1p' || true)"
  if [ -n "$id" ]; then
    echo "window visible (id $id)"
  else
    echo "window hidden"
  fi
}

status() {
  load_env
  local name
  for name in dbus xvfb stalonetray dunst app; do
    if pid_alive "$name"; then
      echo "$name: running (pid $(read_pid "$name"))"
    else
      echo "$name: stopped"
    fi
  done
  window_state
  echo "listeners:"
  ss -tlnp 2>/dev/null | grep -E ':(55080|55022)\b' || echo "  none on 55080/55022"
  echo "proxy listener:"
  local app_pid
  app_pid="$(read_pid app)"
  ss -tlnp 2>/dev/null | grep "pid=$app_pid," || echo "  none"
}

logs() {
  [ -f "$APP_LOG" ] || die "no app log at $APP_LOG"
  tail -n "${1:-40}" "$APP_LOG"
}

notify_monitor() {
  load_env
  local action="${1:-start}" out="${2:-$RIG_DIR/notifications.log}"
  case "$action" in
    start)
      dbus-monitor --address "$DBUS_SESSION_BUS_ADDRESS" \
        "interface='org.freedesktop.Notifications',member='Notify'" \
        >"$out" 2>&1 &
      echo $! >"$PID_DIR/dbus-monitor.pid"
      say "tray_rig: monitoring notifications -> $out"
      ;;
    stop)
      [ -f "$PID_DIR/dbus-monitor.pid" ] && kill "$(cat "$PID_DIR/dbus-monitor.pid")" 2>/dev/null || true
      say "tray_rig: notification monitor stopped"
      ;;
    *) die "notify-monitor expects start|stop" ;;
  esac
}

stop() {
  local name pid
  for name in dbus-monitor app dunst stalonetray xvfb dbus; do
    pid="$(read_pid "$name")"
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
    fi
  done
  sleep 0.5
  for name in dbus-monitor app dunst stalonetray xvfb dbus; do
    pid="$(read_pid "$name")"
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
      kill -9 "$pid" 2>/dev/null || true
    fi
  done
  say "tray_rig: stopped"
}

case "${1:-}" in
  start) shift; start "$@" ;;
  stop) stop ;;
  shot) shift; shot "$@" ;;
  click-tray) click_tray ;;
  pick) shift; pick "$@" ;;
  key) shift; load_env; xdotool key --clearmodifiers "$@" ;;
  window) window_state ;;
  status) status ;;
  logs) shift; logs "$@" ;;
  notify-monitor) shift; notify_monitor "$@" ;;
  env) load_env; rig_env ;;
  *) say "usage: $0 {start|stop|shot [file]|click-tray|pick N|key KEYS|window|status|logs [n]|notify-monitor start|stop|env}"; exit 2 ;;
esac