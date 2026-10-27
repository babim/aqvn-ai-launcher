#!/usr/bin/env python3
"""Deterministic chat-protocol stub for the AQVN Workspace chat shell.

Implements the bridge-proxy surface the UI expects so the shell can be driven
without the real chat server:

    POST /login {username,password}   -> {"ok": true}
    GET  /proxy/csrf                  -> {"ok": true, "token", "runtime_id"}
    POST /api/message_async           -> {"message", "context"}
    POST /api/stop                    -> {"message", "context", "stopped"}
    POST /api/chat_create             -> {"ok": true, "ctxid"}
    WS   /ws  (namespace)             -> state_request ACK + state_push

Test controls (no auth; loopback only):

    POST /stub/expire    -> every /api/* answers with the HTML login page
    POST /stub/restore   -> clears expired mode
    POST /stub/drop      -> drops the transport (client reconnects itself)
    POST /stub/restart   -> emits server_restart and drops the transport
    GET  /stub/events    -> recorded protocol events (JSON)
    GET  /stub/state     -> compact state summary (JSON)

Usage:
    python3 tools/chat_stub_server.py --port 3156
"""

from __future__ import annotations

import argparse
import asyncio
import json
import secrets
import time
import urllib.parse
import uuid

import socketio
import uvicorn
from fastapi import FastAPI, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import HTMLResponse, JSONResponse, RedirectResponse

NAMESPACE = "/ws"
LOGIN_HTML = (
    "<!doctype html><html><head><title>Sign in</title></head>"
    "<body><h1>Sign in</h1><p>Stub login page (simulated session expiry).</p>"
    "</body></html>"
)


class StubState:
    def __init__(self) -> None:
        self.runtime_id = secrets.token_hex(8)
        self.csrf_token = secrets.token_urlsafe(24)
        self.authed = False
        self.expired = False
        self.handlers_off = False
        self.selected: str | None = None
        self.contexts: dict[str, dict] = {}
        self.logs: dict[str, list[dict]] = {}
        self.log_progress = ""
        self.log_progress_active = False
        self.active_stream: str | None = None
        self.events: list[dict] = []
        self.sids: set[str] = set()
        self.counter = 0

    def event(self, kind: str, **fields) -> None:
        entry = {"kind": kind, "ts": round(time.time(), 3)}
        entry.update(fields)
        self.events.append(entry)
        del self.events[:-200]

    def next_no(self) -> int:
        self.counter += 1
        return self.counter

    def new_context(self, name: str | None = None) -> dict:
        ctx_id = str(uuid.uuid4())
        context = {
            "id": ctx_id,
            "name": name or f"Chat {len(self.contexts) + 1}",
            "no": len(self.contexts) + 1,
            "parent_context_id": None,
        }
        self.contexts[ctx_id] = context
        self.logs[ctx_id] = []
        self.selected = ctx_id
        self.event("context_created", context=ctx_id)
        return context

    def add_log(self, ctx_id: str, record: dict) -> dict:
        record.setdefault("no", self.next_no())
        record.setdefault("timestamp", time.time())
        record.setdefault("kvps", {})
        self.logs.setdefault(ctx_id, []).append(record)
        return record

    def find_log(self, ctx_id: str, log_id: str) -> dict | None:
        for record in self.logs.get(ctx_id, []):
            if record.get("id") == log_id:
                return record
        return None

    def snapshot(self, ctx_id: str | None) -> dict:
        return {
            "deselect_chat": False,
            "context": ctx_id,
            "contexts": list(self.contexts.values()),
            "tasks": [],
            "logs": list(self.logs.get(ctx_id or "", [])),
            "log_guid": ctx_id,
            "log_version": self.counter,
            "log_progress": self.log_progress,
            "log_progress_active": self.log_progress_active,
            "paused": False,
            "notifications": [],
            "notifications_guid": "",
            "notifications_version": 0,
        }


STATE = StubState()
sio = socketio.AsyncServer(
    async_mode="asgi", cors_allowed_origins="*", logger=False, engineio_logger=False
)
api = FastAPI(title="AQVN chat stub")
api.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)


def envelope(snapshot: dict) -> dict:
    return {
        "handlerId": "ws_webui",
        "eventId": str(uuid.uuid4()),
        "correlationId": str(uuid.uuid4()),
        "ts": int(time.time() * 1000),
        "data": {
            "runtime_epoch": STATE.runtime_id,
            "seq": STATE.counter,
            "snapshot": snapshot,
        },
    }


async def push(snapshot: dict | None = None) -> None:
    snap = snapshot if snapshot is not None else STATE.snapshot(STATE.selected)
    await sio.emit("state_push", envelope(snap), namespace=NAMESPACE)


async def read_body(request: Request) -> dict:
    raw = await request.body()
    if not raw:
        return {}
    content_type = request.headers.get("content-type", "")
    if "application/x-www-form-urlencoded" in content_type:
        return {k: v[0] for k, v in urllib.parse.parse_qs(raw.decode()).items()}
    try:
        return json.loads(raw.decode())
    except json.JSONDecodeError:
        return {}


