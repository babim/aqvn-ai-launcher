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


def draw_tray(size: int, busy: bool) -> Image.Image:
    """Tray glyph: idle = teal ring, busy = amber filled disc with a notch."""
    scale = 4
    s = size * scale
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    pad = int(s * 0.10)
    if busy:
        draw.ellipse((pad, pad, s - pad, s - pad), fill=(245, 158, 11, 255))
        draw.ellipse(
            (int(s * 0.34), int(s * 0.34), int(s * 0.66), int(s * 0.66)),
            fill=(11, 18, 32, 255),
        )
    else:
        stroke = max(1, int(s * 0.16))
        draw.ellipse((pad, pad, s - pad, s - pad), outline=TEAL, width=stroke)
        draw.ellipse(
            (int(s * 0.42), int(s * 0.42), int(s * 0.58), int(s * 0.58)),
            fill=WHITE,
        )
    return img.resize((size, size), Image.LANCZOS)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    draw_tray(32, False).save(OUT / "tray-idle.png")
    draw_tray(32, True).save(OUT / "tray-busy.png")
    # Raw RGBA (32x32) embedded in the binary; avoids pulling the PNG decoder.
    (OUT / "tray-idle.rgba").write_bytes(draw_tray(32, False).tobytes())
    (OUT / "tray-busy.rgba").write_bytes(draw_tray(32, True).tobytes())

    master = load_master_icon()
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
