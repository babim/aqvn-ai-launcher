#!/usr/bin/env bash
# Regression tests for the E2E cleanup hardening (feature rfc-ssh-e2e-fix-1).
# Exercises tools/_e2e_lib.sh pid handling without starting the harness or the app.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LIB="$REPO_ROOT/tools/_e2e_lib.sh"
A0="$REPO_ROOT/tools/a0_e2e.sh"

TMP="$(mktemp -d)"
FOREIGN_PID=""
MATCHING_PID=""
STUBBORN_PID=""

cleanup() {
  local pid
  for pid in "$FOREIGN_PID" "$MATCHING_PID" "$STUBBORN_PID"; do
    if [ -n "$pid" ]; then
      kill -9 "$pid" 2>/dev/null || true
    fi
  done
  rm -rf "$TMP"
}
trap cleanup EXIT

fail() {
  echo "FAIL $*" >&2
  exit 1
}

pass() { echo "PASS $*"; }

alive() { kill -0 "$1" 2>/dev/null; }

write_pidfile() { # state_dir pid
  mkdir -p "$1"
  printf '%s\n' "$2" >"$1/app.pid"
}

# A pidfile left over from a recycled pid must never authorise a kill.
sleep 60 &
FOREIGN_PID=$!
write_pidfile "$TMP/foreign" "$FOREIGN_PID"
AQVN_E2E_STATE_DIR="$TMP/foreign" bash "$LIB" stop >/dev/null ||
  fail "stop returned nonzero for a foreign pid"
if alive "$FOREIGN_PID"; then
  pass "foreign pid left alive, stale pidfile removed"
else
  fail "stop killed a process that is not the subject app"
fi
if [ -f "$TMP/foreign/app.pid" ]; then
  fail "stale foreign pidfile was kept"
fi

# A pidfile naming our app is still stopped (provenance check must not block it).
bash -c 'exec -a aqvn-workspace sleep 60' &
MATCHING_PID=$!
write_pidfile "$TMP/matching" "$MATCHING_PID"
AQVN_E2E_STATE_DIR="$TMP/matching" bash "$LIB" stop >/dev/null ||
  fail "stop failed for a provenance-matched app pid"
if alive "$MATCHING_PID"; then
  fail "provenance-matched app process survived stop"
fi
if [ -f "$TMP/matching/app.pid" ]; then
  fail "pidfile kept after a successful stop"
fi
pass "provenance-matched app stopped, pidfile removed"

# A process that cannot be killed must keep its pidfile for the next attempt.
bash -c 'exec -a aqvn-workspace sleep 60' &
STUBBORN_PID=$!
write_pidfile "$TMP/stubborn" "$STUBBORN_PID"
set +e
AQVN_E2E_STATE_DIR="$TMP/stubborn" bash -c '
  source "$1"
  kill() { if [ "$1" = "-0" ]; then builtin kill -0 "$2"; else return 1; fi; }
  stop
' _ "$LIB" >/dev/null
rc=$?
set -e
if [ "$rc" -ne 1 ]; then
  fail "stop returned $rc for an unkillable app, expected 1"
fi
if [ ! -f "$TMP/stubborn/app.pid" ]; then
  fail "pidfile removed although the process survived"
fi
if ! alive "$STUBBORN_PID"; then
  fail "stubborn probe process vanished"
fi
pass "unkillable process keeps its pidfile and fails stop"

# Port 5000 must use the shared wait loop, not a single probe.
if ! grep -q 'wait_port_closed 5000' "$A0"; then
  fail "a0_e2e.sh does not wait for port 5000 to close"
fi
pass "a0_e2e.sh waits for port 5000 to close"

# Final cleanup must assert processes exited, not only that ports are free.
if ! grep -q 'stale pidfile' "$A0"; then
  fail "a0_e2e.sh final cleanup does not assert pidfiles are gone"
fi
pass "a0_e2e.sh final cleanup asserts processes exited"
