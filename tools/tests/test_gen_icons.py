#!/usr/bin/env python3
"""Test gen_icons.py generates correct icon files from master icon."""

import subprocess
import sys
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parent.parent.parent
GEN_ICONS = ROOT / "tools" / "gen_icons.py"
ICONS_DIR = ROOT / "src-tauri" / "icons"
ASSET_MASTER = ROOT / "tools" / "assets" / "icon-512.png"


def test_gen_icons_runs_and_creates_all_icons():
    # Run gen_icons.py
    res = subprocess.run([sys.executable, str(GEN_ICONS)], capture_output=True, text=True)
    assert res.returncode == 0, f"gen_icons.py failed: {res.stderr}"

    # Verify asset master exists
    assert ASSET_MASTER.exists(), "tools/assets/icon-512.png must exist"
    master = Image.open(ASSET_MASTER)
    assert master.size == (512, 512), f"master must be 512x512, got {master.size}"

    # Verify PNG icons
    expected_sizes = {
        "icon.png": (512, 512),
        "128x128@" + "2x.png": (256, 256),
        "128x128.png": (128, 128),
        "32x32.png": (32, 32),
        "tray-idle.png": (32, 32),
        "tray-busy.png": (32, 32),
    }
    for filename, size in expected_sizes.items():
        p = ICONS_DIR / filename
        assert p.exists(), f"missing {filename}"
        img = Image.open(p)
        assert img.size == size, f"{filename} size expected {size}, got {img.size}"
        assert img.mode == "RGBA", f"{filename} mode expected RGBA, got {img.mode}"

    # Verify raw tray RGBA
    for tray_raw in ["tray-idle.rgba", "tray-busy.rgba"]:
        p = ICONS_DIR / tray_raw
        assert p.exists(), f"missing {tray_raw}"
        assert p.stat().st_size == 32 * 32 * 4, f"{tray_raw} size incorrect"

    # Verify icon.ico
    ico_path = ICONS_DIR / "icon.ico"
    assert ico_path.exists(), "missing icon.ico"
    assert ico_path.stat().st_size > 0, "icon.ico is empty"
    ico_img = Image.open(ico_path)
    assert ico_img.format == "ICO", f"icon.ico format is {ico_img.format}"

    # Verify icon.png matches master icon content
    icon_png = Image.open(ICONS_DIR / "icon.png")
    assert list(icon_png.getdata()) == list(master.convert("RGBA").getdata()), (
        "icon.png should match master icon-512.png"
    )


if __name__ == "__main__":
    test_gen_icons_runs_and_creates_all_icons()
    print("ALL TESTS PASSED")
