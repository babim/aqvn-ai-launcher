#!/usr/bin/env python3
"""Automated title-sync and navigation verification under Xvfb (VAL-SCAF-007, VAL-SCAF-008, VAL-SCAF-009)."""

import os
import subprocess
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent


def check_static_assignment() -> None:
    ui_dir = REPO_ROOT / "ui"
    found = False
    for html_file in ui_dir.glob("*.html"):
        content = html_file.read_text(encoding="utf-8")
        if 'document.title = "AQVN AI Workspace"' in content:
            print(f"[OK] Found literal document.title assignment in {html_file.name}")
            found = True
            break
    if not found:
        print("[FAIL] Missing literal assignment in ui/", file=sys.stderr)
        sys.exit(1)


def check_runtime_sync() -> None:
    binary = REPO_ROOT / "src-tauri" / "target" / "release" / "aqvn-workspace"
    if not binary.exists():
        print(f"[FAIL] Missing release binary: {binary}", file=sys.stderr)
        sys.exit(1)

    display = ":97"
    env = dict(
        os.environ,
        DISPLAY=display,
        WEBKIT_DISABLE_DMABUF_RENDERER="1",
    )

    print("[*] Starting Xvfb on display :97...")
    xvfb = subprocess.Popen(["Xvfb", display, "-screen", "0", "1280x800x24"])
    time.sleep(1)

    app = None
    try:
        print(f"[*] Starting {binary.name}...")
        app = subprocess.Popen([str(binary)], cwd=str(REPO_ROOT), env=env)
        time.sleep(3)

        if app.poll() is not None:
            print(
                f"[FAIL] App exited prematurely with code {app.returncode}",
                file=sys.stderr,
            )
            sys.exit(1)

        # Query window ID
        res = subprocess.run(
            ["xdotool", "search", "--name", "AQVN"],
            env=env,
            capture_output=True,
            text=True,
        )
        wids = [w.strip() for w in res.stdout.splitlines() if w.strip()]
        if not wids:
            print("[FAIL] No AQVN window found via xdotool", file=sys.stderr)
            sys.exit(1)

        main_wid = wids[-1]
        print(f"[OK] Main window ID: {main_wid}")

        # Phase 0: index.html
        t0 = subprocess.run(
            ["xdotool", "getwindowname", main_wid],
            env=env,
            capture_output=True,
            text=True,
        ).stdout.strip()
        print(f"[OK] Initial window title: {t0!r}")
        assert (
            t0 == "AQVN AI Workspace"
        ), f"Expected 'AQVN AI Workspace', got {t0!r}"
        subprocess.run(
            [
                "ffmpeg",
                "-y",
                "-f",
                "x11grab",
                "-i",
                display,
                "-frames:v",
                "1",
                "/tmp/title_sync_0_index.png",
            ],
            capture_output=True,
        )

        # Phase 1: Click Settings
        print("[*] Clicking Settings link at (40, 30)...")
        subprocess.run(
            [
                "xdotool",
                "mousemove",
                "--window",
                main_wid,
                "40",
                "30",
                "click",
                "1",
            ],
            env=env,
        )
        time.sleep(2)

        t1 = subprocess.run(
            ["xdotool", "getwindowname", main_wid],
            env=env,
            capture_output=True,
            text=True,
        ).stdout.strip()
        print(f"[OK] Settings window title: {t1!r}")
        assert (
            t1 == "AQVN Workspace — Settings"
        ), f"Expected 'AQVN Workspace — Settings', got {t1!r}"
        subprocess.run(
            [
                "ffmpeg",
                "-y",
                "-f",
                "x11grab",
                "-i",
                display,
                "-frames:v",
                "1",
                "/tmp/title_sync_1_settings.png",
            ],
            capture_output=True,
        )

        # Phase 2: Click Back to Chat
        print("[*] Clicking Back to chat link at (40, 30)...")
        subprocess.run(
            [
                "xdotool",
                "mousemove",
                "--window",
                main_wid,
                "40",
                "30",
                "click",
                "1",
            ],
            env=env,
        )
        time.sleep(2)

        t2 = subprocess.run(
            ["xdotool", "getwindowname", main_wid],
            env=env,
            capture_output=True,
            text=True,
        ).stdout.strip()
        print(f"[OK] Round-trip window title: {t2!r}")
        assert (
            t2 == "AQVN AI Workspace"
        ), f"Expected 'AQVN AI Workspace', got {t2!r}"
        subprocess.run(
            [
                "ffmpeg",
                "-y",
                "-f",
                "x11grab",
                "-i",
                display,
                "-frames:v",
                "1",
                "/tmp/title_sync_2_index.png",
            ],
            capture_output=True,
        )

        print("[SUCCESS] All title-sync and navigation checks passed.")

    finally:
        if app and app.poll() is None:
            app.terminate()
            try:
                app.wait(timeout=3)
            except subprocess.TimeoutExpired:
                app.kill()
        xvfb.terminate()
        try:
            xvfb.wait(timeout=3)
        except subprocess.TimeoutExpired:
            xvfb.kill()


if __name__ == "__main__":
    check_static_assignment()
    check_runtime_sync()
