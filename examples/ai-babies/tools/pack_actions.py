"""Bake stable clips from the original generated art; development-only Pillow.

Use one explicit palette, no dithering, binary coverage, and fixed source-space
motion masks. The idle/wave body is shared; runtime still only plays full frames.
Both supported display sizes are sampled directly from the 256px source cells.
"""
from collections import Counter
from pathlib import Path
from PIL import Image, ImageDraw

ASSETS = Path(__file__).resolve().parent.parent / "assets"
COLUMNS, ROWS = 4, 6
SIZES = (48, 64)
# Reserve iris, skin and gold colors, which an automatic frequency-based palette
# otherwise sacrifices in favor of dozens of almost identical lavender shades.
COLORS = [
    (24, 12, 40), (47, 28, 65), (80, 53, 97), (115, 88, 143),
    (150, 123, 177), (180, 156, 204), (205, 185, 225), (225, 208, 240),
    (239, 226, 250), (253, 248, 253), (255, 233, 215), (239, 199, 181),
    (229, 163, 158), (170, 86, 85), (82, 38, 128), (135, 70, 185),
    (189, 121, 224), (255, 255, 255), (255, 238, 204), (246, 199, 126),
    (211, 150, 77), (160, 103, 64), (102, 61, 50), (225, 215, 230),
]
SKIN_HIGHLIGHT = (255, 242, 225)
SKIN_COLORS = [SKIN_HIGHLIGHT, COLORS[10], COLORS[11], COLORS[12]]
# Source-space face bounds and protected eyes/mouth. These follow the authored
# poses; a fixed screen-space rectangle would miss crouching/sleeping faces.
FRONT_EYES = [(86, 104, 120, 129), (138, 104, 170, 129)]
FRONT_FACE = ((80, 92, 176, 146), FRONT_EYES)
FACE_REGIONS = [
    FRONT_FACE, FRONT_FACE, ((80, 92, 176, 146), []), FRONT_FACE,
    FRONT_FACE, FRONT_FACE,
    ((80, 92, 176, 148), [(112, 126, 148, 146)]), FRONT_FACE,
    *[((110, 94, 194, 146), [(116, 102, 146, 129), (162, 99, 190, 128)])] * 4,
    FRONT_FACE,
    ((80, 110, 176, 158), []),
    ((80, 78, 176, 134), [(110, 106, 150, 126)]),
    ((84, 114, 180, 166), []),
    FRONT_FACE, FRONT_FACE,
    ((80, 78, 176, 134), [(84, 86, 118, 115), (132, 80, 170, 111)]),
    ((84, 94, 184, 152), [(86, 102, 118, 132), (130, 108, 168, 138)]),
    ((80, 106, 176, 158), [(84, 116, 122, 140), (130, 116, 172, 140)]),
    ((82, 114, 178, 162), []),
    ((84, 130, 180, 176), []),
    ((82, 98, 178, 148), []),
]


def correct_skin(source, packed, index):
    """Map warm facial skin separately; leave glints, teeth and clothing alone."""
    bounds, protected = FACE_REGIONS[index]

    def inside(x, y, rect):
        left, top, right, bottom = rect
        return left <= x < right and top <= y < bottom

    for y in range(source.height):
        for x in range(source.width):
            sx = (2 * x + 1) * 256 // (2 * source.width)
            sy = (2 * y + 1) * 256 // (2 * source.height)
            if not inside(sx, sy, bounds) or any(inside(sx, sy, r) for r in protected):
                continue
            r, g, b, a = source.getpixel((x, y))
            # Lavender hair has a cooler hue; warm highlights and skin shadows
            # must not compete with the palette's white/lavender clothing colors.
            warm = g - b >= 3 or (r - g >= 3 and r - b >= 5)
            if a >= 128 and r >= 220 and g >= 185 and b >= 165 and r >= g and warm:
                skin = min(SKIN_COLORS, key=lambda c: sum((v - t) ** 2 for v, t in zip((r, g, b), c)))
                packed.putpixel((x, y), (*skin, 255))
    return packed
# Coordinates in a 256x256 source cell. Include the old and new arm positions
# so a raised arm replaces the old hand instead of leaving a second hand behind.
EYES = [
    [(84, 104), (120, 104), (120, 132), (84, 132)],
    [(132, 104), (168, 104), (168, 132), (132, 132)],
]
FACE = [[(84, 104), (172, 104), (172, 148), (84, 148)]]
ARM = [[(68, 140), (84, 124), (108, 124), (116, 144), (112, 188), (100, 200), (68, 200)]]


