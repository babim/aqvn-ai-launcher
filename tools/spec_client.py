#!/usr/bin/env python3
"""Standalone replica of the RFC client wire format from the upstream runtime.

Mirrors the caller-side helpers 1:1 with the standard library only:

    hash = hmac.new(password.encode(), rfc_input.encode(), sha256).hexdigest()
    POST {url}  json={"rfc_input": rfc_input, "hash": hash}

Gate 3 scenarios:
  1. valid hash + read of a written fixture  -> 200 + exact base64 bytes
  2. wrong hash                              -> 403 text/plain
  3. path outside the sandbox                -> 403 text/plain
  4. (password "secret" only) the spec's precomputed-hash worked example

Usage:
    python3 tools/spec_client.py                     # boot the subject app itself
    python3 tools/spec_client.py --url URL --password PW --no-spawn
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import hmac
import json
import os
import socket
import subprocess
import sys
import urllib.error
import urllib.request

DEFAULT_URL = "http://127.0.0.1:55080/api/rfc"
DEFAULT_PASSWORD = "rfc-e2e-test-password"
FIXTURE = b"spec-client fixture: \x00\x01\x02\xfe\xff\n"
WORKED_EXAMPLE_HASH = "b3baa5df8d06d6fe2418d020ff8000b3060db4026ebd2fae45eee854db379694"
WORKED_EXAMPLE_INPUT = (
    '{"module": "helpers.rfc_files", "function_name": '
    '"_read_file_binary_impl", "args": ["/a0/usr/workdir/test.txt"], "kwargs": {}}'
)


def hash_data(data: str, password: str) -> str:
    return hmac.new(password.encode(), data.encode(), hashlib.sha256).hexdigest()


def rfc_input(module: str, function_name: str, args: list, kwargs: dict) -> str:
    return json.dumps(
        {
            "module": module,
            "function_name": function_name,
            "args": args,
            "kwargs": kwargs,
        }
    )


def post(url: str, body: dict) -> tuple[int, str, bytes]:
    request = urllib.request.Request(
        url,
        data=json.dumps(body).encode(),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            return response.status, response.headers.get("Content-Type", ""), response.read()
    except urllib.error.HTTPError as err:
        return err.code, err.headers.get("Content-Type", ""), err.read()


class Client:
    def __init__(self, url: str, password: str) -> None:
        self.url = url
        self.password = password

    def call(
        self,
        module: str,
        function_name: str,
        args: list,
        kwargs: dict | None = None,
        password: str | None = None,
    ) -> tuple[int, str, bytes]:
        payload = rfc_input(module, function_name, args, kwargs or {})
        return post(
            self.url,
            {
                "rfc_input": payload,
                "hash": hash_data(payload, password or self.password),
            },
        )

    def call_raw(self, payload: str, digest: str) -> tuple[int, str, bytes]:
        return post(self.url, {"rfc_input": payload, "hash": digest})


def check(name: str, condition: bool, detail: str) -> None:
    if not condition:
        raise AssertionError(f"{name}: {detail}")
    print(f"PASS {name}: {detail}")


def port_open(url: str) -> bool:
    host = url.split("//", 1)[-1].split("/", 1)[0]
    hostname, _, port = host.partition(":")
    try:
        with socket.create_connection((hostname, int(port or "80")), timeout=1):
            return True
    except OSError:
        return False


def spawn_app(repo_root: str, password: str) -> None:
    env = dict(os.environ)
    env["AQVN_E2E_RFC_PASSWORD"] = password
    subprocess.run(
        ["bash", os.path.join(repo_root, "tools", "_e2e_lib.sh"), "start"],
        check=True,
        env=env,
    )


def stop_app(repo_root: str, password: str) -> None:
    env = dict(os.environ)
    env["AQVN_E2E_RFC_PASSWORD"] = password
    subprocess.run(
        ["bash", os.path.join(repo_root, "tools", "_e2e_lib.sh"), "stop"],
        check=False,
        env=env,
    )


def run_gate(client: Client, password: str) -> int:
    encoded = base64.b64encode(FIXTURE).decode()
    target = "/a0/spec_client/fixture.bin"

    status, content_type, body = client.call(
        "helpers.rfc_files", "_write_file_binary_impl", [target, encoded]
    )
    check(
        "[1/3] valid hash write",
        status == 200 and body == b"true",
        f"HTTP {status} {content_type} body={body[:40]!r}",
    )

    status, content_type, body = client.call(
        "helpers.rfc_files", "_read_file_binary_impl", [target]
    )
    returned = json.loads(body) if status == 200 else None
    check(
        "[1/3] valid hash read",
        status == 200 and returned == encoded and base64.b64decode(returned) == FIXTURE,
        f"HTTP {status} {content_type} bytes={len(base64.b64decode(returned or ''))}",
    )

    payload = rfc_input("helpers.rfc_files", "_read_file_binary_impl", [target], {})
    good = hash_data(payload, password)
    bad = ("0" if good[-1] != "0" else "1") + good[:-1]
    status, content_type, body = client.call_raw(payload, bad)
    check(
        "[2/3] wrong hash",
        status == 403 and content_type.startswith("text/plain") and b"HMAC" in body,
        f"HTTP {status} {content_type} body={body[:60]!r}",
    )

    status, content_type, body = client.call(
        "helpers.rfc_files", "_read_file_binary_impl", ["/etc/passwd"]
    )
    check(
        "[3/3] outside sandbox path",
        status == 403
        and content_type.startswith("text/plain")
        and b"root:" not in body,
        f"HTTP {status} {content_type} body={body[:60]!r}",
    )

    if password == "secret":
        status, content_type, body = client.call(
            "helpers.rfc_files",
            "_write_file_binary_impl",
            ["/a0/usr/workdir/test.txt", encoded],
        )
        check(
            "[spec 7] worked-example fixture",
            status == 200 and body == b"true",
            f"HTTP {status} {content_type}",
        )
        status, content_type, body = client.call_raw(
            WORKED_EXAMPLE_INPUT, WORKED_EXAMPLE_HASH
        )
        check(
            "[spec 7] precomputed hash",
            status == 200 and json.loads(body) == encoded,
            f"HTTP {status} {content_type}",
        )

    print("Gate 3: all spec_client scenarios passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", default=os.environ.get("AQVN_E2E_RFC_URL", DEFAULT_URL))
    parser.add_argument(
        "--password",
        default=os.environ.get("AQVN_E2E_RFC_PASSWORD", DEFAULT_PASSWORD),
    )
    parser.add_argument(
        "--no-spawn",
        action="store_true",
        help="require an already-running server instead of booting the subject app",
    )
    args = parser.parse_args()

    repo_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    spawned = False
    try:
        if not port_open(args.url):
            if args.no_spawn:
                raise SystemExit(f"no server listening at {args.url} (--no-spawn given)")
            spawn_app(repo_root, args.password)
            spawned = True
            if not port_open(args.url):
                raise SystemExit(f"subject app did not open {args.url}")

        client = Client(args.url, args.password)
        return run_gate(client, args.password)
    finally:
        if spawned:
            stop_app(repo_root, args.password)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except AssertionError as err:
        print(f"FAIL {err}", file=sys.stderr)
        sys.exit(1)
    except SystemExit:
        raise