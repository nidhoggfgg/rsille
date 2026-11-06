"""Rebuild embedded 40x48 RGBA sprites from the generated transparent sheet.

Development only: python3 -m pip install Pillow; python3 tools/pack_sprites.py
Runtime needs neither Python, PNG decoding, nor access to these source files.
"""
from pathlib import Path
from PIL import Image

ASSETS = Path(__file__).resolve().parent.parent / "assets"
source = Image.open(ASSETS / "source-v7.png").convert("RGBA")
# Keep Claude's accepted sprite independent of revisions to the other two.
claude_source = Image.open(ASSETS / "source-v6.png").convert("RGBA")
# Reserve skin, mouth and highlight colors explicitly so small facial
# features are not displaced by the much larger hair and clothing regions.
FACE = [
    (36, 18, 19), (95, 54, 42), (165, 111, 85),
    (254, 228, 208), (235, 180, 147), (243, 161, 142),
    (255, 248, 238), (221, 204, 175),
]
COLORS = [
    # Navy maid dress, blue hair/whale tail, white frills and blue eyes.
    [(28, 29, 64), (47, 59, 115), (67, 88, 160), (85, 115, 193),
     (95, 153, 228), (127, 197, 247), (192, 186, 217), (225, 217, 235),
     (221, 170, 79)],
    # Long copper hair, ivory dress, charcoal ribbons and gold sun clip.
    [(236, 130, 65), (206, 94, 43), (166, 65, 30), (255, 169, 94),
     (48, 39, 37), (86, 64, 48), (219, 153, 57), (255, 211, 121),
     (244, 210, 173)],
    # ChatGPT uses its own compact palette below, including facial colors.
    [],
]
CHATGPT_COLORS = [
    (254, 228, 208), (164, 86, 54), (255, 248, 248), (223, 207, 236),
    (157, 132, 180), (57, 37, 73), (145, 93, 193), (213, 166, 76),
]
preview = Image.new("RGBA", (126, 52), (30, 30, 36, 255))
for i, (name, left, right) in enumerate([
    ("deepseek", 0, 512), ("claude", 512, 990), ("chatgpt", 990, 1536)
]):
    sheet = claude_source if name == "claude" else source
    sprite = sheet.crop((left, 0, right, sheet.height))
    bounds = sprite.getchannel("A").point(lambda a: 255 if a >= 128 else 0).getbbox()
    sprite = sprite.crop(bounds)
    sprite.thumbnail((40, 48), Image.Resampling.NEAREST)
    # Preserve the source's discrete eye highlights and iris colors. Averaging
    # these tiny clusters blends the glint into skin and dulls the expression.
    # A fixed palette and binary alpha keep half-block pixels crisp.
    alpha = sprite.getchannel("A").point(lambda a: 255 if a >= 128 else 0)
    colors = CHATGPT_COLORS if name == "chatgpt" else FACE + COLORS[i]
    palette = Image.new("P", (1, 1))
    palette.putpalette([channel for rgb in colors + [colors[0]] * (256 - len(colors)) for channel in rgb])
    sprite = sprite.convert("RGB").quantize(palette=palette, dither=Image.Dither.NONE).convert("RGBA")
    sprite.putalpha(alpha)
    tile = Image.new("RGBA", (40, 48))
    tile.paste(sprite, ((40 - sprite.width) // 2, 48 - sprite.height))
    tile.save(ASSETS / f"{name}.png")
    (ASSETS / f"{name}.rgba").write_bytes(tile.tobytes())
    preview.alpha_composite(tile, (i * 42 + 1, 2))
preview.convert("RGB").resize((1008, 416), Image.Resampling.NEAREST).save(ASSETS / "sprites-preview.png")
