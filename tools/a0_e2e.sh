#!/usr/bin/env bash
# End-to-end proof against the real local harness client:
#   1. harness helpers.rfc.call_rfc round-trip (read/write) against our :55080
#   2. root-password exchange (_provide_root_password) -> RSA-OAEP decrypt
#   3. paramiko SSH session on :55022 with the exchanged password -> `echo ok`
#
# The harness (outside this repo) provides start_a0.sh/stop_a0.sh and its venv.
# Never modified here; its pidfile is the only harness process killed.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STATE_DIR="${AQVN_E2E_STATE_DIR:-/tmp/aqvn-e2e-$(id -u)}"
SANDBOX="${AQVN_E2E_SANDBOX:-$STATE_DIR/sandbox}"
RFC_PORT="${AQVN_E2E_RFC_PORT:-55080}"
SSH_PORT="${AQVN_E2E_SSH_PORT:-55022}"
RFC_PASSWORD="${AQVN_E2E_RFC_PASSWORD:-rfc-e2e-test-password}"
SSH_PASSWORD="${AQVN_E2E_SSH_PASSWORD:-ssh-e2e-test-password}"
HARNESS_DIR="${AQVN_E2E_HARNESS:-}"

if [ -z "$HARNESS_DIR" ]; then
  start_script="$(find "$HOME/Desktop/dev/dev.aqvn" -maxdepth 3 -name start_a0.sh -print -quit 2>/dev/null || true)"
  [ -n "$start_script" ] && HARNESS_DIR="$(dirname "$start_script")"
fi
HARNESS_PID_FILE="$HARNESS_DIR/a0-ui.pid"

# Share the port/pid primitives with the subject-app lifecycle script.
# shellcheck source=tools/_e2e_lib.sh
source "$REPO_ROOT/tools/_e2e_lib.sh"

fail() {
  echo "FAIL $*" >&2
  exit 1
}

app_started=0

stop_harness() { # stops only the pidfile harness; keeps the pidfile on kill failure
  local pid
  pid="$(cat "$HARNESS_PID_FILE" 2>/dev/null || true)"

  if [ -n "$pid" ] && pid_alive "$pid"; then
    if ! pid_matches "$pid" "run_ui.py"; then
      echo "warning: pid $pid in $HARNESS_PID_FILE is not the harness; refusing to kill" >&2
      rm -f "$HARNESS_PID_FILE"
    else
      # stop_a0.sh drops its pidfile unconditionally, so verify the PID here.
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
  if [ -f "$APP_PID_FILE" ] || [ -f "$XVFB_PID_FILE" ] || [ -f "$HARNESS_PID_FILE" ]; then
    echo "cleanup: pidfiles remain, processes may still be running" >&2
    if [ "$status" -eq 0 ]; then status=1; fi
  fi
  if [ "$status" -ne 0 ]; then
    echo "cleanup complete (exit $status)" >&2
  fi
  return "$status"
}
trap cleanup EXIT INT TERM

[ -n "$HARNESS_DIR" ] && [ -x "$HARNESS_DIR/start_a0.sh" ] ||
  fail "harness not found (set AQVN_E2E_HARNESS to the directory holding start_a0.sh)"
[ -x "$HARNESS_DIR/venv/bin/python" ] || fail "harness venv python missing: $HARNESS_DIR/venv/bin/python"

if port_listening 5000; then
  fail "port 5000 already in use; refusing to touch an unknown harness"
fi

echo "== starting subject app =="
bash "$REPO_ROOT/tools/_e2e_lib.sh" start
app_started=1

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

echo "== running harness venv flows =="
AQVN_E2E_HARNESS="$HARNESS_DIR" \
  RFC_PORT="$RFC_PORT" SSH_PORT="$SSH_PORT" \
  RFC_PASSWORD="$RFC_PASSWORD" SSH_PASSWORD="$SSH_PASSWORD" SANDBOX="$SANDBOX" \
  "$HARNESS_DIR/venv/bin/python" - <<'PY'
import asyncio
import base64
import os
import sys

harness_dir = os.environ["AQVN_E2E_HARNESS"]
sys.path.insert(0, os.path.join(harness_dir, "agent-zero"))
os.chdir(os.path.join(harness_dir, "agent-zero"))

import paramiko
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric import rsa

from helpers import crypto, rfc

