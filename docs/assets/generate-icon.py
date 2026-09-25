#!/usr/bin/env python3
"""Regenerates icon-source.png from the wordmark.

`icon-source.png` isn't hand-edited: it's the wordmark composited onto a
dark card shaped as a macOS "continuous corner" superellipse (squircle),
sized to Apple's icon safe area. Re-run this after `wordmark.png` changes,
or to tweak the shape/gradient/margin constants below.

Requires: pillow, numpy (`pip install pillow numpy`)
"""

from pathlib import Path

import numpy as np
from PIL import Image

ASSETS_DIR = Path(__file__).parent
WORDMARK_PATH = ASSETS_DIR.parent.parent / "agent" / "src" / "assets" / "wordmark.png"
OUTPUT_PATH = ASSETS_DIR / "icon-source.png"

CANVAS = 1024
# macOS's icon safe area since Big Sur: content fills ~824/1024 of the
# canvas, leaving ~10% margin so the icon isn't visually heavier than its
# siblings in the Dock and the Cmd+Tab switcher.
CONTENT = round(CANVAS * 824 / 1024)
# Superellipse exponent approximating macOS's continuous-corner icon shape
# (smoother than a simple rounded rectangle).
SQUIRCLE_EXPONENT = 5.0

# A near-black diagonal gradient, dark grays only (R=G=B). Tune these to
# match the brand card's background if it changes.
GRADIENT_TOP_RIGHT = 2
GRADIENT_BOTTOM_LEFT = 30
GRADIENT_TOP_LEFT = 13
GRADIENT_BOTTOM_RIGHT = 20

# The wordmark's size and position as a fraction of the content square,
# matching its proportions in the original hand-designed icon.
WORDMARK_WIDTH_FRACTION = 0.698
WORDMARK_CENTER_X_FRACTION = 0.4956
WORDMARK_CENTER_Y_FRACTION = 0.4978


def build_gradient(size: int) -> np.ndarray:
    """A grayscale diagonal gradient, fitted so its four corners roughly
    match the constants above."""
    yy, xx = np.mgrid[0:size, 0:size].astype(np.float64)
    xfrac, yfrac = xx / size, yy / size

    corners = np.array(
        [
            [0, 0, GRADIENT_TOP_LEFT],
            [1, 0, GRADIENT_TOP_RIGHT],
            [0, 1, GRADIENT_BOTTOM_LEFT],
            [1, 1, GRADIENT_BOTTOM_RIGHT],
        ]
    )
    design = np.column_stack([corners[:, 0], corners[:, 1], np.ones(4)])
    coef, *_ = np.linalg.lstsq(design, corners[:, 2], rcond=None)
    a_x, a_y, c0 = coef
    return np.clip(a_x * xfrac + a_y * yfrac + c0, 0, 255)


def build_squircle_mask(size: int, exponent: float) -> np.ndarray:
    yy, xx = np.mgrid[0:size, 0:size].astype(np.float64)
    s = size / 2.0
    cx, cy = xx - s + 0.5, yy - s + 0.5
    superellipse = (np.abs(cx) / s) ** exponent + (np.abs(cy) / s) ** exponent
    return superellipse <= 1.0


def main() -> None:
    gray = build_gradient(CONTENT)
    inside = build_squircle_mask(CONTENT, SQUIRCLE_EXPONENT)

    rgba = np.zeros((CONTENT, CONTENT, 4), dtype=np.uint8)
    rgba[..., 0] = rgba[..., 1] = rgba[..., 2] = gray.astype(np.uint8)
    rgba[..., 3] = np.where(inside, 255, 0).astype(np.uint8)
    content = Image.fromarray(rgba)

    wordmark = Image.open(WORDMARK_PATH).convert("RGBA")
    target_w = round(CONTENT * WORDMARK_WIDTH_FRACTION)
    target_h = round(wordmark.height * (target_w / wordmark.width))
    wordmark = wordmark.resize((target_w, target_h), Image.LANCZOS)
    wx = round(CONTENT * WORDMARK_CENTER_X_FRACTION - target_w / 2)
    wy = round(CONTENT * WORDMARK_CENTER_Y_FRACTION - target_h / 2)
    content.alpha_composite(wordmark, (wx, wy))

    canvas = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    offset = (CANVAS - CONTENT) // 2
    canvas.paste(content, (offset, offset), content)
    canvas.save(OUTPUT_PATH)
    print(f"Wrote {OUTPUT_PATH} ({canvas.size[0]}x{canvas.size[1]})")


if __name__ == "__main__":
    main()
