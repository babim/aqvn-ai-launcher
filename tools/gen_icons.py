#!/usr/bin/env python3
"""Generate the AQVN Workspace icon set from a geometric AQVN mark."""

from pathlib import Path

from PIL import Image, ImageDraw

OUT = Path(__file__).resolve().parent.parent / "src-tauri" / "icons"

BG = (11, 18, 32, 255)
TEAL = (34, 211, 238, 255)
WHITE = (248, 250, 252, 255)


def draw_mark(size: int) -> Image.Image:
    """Render the mark at 4x then downsample for clean edges."""
    scale = 4
    s = size * scale
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    draw.rounded_rectangle((0, 0, s - 1, s - 1), radius=int(s * 0.22), fill=BG)

    cx = s / 2
    apex_y = s * 0.20
    base_y = s * 0.80
    half_base = s * 0.27
    stroke = max(1, int(s * 0.105))

    draw.line(
        [(cx, apex_y), (cx - half_base, base_y)],
        fill=TEAL,
        width=stroke,
        joint="curve",
    )
    draw.line(
        [(cx, apex_y), (cx + half_base, base_y)],
        fill=WHITE,
        width=stroke,
        joint="curve",
    )
    bar_y = base_y - (base_y - apex_y) * 0.36
    draw.line(
        [(cx - half_base * 0.52, bar_y), (cx + half_base * 0.52, bar_y)],
        fill=TEAL,
        width=max(1, int(stroke * 0.72)),
    )
    return img.resize((size, size), Image.LANCZOS)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for name, size in {
        "icon.png": 512,
        "32x32.png": 32,
        "128x128.png": 128,
        "128x128@2x.png": 256,
    }.items():
        draw_mark(size).save(OUT / name)
    draw_mark(256).save(
        OUT / "icon.ico",
        sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )
    print(f"wrote icons to {OUT}")


if __name__ == "__main__":
    main()