RFC_URL = f"http://127.0.0.1:{os.environ['RFC_PORT']}/api/rfc"
RFC_PASSWORD = os.environ["RFC_PASSWORD"]
SSH_PORT = int(os.environ["SSH_PORT"])
SSH_PASSWORD = os.environ["SSH_PASSWORD"]
SANDBOX = os.path.realpath(os.environ["SANDBOX"])
FIXTURE = b"a0-e2e fixture \x00\x01payload\n"
TARGET = "/a0/e2e/fixture.bin"


def check(name, condition, detail):
    if not condition:
        raise SystemExit(f"FAIL {name}: {detail}")
    print(f"PASS {name}: {detail}", flush=True)


async def rfc_roundtrip():
    encoded = base64.b64encode(FIXTURE).decode()
    written = await rfc.call_rfc(
        RFC_URL, RFC_PASSWORD, "helpers.rfc_files", "_write_file_binary_impl",
        [TARGET, encoded], {},
    )
    check("harness write_file", written is True, f"returned {written!r}")

    read_back = await rfc.call_rfc(
        RFC_URL, RFC_PASSWORD, "helpers.rfc_files", "_read_file_binary_impl",
        [TARGET], {},
    )
    check(
        "harness read_file",
        read_back == encoded and base64.b64decode(read_back) == FIXTURE,
        f"{len(base64.b64decode(read_back))} bytes match fixture",
    )

    exists = await rfc.call_rfc(
        RFC_URL, RFC_PASSWORD, "helpers.rfc_files", "_path_exists_impl",
        [TARGET], {},
    )
    check("harness path_exists", exists is True, f"returned {exists!r}")


def exchange_password():
    private_key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
    public_hex = (
        private_key.public_key()
        .public_bytes(
            encoding=serialization.Encoding.PEM,
            format=serialization.PublicFormat.SubjectPublicKeyInfo,
        )
        .hex()
    )

    async def call_exchange():
        return await rfc.call_rfc(
            RFC_URL, RFC_PASSWORD, "helpers.rfc_exchange", "_provide_root_password",
            [public_hex], {},
        )

    ciphertext_hex = asyncio.run(call_exchange())
    check(
        "root password exchange",
        isinstance(ciphertext_hex, str) and len(ciphertext_hex) > 0,
        f"hex ciphertext {len(ciphertext_hex)} chars",
    )
    decrypted = crypto.decrypt_data(ciphertext_hex, private_key)
    check(
        "RSA-OAEP decrypt",
        decrypted == SSH_PASSWORD,
        "decrypted bytes equal configured ssh_password",
    )
    return decrypted


def ssh_session(password):
    client = paramiko.SSHClient()
    client.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    client.connect(
        "127.0.0.1",
        port=SSH_PORT,
        username="root",
        password=password,
        look_for_keys=False,
        allow_agent=False,
        timeout=20,
    )
    try:
        stdin, stdout, stderr = client.exec_command("echo ok; pwd", timeout=20)
        output = stdout.read().decode(errors="replace")
        error = stderr.read().decode(errors="replace")
    finally:
        client.close()
    check("paramiko connect", True, f"authenticated on :{SSH_PORT} with exchanged password")
    check("remote command", "ok" in output, f"stdout={output.strip()!r} stderr={error.strip()!r}")

    seen = [line.strip() for line in output.replace("\r", "\n").splitlines()]
    cwd = next(
        (line for line in seen if line.startswith("/") and os.path.realpath(line) == SANDBOX),
        None,
    )
    check(
        "remote cwd",
        cwd is not None,
        f"pwd={cwd or '<missing>'} sandbox={SANDBOX}",
    )


def main():
    asyncio.run(rfc_roundtrip())
    ssh_session(exchange_password())
    print("E2E: harness RFC + SSH flows passed", flush=True)


main()
PY

echo "== stopping harness =="
stop_harness || fail "harness did not stop cleanly (pidfile: $HARNESS_PID_FILE)"

echo "== stopping subject app =="
bash "$REPO_ROOT/tools/_e2e_lib.sh" stop || fail "subject app or Xvfb did not stop cleanly"
app_started=0

for port in "$RFC_PORT" "$SSH_PORT" 5000; do
  wait_port_closed "$port" 10 || fail "port $port still open after cleanup"
done
for pidfile in "$APP_PID_FILE" "$XVFB_PID_FILE" "$HARNESS_PID_FILE"; do
  [ ! -f "$pidfile" ] || fail "stale pidfile after cleanup: $pidfile"
done

echo "E2E PASS: harness left stopped, ports free, no orphan processes"