def motion_mask(size, polygons):
    mask = Image.new("L", (size, size))
    draw = ImageDraw.Draw(mask)
    for polygon in polygons:
        draw.polygon([(round(x * size / 256), round(y * size / 256)) for x, y in polygon], fill=255)
    return mask


def clean_specks(frame):
    """Remove only isolated near-color interior noise; protect face and accents."""
    original = frame.copy()
    pixels, output = original.load(), frame.load()
    size = frame.width
    for y in range(1, size - 1):
        for x in range(1, size - 1):
            # Preserve eye glints, facial lines and small gold decorations.
            if 80 <= x * 256 / size <= 176 and 100 <= y * 256 / size <= 150:
                continue
            pixel = pixels[x, y]
            if pixel[3] != 255 or pixel[:3] in COLORS[18:23]:
                continue
            neighbors = [pixels[x + dx, y + dy] for dy in (-1, 0, 1)
                         for dx in (-1, 0, 1) if dx or dy]
            if any(p[3] != 255 for p in neighbors) or pixel in neighbors:
                continue
            dominant, count = Counter(neighbors).most_common(1)[0]
            if count >= 5 and sum((a - b) ** 2 for a, b in zip(pixel[:3], dominant[:3])) <= 2500:
                output[x, y] = dominant
    return frame


def build_frames(source, size):
    palette = Image.new("P", (1, 1))
    palette.putpalette([v for rgb in COLORS + [COLORS[0]] * (256 - len(COLORS)) for v in rgb])
    frames = []
    for row in range(ROWS):
        for col in range(COLUMNS):
            tile = source.crop((col * 256, row * 256, (col + 1) * 256, (row + 1) * 256))
            tile = tile.resize((size, size), Image.Resampling.NEAREST)
            sampled = tile.copy()
            alpha = tile.getchannel("A").point(lambda a: 255 if a >= 128 else 0)
            tile = tile.convert("RGB").quantize(palette=palette, dither=Image.Dither.NONE).convert("RGBA")
            tile.putalpha(alpha)
            # Hidden RGB must not leak colors into subsequent exports/analysis.
            tile.paste((0, 0, 0, 0), mask=alpha.point(lambda a: 255 - a))
            tile = correct_skin(sampled, tile, row * COLUMNS + col)
            frames.append(clean_specks(tile))
    base = frames[0].copy()
    frames[1] = base.copy()
    frames[2] = Image.composite(frames[2], base, motion_mask(size, EYES))
    frames[3] = base.copy()
    frames[4] = base.copy()
    frames[5] = Image.composite(frames[5], base, motion_mask(size, ARM))
    frames[6] = Image.composite(frames[6], base, motion_mask(size, ARM + FACE))
    frames[7] = Image.composite(frames[7], base, motion_mask(size, ARM))
    return frames


def validate_frames(frames, size):
    allowed_colors = set(COLORS + [SKIN_HIGHLIGHT])
    for i, frame in enumerate(frames):
        visible = [p for p in frame.get_flattened_data() if p[3]]
        if not visible or not all(p[3] == 255 and p[:3] in allowed_colors for p in visible):
            raise ValueError(f"Frame {i}: invalid palette or alpha")
    base = list(frames[0].get_flattened_data())
    for i in range(1, 8):
        polygons = EYES if i == 2 else ARM + FACE if i == 6 else ARM if i in (5, 7) else []
        mask = list(motion_mask(size, polygons).get_flattened_data())
        actual = list(frames[i].get_flattened_data())
        if any(a != b and not moving for a, b, moving in zip(base, actual, mask)):
            raise ValueError(f"Frame {i}: stationary pixels changed")
    if frames[0].tobytes() == frames[2].tobytes():
        raise ValueError("Blink disappeared")
    if len({f.tobytes() for f in frames[4:8]}) != 4:
        raise ValueError("Wave poses disappeared")


def main():
    source = Image.open(ASSETS / "dragon-actions-source.png")
    if source.mode != "RGBA" or source.size != (1024, 1536):
        raise ValueError("Expected the original 1024x1536 RGBA atlas")
    for size in SIZES:
        frames = build_frames(source, size)
        validate_frames(frames, size)
        atlas = Image.new("RGBA", (COLUMNS * size, ROWS * size))
        for i, frame in enumerate(frames):
            atlas.paste(frame, ((i % COLUMNS) * size, (i // COLUMNS) * size))
        stem = ASSETS / f"dragon-actions-v2-{size}"
        atlas.save(stem.with_suffix(".png"))
        data = b"".join(frame.tobytes() for frame in frames)
        stem.with_suffix(".rgba").write_bytes(data)
        print(f"Packed {len(frames)} frames at {size}px: {len(data):,} bytes; palette/motion checks passed")


if __name__ == "__main__":
    main()