# --------------------------------------------------------------------------
# Socket.IO
# --------------------------------------------------------------------------


@sio.event(namespace=NAMESPACE)
async def connect(sid, environ, auth):
    auth = auth or {}
    STATE.event(
        "socket_connect",
        csrf_present=bool(auth.get("csrf_token")),
        handlers=auth.get("handlers"),
    )
    if auth.get("csrf_token") != STATE.csrf_token:
        raise ConnectionRefusedError("invalid session")
    if list(auth.get("handlers") or []) != ["ws_webui"]:
        raise ConnectionRefusedError("unsupported handlers")
    STATE.sids.add(sid)


@sio.event(namespace=NAMESPACE)
async def disconnect(sid):
    STATE.sids.discard(sid)
    STATE.event("socket_disconnect")


@sio.on("state_request", namespace=NAMESPACE)
async def state_request(sid, data=None):
    data = data or {}
    STATE.event("state_request", payload=data)
    correlation = data.get("correlationId")
    context = data.get("context")
    if context and context in STATE.contexts:
        STATE.selected = context
    if STATE.handlers_off:
        return {
            "correlationId": correlation,
            "results": [
                {
                    "handlerId": "ws.dispatch",
                    "ok": False,
                    "error": {"code": "NO_HANDLERS", "error": "No handlers activated"},
                }
            ],
        }
    ack = {
        "correlationId": correlation,
        "results": [
            {
                "handlerId": "ws_webui",
                "ok": True,
                "correlationId": correlation,
                "data": {"runtime_epoch": STATE.runtime_id, "seq_base": 1},
            }
        ],
    }
    await push(STATE.snapshot(STATE.selected))
    return ack


# --------------------------------------------------------------------------
# REST
# --------------------------------------------------------------------------


@api.get("/proxy/csrf")
async def csrf():
    STATE.event("csrf_bootstrap")
    return {"ok": True, "token": STATE.csrf_token, "runtime_id": STATE.runtime_id}


@api.get("/api/csrf_token")
async def csrf_token():
    """Upstream CSRF endpoint the bridge proxy bootstraps from."""
    STATE.event("csrf_bootstrap")
    return {"ok": True, "token": STATE.csrf_token, "runtime_id": STATE.runtime_id}


@api.get("/")
async def root():
    return HTMLResponse(
        "<!doctype html><html><body><h1>AQVN chat stub</h1></body></html>"
    )


@api.post("/login")
async def login(request: Request):
    is_form = "application/x-www-form-urlencoded" in request.headers.get(
        "content-type", ""
    )
    body = await read_body(request)
    username = str(body.get("username", ""))
    password = str(body.get("password", ""))
    ok = username == "admin" and password == "test-auth-2026"
    STATE.event("login", username=username, ok=ok)
    if not ok:
        if is_form:
            # Mirrors the real server: re-render the login page.
            return HTMLResponse(LOGIN_HTML)
        return JSONResponse({"ok": False, "error": "Invalid credentials"}, status_code=401)
    STATE.authed = True
    if not STATE.contexts:
        STATE.new_context("Welcome")
    if is_form:
        # The bridge proxy requires a redirect away from /login on success.
        return RedirectResponse("/", status_code=302)
    return {"ok": True}


def expired_response() -> HTMLResponse:
    STATE.event("api_expired")
    return HTMLResponse(LOGIN_HTML)


@api.post("/api/message_async")
async def message_async(request: Request):
    if STATE.expired:
        return expired_response()
    body = await read_body(request)
    text = str(body.get("text", ""))
    message_id = str(body.get("message_id") or uuid.uuid4())
    context_id = str(body.get("context") or "")
    STATE.event(
        "message_async",
        text=text,
        context=context_id or None,
        message_id=message_id,
        csrf_present=bool(request.headers.get("x-csrf-token")),
    )
    if context_id not in STATE.contexts:
        context_id = STATE.new_context()["id"]
    STATE.selected = context_id

    if not STATE.find_log(context_id, message_id):
        STATE.add_log(
            context_id,
            {
                "id": message_id,
                "type": "user",
                "heading": "",
                "content": text,
                "kvps": {},
                "agentno": None,
            },
        )

    stream_id = str(uuid.uuid4())
    STATE.active_stream = stream_id
    asyncio.create_task(stream_reply(context_id, stream_id))

    return {"message": "Message received.", "context": context_id}


