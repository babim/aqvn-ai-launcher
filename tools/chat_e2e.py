#!/usr/bin/env python3
"""Protocol-level chat E2E for the AQVN Workspace chat shell.

Two modes:

  proxy   drives the local bridge proxy exactly like the webview:
          POST /login (JSON) -> GET /proxy/csrf -> Socket.IO /ws (auth)
          -> state_request -> /api/message_async -> state_push -> /api/stop

  direct  drives the chat server without the proxy (real-server proof):
          POST /login (form) -> GET /api/csrf_token -> Socket.IO /ws
          (auth + cookies) -> state_request -> /api/message_async
          -> state_push -> /api/stop

Credentials are read from flags, then from the repo `.env`
(A0_USERNAME / A0_PASSWORD), then from the local-harness defaults.
Secrets are never printed.

Exit codes: 0 = PASS, 1 = FAIL, 3 = BLOCKED (unreachable / no credentials).

    python3 tools/chat_e2e.py --mode proxy --base http://127.0.0.1:54321
    python3 tools/chat_e2e.py --mode direct --base https://a0.matmagoc.com
"""

from __future__ import annotations

import argparse
import asyncio
import json
import os
import sys
import time
import uuid
from pathlib import Path

import requests
import socketio

NAMESPACE = "/ws"
HARNESS_USER = "admin"
HARNESS_PASSWORD = "test-auth-2026"


class Blocked(Exception):
    """Environment cannot run the check (unreachable host or missing creds)."""


def load_dotenv(path: Path) -> None:
    if not path.is_file():
        return
    for line in path.read_text(encoding="utf-8", errors="ignore").splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        key = key.strip()
        value = value.strip().strip('"').strip("'")
        if key and value and key not in os.environ:
            os.environ[key] = value


def check(name: str, condition: bool, detail: str) -> None:
    if not condition:
        raise SystemExit(f"FAIL {name}: {detail}")
    print(f"PASS {name}: {detail}", flush=True)


def http_json(method: str, url: str, *, payload=None, data=None, headers=None, session=None):
    agent = session or requests
    timeout = float(os.environ.get("CHAT_E2E_HTTP_TIMEOUT", "120"))
    try:
        return agent.request(
            method, url, json=payload, data=data, headers=headers, timeout=timeout
        )
    except requests.exceptions.RequestException as err:
        raise Blocked(f"{method} {url} unreachable: {type(err).__name__}") from err


def parse_json(response, label: str) -> dict:
    try:
        return response.json()
    except ValueError as err:
        raise SystemExit(
            f"FAIL {label}: non-JSON response (HTTP {response.status_code})"
        ) from err


async def wait_for_records(pushes: list[dict], message_id: str, timeout: float):
    deadline = time.monotonic() + timeout
    saw_progress = False
    while time.monotonic() < deadline:
        for push in list(pushes):
            snapshot = (push.get("data") or {}).get("snapshot") or {}
            if snapshot.get("log_progress_active") is True:
                saw_progress = True
            logs = snapshot.get("logs") or []
            user = any(
                rec.get("id") == message_id and rec.get("type") == "user"
                for rec in logs
            )
            answered = any(
                rec.get("type") in {"agent", "response"} for rec in logs
            )
            if user and answered:
                return saw_progress
        await asyncio.sleep(0.1)
    return None


async def poll_for_records(
    mode: str,
    base: str,
    csrf: dict,
    cookies: str,
    context_id: str,
    message_id: str,
    timeout: float,
):
    """Same record check through /api/poll (WS handlers unavailable)."""
    deadline = time.monotonic() + timeout
    saw_progress = False
    headers = {"X-CSRF-Token": csrf["token"]}
    if mode == "direct" and cookies:
        headers["Cookie"] = cookies
    while time.monotonic() < deadline:
        response = await asyncio.to_thread(
            http_json,
            "POST",
            base + "/api/poll",
            payload={
                "context": context_id,
                "log_from": 0,
                "notifications_from": 0,
                "timezone": "UTC",
            },
            headers=headers,
        )
        if response.status_code == 200:
            try:
                snapshot = response.json()
            except ValueError:
                snapshot = {}
            if snapshot.get("log_progress_active") is True:
                saw_progress = True
            logs = snapshot.get("logs") or []
            user = any(
                rec.get("id") == message_id and rec.get("type") == "user"
                for rec in logs
            )
            answered = any(rec.get("type") in {"agent", "response"} for rec in logs)
            if user and answered:
                return saw_progress
        await asyncio.sleep(0.6)
    return None


