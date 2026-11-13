# AI Babies

English · [中文](README_cn.md)

Three animated sprite characters inspired by ChatGPT, DeepSeek and Claude, displayed
one at a time. Each has six actions with 24 pre-rendered frame slots: idle/blink, wave,
walk in place, hop, think and sleep. ChatGPT is selected initially; the automatic
playlist loops in about 17 seconds.

![ChatGPT character](preview-pet.gif)

| DeepSeek | Claude |
| --- | --- |
| ![DeepSeek actions](preview-deepseek.gif) | ![Claude actions](preview-claude.gif) |

## Run

From the repository root:

```bash
cargo run --release --manifest-path examples/ai-babies/Cargo.toml
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --character deepseek
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --character claude --height 24
```

All animation modes use inline output on the normal screen, without an alternate
screen or mouse capture. Default playback uses True Color and 2 × 2 Quadrant encoding. The default
height cap is 24 terminal rows; `--height ROWS` accepts only 24 or 32 and is limited by
the actual terminal during playback. An 80 × 26 terminal is recommended, allowing room for the
shell prompt. Transparent pixels use the terminal's default background.

24 rows use 96 × 48 frames; 32 rows use 128 × 64 frames, sampled directly from the
original art. Horizontal sampling is doubled while the displayed proportions stay
unchanged. Smaller windows scale down; more height adds space without enlarging the
character beyond 128 × 64 samples. Each cell fits its samples to two terminal colors;
partially transparent cells use one foreground color and the terminal background.
The legacy path embeds raw pixel data and needs no image decoder. No playback mode
needs Python or network access at runtime. The GIFs above show the existing legacy
Quadrant previews, not the new raster/Sixel output.

## Raster and Sixel

These opt-in animation modes leave default transparent Quadrant playback, static mode
and existing exports unchanged:

```bash
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --sixel
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --sixel --character claude
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --sixel --cell-pixels 8x16
```

- `--sixel` uses native inline output on the normal
  screen, with no silent fallback. No additional `--inline` argument is needed.
  Choose one rendering mode.
- `--background RRGGBB` (optional leading `#`, quoted in shell commands) applies to
  either mode and defaults to black. RGBA is composited onto this opaque background;
  neither mode preserves terminal transparency.
- `--cell-pixels WxH` is Sixel-only: supply the **actual physical character-cell size
  in pixels**, not the terminal's column/row grid. Omit it to use `CellPixels::resolve`
  startup discovery. Probing happens before input handling; metric resolution failures
  are errors. `--force-cell-pixels` is allowed only with `--sixel --cell-pixels` for a
  measured override consistent with the terminal grid. Restart after font or DPI changes.

`--sixel` is your assertion that the **whole terminal/multiplexer path** supports Sixel,
stable palettes, reliable row-local EL 2 graphics erasure without damaging other rows,
graphics following text scrolling during row reservation, safe positioned Sixel
output, and transparent partial-band padding that does not erase adjacent cells. It sets `reliable_inline_erase: true` and
`reliable_erase: false`; alternate-screen ED 2 erasure is not required or asserted.
Inline owns the full width of its occupied rows, not just the image columns.
`no_scroll` requires Sixel emission itself not to scroll; inline reserves a bottom
margin for the final band's padding rather than relying on transparency alone.
Controlled text scrolling to reserve inline rows is permitted. There is no capability auto-detection; these are
compatibility requirements that headless checks cannot verify. The current Sixel encoder uses a
fixed RGB332 256-color palette: native output improves spatial resolution, but is
not lossless 24-bit color.

Both modes use embedded PNG atlases with 320 × 320 frames for all three characters.
The existing packers generate them directly from the original 256 × 256 source cells:
this is upscaling, not new native 320-pixel detail or enlargement of the old 128 × 64
frames. The added `png` 0.18 decoder decodes and caches frames once at startup.
`--height` still accepts only 24/32 (default 24) and caps content height. Native inline
content is also limited to the rows needed for the 320-pixel image and the terminal
height remaining after blank guard rows are reserved. With enough space, actual
10×20-pixel cells use **16 content rows + 1 blank guard row**, even with `--height 24`
or `--height 32`; 8×18 cells need no extra guard row. For cell height `h`, the guard
is `ceil(((6 - h % 6) % 6) / h)` rows (five at `h = 1`). After each successful frame,
the cursor returns to the last content row, so an exit newline can use the guard.
Sixel left-aligns a native area up to
approximately 320 × 320 physical pixels; smaller terminals scale it to fit.
Raster character previews are not supported; the default Canvas mode is unchanged.

