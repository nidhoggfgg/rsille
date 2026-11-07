"""Preview actual --frames exports; optional side-by-side idle/wave comparison."""
import argparse
from pathlib import Path
from PIL import Image, ImageDraw


def save_gif(images, path):
    options = {}
    if images[0].mode == "RGBA":
        indexed = []
        for image in images:
            # Reserve an explicit palette index for alpha, keeping opaque white
            # clothes/eye glints separate from the transparent canvas.
            frame = image.convert("RGB").quantize(colors=255, dither=Image.Dither.NONE)
            palette = frame.getpalette()
            palette += [0] * (768 - len(palette))
            frame.putpalette(palette)
            frame.paste(255, mask=image.getchannel("A").point(lambda a: 255 if a == 0 else 0))
            indexed.append(frame)
        images = indexed
        options = {"transparency": 255, "background": 255, "optimize": False}
    # GIF timestamps have 10ms resolution; six frames total 250ms = 24 FPS.
    images[0].save(path, save_all=True, append_images=images[1:],
                   duration=[50 if i % 6 == 5 else 40 for i in range(len(images))],
                   loop=0, disposal=2, **options)


def read_frame(path):
    with Image.open(path) as image:
        frame = image.convert("RGB")
    mask_path = path.with_suffix(".pgm")
    if mask_path.exists():
        with Image.open(mask_path) as mask:
            if mask.size != frame.size:
                raise ValueError(f"Alpha mask size mismatch: {mask_path}")
            frame.putalpha(mask.convert("L"))
    return frame


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("frames", type=Path)
    parser.add_argument("--compare", type=Path, help="older --frames export directory")
    args = parser.parse_args()
    files = sorted(args.frames.glob("*.ppm"))
    if not files:
        raise SystemExit("No exported PPM frames found")
    images = []
    for path in files:
        frame = read_frame(path)
        images.append(frame.resize((frame.width * 4, frame.height * 4), Image.Resampling.NEAREST))
    output = Path(__file__).resolve().parent.parent
    images[0].save(output / "preview-pet.png")
    save_gif(images, output / "preview-pet.gif")
    if args.compare:
        # First 4.4 seconds: idle + wave, the two clips with frozen body regions.
        if len(files) < 106:
            raise SystemExit("Comparison requires a full playlist export")
        comparison = []
        for i, path in enumerate(files[:106]):
            before, after = read_frame(args.compare / path.name), read_frame(path)
            if before.size != after.size:
                raise SystemExit("Comparison exports must have the same terminal size")
            size = (before.width * 3, before.height * 3)
            panel = Image.new("RGB", (size[0] * 2, size[1] + 24), (250, 248, 253))
            for image, x in [(before, 0), (after, size[0])]:
                image = image.resize(size, Image.Resampling.NEAREST)
                panel.paste(image, (x, 24), image.getchannel("A") if image.mode == "RGBA" else None)
            draw = ImageDraw.Draw(panel)
            draw.text((12, 6), "BEFORE", fill=(47, 28, 65))
            draw.text((size[0] + 12, 6), "AFTER - stable body / shared palette", fill=(47, 28, 65))
            comparison.append(panel)
        comparison[0].save(output / "preview-pet-comparison.png")
        save_gif(comparison, output / "preview-pet-comparison.gif")
    print(f"Saved terminal previews from {len(images)} frames to {output}")


if __name__ == "__main__":
    main()
