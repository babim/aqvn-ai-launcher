#!/usr/bin/env python3
"""Deterministic OpenAI-compatible model stub for the local chat harness.

The harness needs a chat model to emit an assistant record; this server is the
smallest endpoint that satisfies it. Loopback-only, stdlib-only, no state.

    POST /v1/chat/completions   plain chat.completion + SSE stream
    POST /v1/embeddings         deterministic vectors (chat harness memory)
    GET  /v1/models             model listing probe
    GET  /health                readiness probe

Every completion answers with one tool request for the harness `response`
tool. Its text is the deterministic echo:

    AQVN stub model reply: <last user message>

Usage:
    python3 tools/model_stub_server.py --port 0 --port-file /tmp/model-stub.port
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

REPLY_PREFIX = "AQVN stub model reply: "
CHAT_MODEL = "aqvn-stub-chat"
EMBED_MODEL = "aqvn-stub-embed"
EMBED_DIM = 384
MAX_ECHO = 500


def last_user_text(messages: object) -> str:
    if not isinstance(messages, list):
        return ""
    for message in reversed(messages):
        if not isinstance(message, dict) or message.get("role") != "user":
            continue
        content = message.get("content")
        if isinstance(content, str):
            return content
        if isinstance(content, list):
            parts = [
                part.get("text", "")
                for part in content
                if isinstance(part, dict) and part.get("type") == "text"
            ]
            return " ".join(part for part in parts if part)
    return ""


def reply_content(messages: object) -> str:
    echo = " ".join(last_user_text(messages).split())[:MAX_ECHO]
    if not echo:
        echo = "(no user message)"
    request = {
        "thoughts": "Deterministic stub model reply.",
        "tool_name": "response",
        "tool_args": {"text": REPLY_PREFIX + echo},
    }
    return json.dumps(request, ensure_ascii=False)


def embedding_for(text: str) -> list[float]:
    digest = b""
    seed = text.encode("utf-8", "replace")
    while len(digest) < EMBED_DIM:
        seed = hashlib.sha256(seed).digest()
        digest += seed
    return [round((byte / 127.5) - 1.0, 6) for byte in digest[:EMBED_DIM]]


def usage(messages: object, content: str) -> dict:
    prompt = len(json.dumps(messages, ensure_ascii=False)) // 4
    completion = max(1, len(content) // 4)
    return {
        "prompt_tokens": prompt,
        "completion_tokens": completion,
        "total_tokens": prompt + completion,
    }


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    server_version = "aqvn-model-stub/1.0"

    def log_message(self, fmt: str, *args) -> None:
        sys.stderr.write("model-stub %s\n" % (fmt % args))
        sys.stderr.flush()

    def _json(self, status: int, payload: dict) -> None:
        body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self) -> None:  # noqa: N802 - stdlib handler naming
        if self.path.startswith("/health"):
            self._json(200, {"ok": True, "service": "model-stub"})
        elif self.path.startswith("/v1/models"):
            self._json(
                200,
                {
                    "object": "list",
                    "data": [
                        {"id": CHAT_MODEL, "object": "model", "owned_by": "aqvn"},
                        {"id": EMBED_MODEL, "object": "model", "owned_by": "aqvn"},
                    ],
                },
            )
        else:
            self._json(404, {"error": {"message": f"no route: {self.path}"}})

    def do_POST(self) -> None:  # noqa: N802 - stdlib handler naming
        length = int(self.headers.get("Content-Length") or 0)
        try:
            request = json.loads(self.rfile.read(length) or b"{}")
        except ValueError:
            self._json(400, {"error": {"message": "invalid JSON body"}})
            return
        if not isinstance(request, dict):
            self._json(400, {"error": {"message": "JSON body must be an object"}})
            return
        if self.path.startswith("/v1/chat/completions"):
            self._chat(request)
        elif self.path.startswith("/v1/embeddings"):
            self._embeddings(request)
        else:
            self._json(404, {"error": {"message": f"no route: {self.path}"}})

    def _chat(self, request: dict) -> None:
        model = str(request.get("model") or CHAT_MODEL)
        messages = request.get("messages")
        content = reply_content(messages)
        completion_id = "chatcmpl-aqvn-stub"
        created = int(time.time())
        stats = usage(messages, content)

        if request.get("stream") is not True:
            self._json(
                200,
                {
                    "id": completion_id,
                    "object": "chat.completion",
                    "created": created,
                    "model": model,
                    "choices": [
                        {
                            "index": 0,
                            "message": {"role": "assistant", "content": content},
                            "finish_reason": "stop",
                        }
                    ],
                    "usage": stats,
                },
            )
            return

        include_usage = bool(
            (request.get("stream_options") or {}).get("include_usage")
        )
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        self.send_header("Connection", "close")
        self.end_headers()
        self.close_connection = True

        def emit(payload: object) -> None:
            self.wfile.write(
                b"data: " + json.dumps(payload, ensure_ascii=False).encode() + b"\n\n"
            )
            self.wfile.flush()

        base = {
            "id": completion_id,
            "object": "chat.completion.chunk",
            "created": created,
            "model": model,
        }
        emit(
            {
                **base,
                "choices": [
                    {"index": 0, "delta": {"role": "assistant"}, "finish_reason": None}
                ],
            }
        )
        for offset in range(0, len(content), 80):
            emit(
                {
                    **base,
                    "choices": [
                        {
                            "index": 0,
                            "delta": {"content": content[offset : offset + 80]},
                            "finish_reason": None,
                        }
                    ],
                }
            )
        emit(
            {
                **base,
                "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}],
            }
        )
        if include_usage:
            emit({**base, "choices": [], "usage": stats})
        self.wfile.write(b"data: [DONE]\n\n")
        self.wfile.flush()

    def _embeddings(self, request: dict) -> None:
        raw = request.get("input")
        items = raw if isinstance(raw, list) else [raw]
        data = []
        tokens = 0
        for index, item in enumerate(items):
            text = item if isinstance(item, str) else json.dumps(item, ensure_ascii=False)
            tokens += max(1, len(text) // 4)
            data.append(
                {"object": "embedding", "index": index, "embedding": embedding_for(text)}
            )
        self._json(
            200,
            {
                "object": "list",
                "model": str(request.get("model") or EMBED_MODEL),
                "data": data,
                "usage": {"prompt_tokens": tokens, "total_tokens": tokens},
            },
        )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=0)
    parser.add_argument("--port-file", default="")
    args = parser.parse_args()

    server = ThreadingHTTPServer((args.host, args.port), Handler)
    server.daemon_threads = True
    port = server.server_address[1]
    if args.port_file:
        tmp = f"{args.port_file}.{os.getpid()}"
        with open(tmp, "w", encoding="utf-8") as handle:
            handle.write(f"{port}\n")
        os.replace(tmp, args.port_file)
    print(f"MODEL STUB listening on http://{args.host}:{port}", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