async def stream_reply(context_id: str, stream_id: str) -> None:
    response_id = str(uuid.uuid4())
    agent_id = str(uuid.uuid4())

    STATE.log_progress = "Generating response..."
    STATE.log_progress_active = True
    await push()

    await asyncio.sleep(0.2)
    if STATE.active_stream != stream_id:
        return

    STATE.add_log(
        context_id,
        {
            "id": agent_id,
            "type": "agent",
            "heading": "Assistant",
            "content": "Thinking about your message.",
            "kvps": {},
            "agentno": 1,
        },
    )
    STATE.add_log(
        context_id,
        {
            "id": response_id,
            "type": "response",
            "heading": "Assistant",
            "content": "Hel",
            "kvps": {},
            "agentno": 1,
        },
    )
    await push()

    await asyncio.sleep(0.35)
    if STATE.active_stream != stream_id:
        return
    record = STATE.find_log(context_id, response_id)
    if record is not None:
        record["content"] = "Hello world"
    await push()

    await asyncio.sleep(0.35)
    if STATE.active_stream != stream_id:
        return

    STATE.log_progress = ""
    STATE.log_progress_active = False
    STATE.active_stream = None
    STATE.add_log(
        context_id,
        {
            "id": str(uuid.uuid4()),
            "type": "info",
            "heading": "",
            "content": "Task finished.",
            "kvps": {"finished": True},
            "agentno": 1,
        },
    )
    await push()


@api.post("/api/stop")
async def stop(request: Request):
    if STATE.expired:
        return expired_response()
    body = await read_body(request)
    context_id = str(body.get("context") or STATE.selected or "")
    STATE.event("stop", context=context_id)
    STATE.active_stream = None
    STATE.log_progress = ""
    STATE.log_progress_active = False
    if context_id in STATE.contexts:
        STATE.add_log(
            context_id,
            {
                "id": str(uuid.uuid4()),
                "type": "info",
                "heading": "",
                "content": "Stopped.",
                "kvps": {},
                "agentno": 1,
            },
        )
    await push()
    return {"message": "Stopped.", "context": context_id, "stopped": True}


@api.post("/api/chat_create")
async def chat_create(request: Request):
    if STATE.expired:
        return expired_response()
    body = await read_body(request)
    STATE.event("chat_create", current_context=body.get("current_context"))
    STATE.active_stream = None
    STATE.log_progress = ""
    STATE.log_progress_active = False
    context = STATE.new_context()
    await push()
    return {"ok": True, "ctxid": context["id"], "message": "New chat created."}


@api.post("/api/poll")
async def poll_post(request: Request):
    if STATE.expired:
        return expired_response()
    body = await read_body(request)
    STATE.event("poll", context=body.get("context"), log_from=body.get("log_from"))
    return STATE.snapshot(str(body.get("context") or "") or STATE.selected)


@api.get("/api/poll")
async def poll(context: str | None = None):
    if STATE.expired:
        return expired_response()
    return STATE.snapshot(context or STATE.selected)


# --------------------------------------------------------------------------
# Test controls
# --------------------------------------------------------------------------


async def drop_transports() -> int:
    """Drop clients so the reconnect path is exercised (socket.io disconnect)."""
    dropped = 0
    for sid in list(STATE.sids):
        try:
            await sio.disconnect(sid, namespace=NAMESPACE)
            dropped += 1
            continue
        except Exception:  # noqa: BLE001 - test helper, best effort
            pass
        try:
            await sio.eio.disconnect(sid)
            dropped += 1
        except Exception:  # noqa: BLE001 - test helper, best effort
            pass
    STATE.sids.clear()
    return dropped


@api.post("/stub/expire")
async def stub_expire():
    STATE.expired = True
    return {"ok": True, "expired": True}


@api.post("/stub/restore")
async def stub_restore():
    STATE.expired = False
    return {"ok": True, "expired": False}


@api.post("/stub/handlers-off")
async def stub_handlers_off():
    STATE.handlers_off = True
    return {"ok": True, "handlers_off": True}


@api.post("/stub/handlers-on")
async def stub_handlers_on():
    STATE.handlers_off = False
    return {"ok": True, "handlers_off": False}


@api.post("/stub/drop")
async def stub_drop():
    STATE.event("stub_drop")
    dropped = await drop_transports()
    return {"ok": True, "dropped": dropped}


@api.post("/stub/restart")
async def stub_restart():
    STATE.event("stub_restart")
    STATE.runtime_id = secrets.token_hex(8)
    STATE.csrf_token = secrets.token_urlsafe(24)
    await sio.emit("server_restart", {"runtime_epoch": STATE.runtime_id}, namespace=NAMESPACE)
    await asyncio.sleep(0.1)
    dropped = await drop_transports()
    return {"ok": True, "dropped": dropped, "runtime_id": STATE.runtime_id}


@api.get("/stub/events")
async def stub_events():
    return {"ok": True, "events": STATE.events}


@api.get("/stub/state")
async def stub_state():
    return {
        "ok": True,
        "runtime_id": STATE.runtime_id,
        "contexts": list(STATE.contexts.values()),
        "selected": STATE.selected,
        "log_progress_active": STATE.log_progress_active,
        "logs": {k: len(v) for k, v in STATE.logs.items()},
    }


app = socketio.ASGIApp(sio, other_asgi_app=api, socketio_path="socket.io")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=3156)
    args = parser.parse_args()
    uvicorn.run(app, host=args.host, port=args.port, log_level="warning")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())