#!/usr/bin/env bash
# Regression checks for the deterministic model stub used by the local chat E2E.
# Covers the OpenAI-compatible surface (plain + streaming + embeddings) and the
# chat_e2e.sh lifecycle wiring. Loopback only; every process is killed by PID.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/aqvn-model-stub-test.XXXXXX")"
PORT_FILE="$TMP_DIR/port"
LOG_FILE="$TMP_DIR/stub.log"
STUB_PID=""
STUB_PORT=""

cleanup() {
  if [ -n "$STUB_PID" ] && kill -0 "$STUB_PID" 2>/dev/null; then
    kill "$STUB_PID" 2>/dev/null || true
    for _ in $(seq 1 20); do
      kill -0 "$STUB_PID" 2>/dev/null || break
      sleep 0.1
    done
    kill -9 "$STUB_PID" 2>/dev/null || true
  fi
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

fail() {
  echo "FAIL $*" >&2
  exit 1
}

check() { # name condition
  [ "$2" = "0" ] || fail "$1"
  echo "PASS $1"
}

# --- chat_e2e.sh must own the stub lifecycle --------------------------------
check "chat_e2e.sh starts the model stub" \
  "$(rg -q 'model_stub_server\.py' "$REPO_ROOT/tools/chat_e2e.sh" && echo 0 || echo 1)"
check "chat_e2e.sh points harness presets at the stub" \
  "$(rg -q 'presets\.yaml' "$REPO_ROOT/tools/chat_e2e.sh" && echo 0 || echo 1)"
check "chat_e2e.sh restores the preset backup" \
  "$(rg -q 'presets\.yaml\.bak|PRESETS_BACKUP' "$REPO_ROOT/tools/chat_e2e.sh" && echo 0 || echo 1)"
check "chat_e2e.py selects the returned context before awaiting records" \
  "$(rg -q '"context": context_id' "$REPO_ROOT/tools/chat_e2e.py" && echo 0 || echo 1)"

# --- stub OpenAI surface ----------------------------------------------------
python3 "$REPO_ROOT/tools/model_stub_server.py" --port 0 --port-file "$PORT_FILE" \
  >"$LOG_FILE" 2>&1 &
STUB_PID=$!
for _ in $(seq 1 50); do
  [ -s "$PORT_FILE" ] && break
  sleep 0.1
done
[ -s "$PORT_FILE" ] || fail "model stub did not publish a port ($(tail -n 5 "$LOG_FILE" 2>/dev/null))"
STUB_PORT="$(cat "$PORT_FILE")"
BASE="http://127.0.0.1:$STUB_PORT"

python3 - "$BASE" <<'PY'
import json
import sys
import urllib.request

base = sys.argv[1]


def post(path, payload):
    request = urllib.request.Request(
        base + path,
        data=json.dumps(payload).encode(),
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(request, timeout=10) as response:
        return response.status, response.read()


def parse_reply(content):
    reply = json.loads(content)
    assert reply["tool_name"] == "response", reply
    return reply["tool_args"]["text"]


status, body = post(
    "/v1/chat/completions",
    {"model": "aqvn-stub-chat", "messages": [{"role": "user", "content": "ping"}]},
)
assert status == 200, status
payload = json.loads(body)
assert payload["object"] == "chat.completion", payload
text = parse_reply(payload["choices"][0]["message"]["content"])
assert text == "AQVN stub model reply: ping", text
print("PASS plain completion echoes the last user message")

request = urllib.request.Request(
    base + "/v1/chat/completions",
    data=json.dumps(
        {
            "model": "aqvn-stub-chat",
            "stream": True,
            "stream_options": {"include_usage": True},
            "messages": [{"role": "user", "content": "ping"}],
        }
    ).encode(),
    headers={"Content-Type": "application/json"},
)
with urllib.request.urlopen(request, timeout=10) as response:
    assert response.headers["Content-Type"].startswith("text/event-stream")
    lines = [line for line in response.read().decode().splitlines() if line.strip()]
assert lines[-1] == "data: [DONE]", lines[-1]
deltas = []
for line in lines:
    if not line.startswith("data: ") or line == "data: [DONE]":
        continue
    chunk = json.loads(line[6:])
    for choice in chunk.get("choices") or []:
        deltas.append((choice.get("delta") or {}).get("content") or "")
assert parse_reply("".join(deltas)) == "AQVN stub model reply: ping", deltas
print("PASS streaming SSE accumulates the same reply")

status, body = post("/v1/embeddings", {"model": "aqvn-stub-embed", "input": ["hello"]})
assert status == 200, status
vector = json.loads(body)["data"][0]["embedding"]
assert len(vector) == 384, len(vector)
assert all(isinstance(value, (int, float)) for value in vector)
print("PASS embeddings return a deterministic 384-dim vector")
PY

kill "$STUB_PID"
for _ in $(seq 1 50); do
  kill -0 "$STUB_PID" 2>/dev/null || break
  sleep 0.1
done
! kill -0 "$STUB_PID" 2>/dev/null || fail "model stub survived SIGTERM"
STUB_PID=""
if (exec 3<>"/dev/tcp/127.0.0.1/$STUB_PORT") 2>/dev/null; then
  fail "model stub port $STUB_PORT still open"
fi
echo "PASS model stub shut down and released its port"

echo "MODEL STUB TEST PASS"