## Controls

| Key | Action |
| --- | --- |
| Tab / g / d / c | Next character / ChatGPT / DeepSeek / Claude |
| 1 / 2 / 3 / 4 / 5 / 6 | Idle / wave / walk / hop / think / sleep |
| 0 | Restart the automatic playlist |
| Space | Pause or resume |
| q / Esc / Ctrl+C | Exit |

Selecting an action restarts it. Switching characters retains the selected action and
restarts its first frame. Both operations preserve the paused state.

## Verify and export animation

```bash
cargo test --manifest-path examples/ai-babies/Cargo.toml
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --headless
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --snapshot /tmp/ai-babies.ppm 0
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --character deepseek --height 24 --frames /tmp/deepseek-frames
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --character claude --action wave /tmp/claude-wave
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --sixel --headless
```

Place `--character`, `--height` and any new rendering arguments before the export
option, in any order. Export options are alternatives:

| Option | Output |
| --- | --- |
| `--headless` | Check 90 sampled frames without a terminal |
| `--snapshot FILE.ppm [SECONDS [COLUMNS ROWS]]` | One frame; defaults to time zero; explicit dimensions override `--height` |
| `--frames DIRECTORY` | One complete playlist at 24 FPS |
| `--action idle\|wave\|walk\|hop\|think\|sleep DIRECTORY` | One action cycle at 24 FPS |

Snapshots accept 0–86400 seconds and explicit sizes of 1–512 columns by 1–128 rows.
Each PPM has a matching `.pgm` alpha mask. In the default legacy mode, PPM alone shows
transparent areas as black; this black is not painted in the terminal.

`--sixel` permits only `--headless`; raster image exports (`--snapshot`,
`--frames`, `--action`) are rejected rather than exporting placeholders.
Image exports remain available in the default Canvas mode. Headless mode tests drawing without a
terminal and does **not** certify terminal/multiplexer compatibility.

Use fresh output directories to avoid mixing old and new frames. With Pillow installed,
create previews from both files (transparent only for the legacy path):

```bash
python3 examples/ai-babies/tools/preview_actions.py /tmp/deepseek-frames --name deepseek
```

The preview script writes `preview-<name>.png` and replaces `preview-<name>.gif` in this
example directory.

Tests cover action timing, character switching, pause, resizing, transparent edges,
encoded colors and stable pixels in idle/wave animations.

## Static trio mode

![Static trio preview](preview-static.png)

```bash
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --static
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --static --height 20 --snapshot /tmp/ai-babies-static.ppm
```

`--static` must come first. This mode prints once and exits, using Quadrant encoding
and the reference image's white background. The default is 144 columns × 24 rows;
`--height` accepts 1–40, with width equal to six times the height. It uses the separate
240 × 80 RGB source, not the animation atlases.

## Assets and source

[scene.rs](src/scene.rs) selects clips and samples embedded RGBA frames;
[main.rs](src/main.rs) handles playback and input; [raster.rs](src/raster.rs) decodes
and caches high-resolution canvases; [options.rs](src/options.rs) validates rendering
options; [export.rs](src/export.rs) reconstructs
encoded terminal cells; [static_image.rs](src/static_image.rs) implements static mode.
Previews reconstruct block cells and may differ from a terminal font's seams or proportions.

| Files | Purpose |
| --- | --- |
| [chibi-trio-source.png](assets/chibi-trio-source.png) | Original three-character reference |
| [chatgpt-actions-source.png](assets/chatgpt-actions-source.png) | ChatGPT action source |
| `assets/{deepseek,claude}-actions-clean-source.png` | Companion action sources |
| `assets/*-actions-*.png` and `.rgba` (96x48/128x64) | Legacy preview atlases and embedded animation pixels |
| `assets/*-actions*-320x320.png` | Embedded high-resolution PNG atlases for raster/Sixel; no 320x320 `.rgba` files |
| `assets/chibi-trio-240x80.png` and `.rgb` | Static source and embedded pixels |
| `assets/*-prompt.txt` | Generation and cleanup prompts |

These are unofficial character interpretations. Packing uses fixed palettes and binary
alpha, with shared body pixels for idle and wave. To rebuild pixel data, install Pillow
and run the corresponding script:

```bash
python3 examples/ai-babies/tools/pack_actions.py
python3 examples/ai-babies/tools/pack_companions.py
python3 examples/ai-babies/tools/pack_static.py
```

[All examples](../README.md)