async def run_socket(
    mode: str, base: str, csrf: dict, cookies: str, allow_poll_fallback: bool
) -> bool:
    handlers = [
        item for item in os.environ.get("CHAT_E2E_HANDLERS", "ws_webui").split(",") if item
    ]
    transports = [
        item
        for item in os.environ.get("CHAT_E2E_TRANSPORTS", "websocket").split(",")
        if item
    ]
    auth = {"csrf_token": csrf["token"], "handlers": handlers}
    headers = {}
    if mode == "direct":
        # The server validates Origin on the WebSocket handshake; a browser
        # sends it automatically, a script must provide it.
        headers["Origin"] = base
        if cookies:
            headers["Cookie"] = cookies

    sio = socketio.AsyncClient(
        reconnection=False, logger=False, engineio_logger=False
    )
    pushes: list[dict] = []

    @sio.on("state_push", namespace=NAMESPACE)
    async def on_push(data):
        pushes.append(data if isinstance(data, dict) else {"data": {}})

    try:
        await sio.connect(
            base,
            auth=auth,
            headers=headers or None,
            namespaces=[NAMESPACE],
            transports=transports,
            wait_timeout=30,
        )
    except (socketio.exceptions.ConnectionError, OSError) as err:
        message = str(err)
        if "unreachable" in message.lower() or "timed out" in message.lower():
            raise Blocked(f"socket.io connect failed: {type(err).__name__}") from err
        raise SystemExit(f"FAIL socket.io connect: {message[:200]}") from err

    check("socket.io connect", sio.connected, "namespace /ws connected")

    correlation = "chat-e2e-" + uuid.uuid4().hex[:8]
    state_payload = {
        "context": None,
        "log_from": 0,
        "notifications_from": 0,
        "timezone": "UTC",
        "collections_delta": True,
        "correlationId": correlation,
    }
    ack = await sio.call("state_request", state_payload, namespace=NAMESPACE, timeout=30)
    results = (ack or {}).get("results") or []
    check("state_request ACK", bool(results), f"results={len(results)}")
    first = results[0]
    handler_id = str(first.get("handlerId", ""))
    ws_ok = handler_id.startswith("ws_webui") and first.get("ok") is True
    if ws_ok:
        check("state_request handler", True, f"handlerId={handler_id!r} ok=True")
    else:
        print(f"INFO state_request result: {json.dumps(first)[:400]}", flush=True)
        if not allow_poll_fallback:
            check(
                "state_request handler",
                False,
                f"handlerId={handler_id!r} ok={first.get('ok')!r}",
            )
        print("INFO ws handlers unavailable; falling back to /api/poll", flush=True)

    message_id = str(uuid.uuid4())
    payload = {"text": "ping", "context": "", "message_id": message_id}
    if mode == "proxy":
        response = await asyncio.to_thread(
            http_json,
            "POST",
            base + "/api/message_async",
            payload=payload,
            headers={"X-CSRF-Token": csrf["token"]},
        )
    else:
        response = await asyncio.to_thread(
            http_json,
            "POST",
            base + "/api/message_async",
            payload=payload,
            headers={"X-CSRF-Token": csrf["token"], "Cookie": cookies},
        )
    check(
        "message_async",
        response.status_code == 200,
        f"HTTP {response.status_code}",
    )
    sent = parse_json(response, "message_async")
    context_id = str(sent.get("context") or "")
    check("message_async context", bool(context_id), "context id returned")

    timeout = float(os.environ.get("CHAT_E2E_TIMEOUT", "180"))
    if ws_ok:
        saw_progress = await wait_for_records(pushes, message_id, timeout)
        record_label = "state_push records"
    else:
        saw_progress = await poll_for_records(
            mode, base, csrf, cookies, context_id, message_id, timeout
        )
        record_label = "poll records"
    check(
        record_label,
        saw_progress is not None,
        "user record with client GUID + assistant record observed",
    )
    print(
        f"INFO log_progress_active observed: {'yes' if saw_progress else 'no'}",
        flush=True,
    )

    stop_payload = {"context": context_id}
    stop_headers = {"X-CSRF-Token": csrf["token"]}
    if mode == "direct":
        stop_headers["Cookie"] = cookies
    stop_response = await asyncio.to_thread(
        http_json,
        "POST",
        base + "/api/stop",
        payload=stop_payload,
        headers=stop_headers,
    )
    check("stop", stop_response.status_code == 200, f"HTTP {stop_response.status_code}")
    stop_body = parse_json(stop_response, "stop")
    check("stop payload", "stopped" in stop_body, "stopped field present")

    await sio.disconnect()
    check("socket.io disconnect", not sio.connected, "clean shutdown")
    await sio.shutdown()
    return ws_ok


