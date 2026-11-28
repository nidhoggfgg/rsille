"""Losslessly pack the approved 240×80 image for the static Quadrant demo."""
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]

with Image.open(ROOT / "assets/chibi-trio-240x80.png") as source:
    if source.size != (240, 80):
        raise ValueError("Expected a 240×80 reference image")
    if source.convert("RGBA").getchannel("A").getextrema() != (255, 255):
        raise ValueError("Expected an opaque reference image")
    (ROOT / "assets/chibi-trio-240x80.rgb").write_bytes(source.convert("RGB").tobytes())
