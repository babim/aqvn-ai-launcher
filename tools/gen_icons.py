#!/usr/bin/env python3
"""Generate the AQVN Workspace icon set from the master official icon."""

import urllib.request
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "src-tauri" / "icons"
ASSETS = Path(__file__).resolve().parent / "assets"
MASTER_FILE = ASSETS / "icon-512.png"
MASTER_URL = "https://a0.matmagoc.com/public/icon-512.png"

BG = (11, 18, 32, 255)
TEAL = (34, 211, 238, 255)
WHITE = (248, 250, 252, 255)


def load_master_icon() -> Image.Image:
    """Load the 512x512 master icon, fetching from official URL if absent."""
    if not MASTER_FILE.exists():
        ASSETS.mkdir(parents=True, exist_ok=True)
        req = urllib.request.Request(MASTER_URL, headers={"User-Agent": "Mozilla/5.0"})
        with urllib.request.urlopen(req, timeout=30) as resp:
            MASTER_FILE.write_bytes(resp.read())
    img = Image.open(MASTER_FILE).convert("RGBA")
    if img.size != (512, 512):
        img = img.resize((512, 512), Image.LANCZOS)
    return img


def draw_tray(size: int = 32, busy: bool = False, master: Image.Image | None = None) -> Image.Image:
    """Tray glyph: idle = official logo directly, busy = logo with amber badge dot."""
    if master is None:
        master = load_master_icon()
    base = master.resize((size, size), Image.LANCZOS).convert("RGBA")
    if not busy:
        return base
    img = base.copy()
    draw = ImageDraw.Draw(img)
    # Distinct amber badge dot at bottom-right
    draw.ellipse((21, 21, 31, 31), fill=(245, 158, 11, 255), outline=(11, 18, 32, 255), width=1)
    return img


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    master = load_master_icon()
    idle = draw_tray(32, False, master)
    busy = draw_tray(32, True, master)
    idle.save(OUT / "tray-idle.png")
    busy.save(OUT / "tray-busy.png")
    # Raw RGBA (32x32) embedded in the binary; avoids pulling the PNG decoder.
    (OUT / "tray-idle.rgba").write_bytes(idle.tobytes())
    (OUT / "tray-busy.rgba").write_bytes(busy.tobytes())

    targets = {
        "icon.png": 512,
        "128x128@" + "2x.png": 256,
        "128x128.png": 128,
        "32x32.png": 32,
    }
    for name, size in targets.items():
        if master.size == (size, size):
            icon = master.copy()
        else:
            icon = master.resize((size, size), Image.LANCZOS)
        icon.save(OUT / name, format="PNG")

    ico_base = master.resize((256, 256), Image.LANCZOS)
    ico_base.save(
        OUT / "icon.ico",
        format="ICO",
        sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )
    print(f"wrote icons to {OUT}")


if __name__ == "__main__":
    main()
