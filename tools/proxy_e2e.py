#!/usr/bin/env python3
"""Harness-backed proof for the local bridge proxy.

Runs from the harness venv (python-socketio + websockets installed) and drives
the proxy exactly like the webview does:

1. POST /login JSON ``{username,password}`` — the proxy performs the form login
   and owns the resulting session cookie.
2. GET /proxy/csrf — the token the Socket.IO namespace CONNECT must carry.
3. Connect Socket.IO through the proxy (namespace ``/ws``, websocket transport).
4. Emit ``state_request`` and require an ACK with ``handlerId == ws_webui``.
5. Stay idle for ``--idle-seconds`` (default 50, the 45 s ping interval plus
   margin), then repeat ``state_request`` to prove the session survived.

The script prints PASS/FAIL lines and exits non-zero on the first failure.
Credentials and tokens are never printed.
"""

from __future__ import annotations

import argparse
import asyncio
import json
import sys
import time
import urllib.error
import urllib.request

import socketio


def http_json(base: str, method: str, path: str, payload: dict | None = None):
    data = None
    headers = {}
    if payload is not None:
        data = json.dumps(payload).encode("utf-8")
        headers["Content-Type"] = "application/json"
    request = urllib.request.Request(base + path, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            return response.status, response.read()
    except urllib.error.HTTPError as error:
        return error.code, error.read()


def check(name: str, condition: bool, detail: str) -> None:
    if not condition:
        raise SystemExit(f"FAIL {name}: {detail}")
    print(f"PASS {name}: {detail}", flush=True)


async def state_request(sio: socketio.AsyncClient, correlation: str) -> None:
    payload = {
        "context": None,
        "log_from": 0,
        "notifications_from": 0,
        "timezone": "UTC",
        "collections_delta": True,
        "correlationId": correlation,
    }
    ack = await sio.call("state_request", payload, namespace="/ws", timeout=30)
    check(
        f"{correlation} ACK",
        isinstance(ack, dict) and bool(ack.get("results")),
        f"ack keys={sorted(ack) if isinstance(ack, dict) else type(ack).__name__}",
    )
    result = ack["results"][0]
    # Harness identifiers are module-qualified (`ws_webui.WsWebui`); the
    # contract accepts the `ws_webui` handler.
    handler_id = str(result.get("handlerId", ""))
    check(
        f"{correlation} handler",
        handler_id.startswith("ws_webui") and result.get("ok") is True,
        f"handlerId={handler_id!r} ok={result.get('ok')!r}",
    )


async def run(args: argparse.Namespace) -> int:
    base = args.proxy_base.rstrip("/")

    status, body = await asyncio.to_thread(
        http_json,
        base,
        "POST",
        "/login",
        {"username": args.username, "password": args.password},
    )
    check(
        "proxy login",
        status == 200 and json.loads(body).get("ok") is True,
        f"HTTP {status} body={body[:80]!r}",
    )

    status, body = await asyncio.to_thread(http_json, base, "GET", "/proxy/csrf")
    check("csrf endpoint", status == 200, f"HTTP {status}")
    csrf = json.loads(body)
    check(
        "csrf token",
        csrf.get("ok") is True and bool(csrf.get("token")),
        f"token_length={len(csrf.get('token', ''))}",
    )

    sio = socketio.AsyncClient(reconnection=False, logger=False, engineio_logger=False)
    await sio.connect(
        base,
        auth={"csrf_token": csrf["token"], "handlers": ["ws_webui"]},
        namespaces=["/ws"],
        transports=["websocket"],
        wait_timeout=30,
    )
    check("socket.io connect", sio.connected, "websocket transport through proxy")

    await state_request(sio, "proxy-e2e-initial")

    if args.idle_seconds > 0:
        print(
            f"INFO idling {args.idle_seconds:.0f}s to cross the 45s ping interval",
            flush=True,
        )
        started = time.monotonic()
        await asyncio.sleep(args.idle_seconds)
        elapsed = time.monotonic() - started
        check(
            "session alive after idle",
            sio.connected,
            f"still connected after {elapsed:.1f}s",
        )
        await state_request(sio, "proxy-e2e-after-idle")

    await sio.disconnect()
    check("socket.io disconnect", not sio.connected, "clean shutdown")

    print("PROXY E2E PASS", flush=True)
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--proxy-base", required=True, help="e.g. http://127.0.0.1:54321")
    parser.add_argument("--username", default="admin")
    parser.add_argument("--password", default="test-auth-2026")
    parser.add_argument("--idle-seconds", type=float, default=50.0)
    args = parser.parse_args()
    return asyncio.run(run(args))


if __name__ == "__main__":
    sys.exit(main())