async def run(args: argparse.Namespace) -> int:
    base = args.base.rstrip("/")
    load_dotenv(Path(args.env_file))

    username = args.username or os.environ.get("A0_USERNAME") or ""
    password = args.password or os.environ.get("A0_PASSWORD") or ""
    if not username or not password:
        if args.mode == "direct" and "matmagoc" not in base:
            username, password = HARNESS_USER, HARNESS_PASSWORD
        else:
            raise Blocked("A0_USERNAME/A0_PASSWORD missing (repo .env)")
    if args.mode == "proxy" and (not username or not password):
        raise Blocked("proxy login credentials missing")

    print(f"INFO mode={args.mode} base={base} user_present={bool(username)}", flush=True)

    if args.mode == "proxy":
        response = await asyncio.to_thread(
            http_json,
            "POST",
            base + "/login",
            payload={"username": username, "password": password},
        )
        check("login", response.status_code == 200, f"HTTP {response.status_code}")
        login = parse_json(response, "login")
        check("login payload", login.get("ok") is True, "ok=true")

        response = await asyncio.to_thread(http_json, "GET", base + "/proxy/csrf")
        check("csrf endpoint", response.status_code == 200, f"HTTP {response.status_code}")
        csrf = parse_json(response, "csrf")
        check("csrf token", bool(csrf.get("token")), "token present")
        cookies = ""
    else:
        session = requests.Session()
        response = await asyncio.to_thread(
            http_json,
            "POST",
            base + "/login",
            data={"username": username, "password": password},
            session=session,
        )
        check("login", response.status_code in (200, 302), f"HTTP {response.status_code}")
        check(
            "session cookie",
            bool(session.cookies),
            "session cookie received",
        )

        response = await asyncio.to_thread(
            http_json, "GET", base + "/api/csrf_token", session=session
        )
        check("csrf endpoint", response.status_code == 200, f"HTTP {response.status_code}")
        csrf = parse_json(response, "csrf")
        check("csrf token", bool(csrf.get("token")), "token present")
        cookies = "; ".join(f"{c.name}={c.value}" for c in session.cookies)

    ws_ok = await run_socket(
        args.mode, base, csrf, cookies, args.allow_poll_fallback
    )
    if not ws_ok:
        print(
            "BLOCKED ws state handler unavailable on this server; "
            "poll fallback verified the rest of the flow",
            flush=True,
        )
        return 3
    print("CHAT E2E PASS", flush=True)
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=["proxy", "direct"], default="proxy")
    parser.add_argument("--base", required=True, help="proxy URL or chat server URL")
    parser.add_argument("--username", default="")
    parser.add_argument("--password", default="")
    parser.add_argument(
        "--allow-poll-fallback",
        action="store_true",
        help="verify the /api/poll path when the server has no ws handlers",
    )
    parser.add_argument(
        "--env-file",
        default=str(Path(__file__).resolve().parent.parent / ".env"),
    )
    args = parser.parse_args()
    try:
        return asyncio.run(run(args))
    except Blocked as err:
        print(f"BLOCKED {err}", flush=True)
        return 3


if __name__ == "__main__":
    sys.exit(main())