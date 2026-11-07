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

Animated playback uses inline mode, True Color and Half-block encoding. The default
height cap is 32 terminal rows; `--height ROWS` accepts 1–128 and is limited by the actual
terminal during playback. An 80 × 34 terminal is recommended, allowing room for the
shell prompt. Transparent pixels use the terminal's default background.

24 rows use the 48-pixel atlas; 32 rows use the 64-pixel atlas. Smaller windows scale
down; more height adds space without enlarging the character beyond 64 × 64 pixels.
The binary embeds the pixel data, so playback needs no network, Python or image decoder.

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
```

Place `--character` and `--height` before an export option; they may appear in either
order. Export options are alternatives:

| Option | Output |
| --- | --- |
| `--headless` | Check 90 sampled frames without a terminal |
| `--snapshot FILE.ppm [SECONDS [COLUMNS ROWS]]` | One frame; defaults to time zero; explicit dimensions override `--height` |
| `--frames DIRECTORY` | One complete playlist at 24 FPS |
| `--action idle\|wave\|walk\|hop\|think\|sleep DIRECTORY` | One action cycle at 24 FPS |

Snapshots accept 0–86400 seconds and explicit sizes of 1–512 columns by 1–128 rows.
Each PPM has a matching `.pgm` alpha mask. PPM alone shows transparent areas as black;
this black is not painted in the terminal. Use fresh output directories to avoid mixing
old and new frames. With Pillow installed, create transparent previews from both files:

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
[main.rs](src/main.rs) handles playback and input; [export.rs](src/export.rs) reconstructs
encoded terminal cells; [static_image.rs](src/static_image.rs) implements static mode.
Previews reconstruct block cells and may differ from a terminal font's seams or proportions.

| Files | Purpose |
| --- | --- |
| [chibi-trio-source.png](assets/chibi-trio-source.png) | Original three-character reference |
| [dragon-actions-source.png](assets/dragon-actions-source.png) | ChatGPT action source |
| `assets/{deepseek,claude}-actions-clean-source.png` | Companion action sources |
| `assets/*-actions-*.png` and `.rgba` (48/64) | Current preview atlases and embedded animation pixels |
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
