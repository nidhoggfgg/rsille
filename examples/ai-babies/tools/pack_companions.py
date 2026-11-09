"""Pack the generated DeepSeek/Claude sheets for runtime playback (Pillow only).

Each source is a 4x6 grid of 256px cells. Sample 96x48 and 128x64 Quadrant
frames directly from the cleaned source for real horizontal detail, with an
explicit material palette and binary alpha. Legacy square assets are untouched.
Idle/wave share stationary pixels; only authored eyes/arm/expression move.
No per-frame crop or recentering: authored walking/jumping offsets survive.
ChatGPT uses the separately tuned pack_actions.py pipeline.
"""
from collections import Counter
from pathlib import Path
from PIL import Image, ImageDraw

ASSETS = Path(__file__).resolve().parent.parent / "assets"
COLUMNS, ROWS, CELL = 4, 6, 256

# Few deliberately separated shades per material instead of frequency-based
# quantization, which spends colors on hair noise and loses eyes/skin/trim.
PALETTES = {
    "deepseek": [
        (24, 20, 48), (38, 44, 83), (49, 67, 126), (65, 96, 170),
        (100, 150, 223), (144, 206, 250), (207, 211, 235), (255, 251, 249),
        (255, 233, 213), (239, 190, 164), (224, 143, 132), (155, 65, 70),
        (78, 109, 199), (54, 150, 225), (255, 255, 255), (151, 156, 188),
    ],
    "claude": [
        (43, 23, 26), (91, 42, 30), (151, 62, 35), (205, 91, 44),
        (241, 137, 70), (255, 183, 104), (53, 43, 45), (98, 81, 75),
        (215, 199, 177), (255, 249, 231), (255, 233, 213), (239, 190, 164),
        (224, 143, 132), (155, 65, 70), (237, 174, 67), (174, 112, 43),
        (255, 255, 255),
    ],
}

# Source-cell coordinates, before the common 16px padding. Include both the
# lowered and raised arm so compositing cannot leave an extra hand behind.
MOTION = {
    "deepseek": {
        "eyes": [(86, 111, 119, 138), (138, 111, 172, 138)],
        "face": [(86, 111, 172, 155)],
        "arm": [(68, 134, 112, 194)],
    },
    "claude": {
        "eyes": [(87, 94, 120, 123), (136, 94, 169, 123)],
        "face": [(87, 94, 169, 140)],
        "arm": [(73, 124, 116, 181)],
    },
}


def motion_mask(name, size, frame):
    regions = MOTION[name]
    boxes = regions["eyes"] if frame == 2 else (
        regions["arm"] + (regions["face"] if frame == 6 else [])
        if frame in (5, 6, 7) else [])
    width, height = size
    mask = Image.new("L", size)
    draw = ImageDraw.Draw(mask)
    for box in boxes:
        draw.rectangle([round((v + 16) * axis / (CELL + 32))
                                for v, axis in zip(box, (width, height, width, height))], fill=255)
    return mask


def clean_hair_specks(frame, name):
    """Merge isolated near-color hair pixels; never touch skin, glints or edges."""
    hair = set(PALETTES[name][2:6])
    pixels = frame.copy().load()
    for y in range(1, frame.height - 1):
        for x in range(1, frame.width - 1):
            pixel = pixels[x, y]
            if pixel[3] != 255 or pixel[:3] not in hair:
                continue
            neighbors = [pixels[x + dx, y + dy] for dy in (-1, 0, 1)
                         for dx in (-1, 0, 1) if dx or dy]
            if any(p[3] != 255 for p in neighbors) or pixel in neighbors:
                continue
            dominant, count = Counter(neighbors).most_common(1)[0]
            if dominant[:3] in hair and count >= 5 and sum(
                    (a - b) ** 2 for a, b in zip(pixel[:3], dominant[:3])) <= 7000:
                frame.putpixel((x, y), dominant)
    return frame


def pack(name):
    with Image.open(ASSETS / f"{name}-actions-clean-source.png") as source:
        if source.mode != "RGBA" or source.size != (1024, 1536):
            raise ValueError(f"{name}: expected a 1024x1536 RGBA sheet")
        if source.getchannel("A").getextrema()[0] != 0:
            raise ValueError(f"{name}: expected art on a transparent background")
        colors = PALETTES[name]
        palette = Image.new("P", (1, 1))
        palette.putpalette([v for rgb in colors + [colors[0]] * (256 - len(colors)) for v in rgb])
        for size in ((96, 48), (128, 64)):
            width, height = size
            frames = []
            for row in range(ROWS):
                for col in range(COLUMNS):
                    tile = source.crop((col * CELL, row * CELL,
                                        (col + 1) * CELL, (row + 1) * CELL))
                    # Fixed padding for EVERY cell protects DeepSeek's tall
                    # hair curl during the hop and matches the dragon's scale.
                    padded = Image.new("RGBA", (CELL + 32, CELL + 32))
                    padded.paste(tile, (16, 16))
                    tile = padded
                    tile = tile.resize(size, Image.Resampling.NEAREST)
                    alpha = tile.getchannel("A").point(lambda a: 255 if a >= 128 else 0)
                    tile = tile.convert("RGB").quantize(palette=palette,
                            dither=Image.Dither.NONE).convert("RGBA")
                    tile.putalpha(alpha)
                    tile.paste((0, 0, 0, 0), mask=alpha.point(lambda a: 255 - a))
                    frames.append(clean_hair_specks(tile, name))
            for index in range(1, 8):
                frames[index] = Image.composite(frames[index], frames[0],
                                                motion_mask(name, size, index))
            # Hop and think also start on the shared neutral pose.
            for index in (12, 16):
                frames[index] = frames[0].copy()
            for index in range(1, 8):
                mask = motion_mask(name, size, index).get_flattened_data()
                if any(a != b and not moving for a, b, moving in zip(
                        frames[0].get_flattened_data(), frames[index].get_flattened_data(), mask)):
                    raise ValueError(f"{name}/{size}/{index}: stationary pixels changed")
            for row in range(ROWS):
                clip = frames[row * COLUMNS:(row + 1) * COLUMNS]
                if len({f.tobytes() for f in clip}) != (2 if row == 0 else 4):
                    raise ValueError(f"{name}/{size}: missing poses in action {row}")
            for index, frame in enumerate(frames):
                if frame.size != size:
                    raise ValueError(f"{name}/{index}: unexpected dimensions {frame.size}")
                alpha = frame.getchannel("A")
                bounds = alpha.getbbox()
                if not bounds or alpha.getextrema() != (0, 255):
                    raise ValueError(f"{name}/{size}/{index}: missing art or transparency")
                if bounds[0] == 0 or bounds[1] == 0 or bounds[2] == width or bounds[3] == height:
                    raise ValueError(f"{name}/{size}/{index}: sprite touches cell edge")
            atlas = Image.new("RGBA", (COLUMNS * width, ROWS * height))
            for index, frame in enumerate(frames):
                atlas.paste(frame, (index % COLUMNS * width, index // COLUMNS * height))
            stem = ASSETS / f"{name}-actions-{width}x{height}"
            atlas.save(stem.with_suffix(".png"))
            data = b"".join(frame.tobytes() for frame in frames)
            stem.with_suffix(".rgba").write_bytes(data)
            print(f"Packed {name}: 24 frames at {width}x{height}px, {len(colors)} colors, "
                  f"{len(data):,} bytes; motion/bounds checks passed")


if __name__ == "__main__":
    for character in ("deepseek", "claude"):
        pack(character)
