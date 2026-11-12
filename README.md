# rsille

English · [中文](README_cn.md)

<img align="right" src="examples/ai-babies/preview-deepseek.gif" width="96" alt="">

**Make your terminal come alive.**

Draw a glowing ocean, bring a mechanical creature to life, or combine draggable
windows and 3D scenes in one terminal. **rsille is a Rust framework for terminal applications and
graphics**, bringing rendering, TUI, animation and interaction into one composable API.

## One terminal. Many possibilities.

Click a preview for source code, run commands and controls.

| Jellyfish · An ocean in your terminal | Lumen · A workspace you can move around |
| --- | --- |
| [![Glowing jellyfish, light shafts and drifting particles in a terminal](examples/jellyfish/preview.gif)](examples/jellyfish/README.md) | [![Lumen's 3D canvas, live curves and draggable floating windows](examples/lumen/preview-desk.png)](examples/lumen/README.md) |
| **Rendering.** Full-color samples, half-block encoding and continuous redraw bring procedural jellyfish, soft light shafts and drifting particles to life. | **TUI.** Canvas graphics and native widgets share a layout. Drag, resize, collapse and restore windows, with focus management and a command palette. |

| Clockwork · Parts that assemble themselves | Pokémon Battle · Take your turn |
| --- | --- |
| [![132 parts disassembling and rebuilding into mechanical creatures](examples/clockwork/preview.gif)](examples/clockwork/README.md) | [![Pikachu, Blastoise and native move buttons in a 3D arena](examples/pokemon-battle/preview.png)](examples/pokemon-battle/README.md) |
| **Animation.** The example uses springs to drive 132 parts between three forms; rsille handles frame rendering and input. Orbit the camera while the machine rebuilds. | **Graphics × TUI × interaction.** A 3D scene with native move buttons, health bars and frosted backdrops. Click to attack, drag to orbit and watch the action unfold. |

Previews are exported from rendered terminal cells; Clockwork's GIF is sped up.
The examples supply scenes and simulation; rsille handles the canvas,
encoding, widgets, rendering and event loop.

## Big visuals. Small building blocks.

<img align="right" src="examples/ai-babies/preview-pet.gif" width="96" alt="">

- **Put effects on your widgets.** Wrap a button, panel or canvas in `visual(...)`.
  Chain gradients, light sweeps, dissolves and waves; add a frosted backdrop or an
  enter/exit transition. The runtime drives the animation.
- **Draw a world inside your UI.** Use Braille, half blocks or quadrants for colored
  graphics. Embed a `canvas(...)` in a layout, with world coordinates and matching
  pointer transforms, or use the renderer directly for a whole scene.
- **Keep the application code simple.** Describe the UI in `view`, change state in
  `update`. Compose Flex/Grid layouts, text editors, tables and draggable windows;
  let the runtime handle focus, input, background tasks and cleanup.

Explore the **[example gallery](examples/README.md)**: a
[voxel sandbox](examples/minecraft/README.md),
[a file manager with drag and drop](examples/orbit-files/README.md) and
[a seven-page widget showcase](examples/tui-showcase/README.md).
For one API at a time, start with the [small TUI examples](packages/tui/examples/README.md).

## Add rsille to an application

Add rsille `3.0.4` to your application:

```toml
[dependencies]
rsille = "3.0.4"
```

The public package is `rsille`. The repository's `tui`, `render` and `canvas` crates
are private development targets, all with `publish = false`.

For Typst math → RGB raster rendering, see the independent
[`rsille-typst` extension](packages/rsille-typst/README.md). It uses Typst math
syntax directly and does not add Typst dependencies to the core crate. Display a
block formula inline on the normal screen, without mouse capture, in a compatible
terminal (`q` / `Esc` to quit). Original pixel size is preserved; narrow/short
viewports clip rather than enlarge or reflow the formula:

```sh
cargo run -p rsille-typst --features sixel --example formula -- 'sqrt(x)'
cargo run -p rsille-typst --example formula -- --help
```

No arguments starts the interactive default formula; `--help` shows help.
The terminal and any multiplexer must support
Sixel, local EL 2 graphics erasure, graphics following text scrolling, stable
palettes, non-scrolling positioned output and transparent partial-band padding.
These capabilities are not automatically detected, and output never silently
falls back. See the extension README for measured cell-pixel overrides
and no-TTY `--headless` checks.

Default features are empty. Enable `syntax-highlighting` for Syntect highlighting in
code viewers:

```toml
[dependencies]
rsille = { version = "3.0.4", features = ["syntax-highlighting"] }
```

Without that feature, code viewers still support plain text and navigation.

For local development, replace the registry dependency with `rsille = { path = "../rsille" }`,
adjusting the path to your checkout.

## Opaque raster drawing

`rsille::raster::RasterCanvas` provides fixed-resolution RGB pixels, lines,
rectangles and circles, independently of the existing sample-based `Canvas`.
Embed it with `raster_canvas(...)` or use revision-cached
`RasterCanvasWidget::from_draw(...)`. Layout, clipping, scrolling and opaque
coverage retain raster metadata, including in headless frames. Raster terminal
output is Sixel-only; existing Canvas character encodings are unchanged.

The optional `sixel` feature enables native encoding. `RasterOutput` contains
only host-supplied capabilities; its default asserts no Sixel support. Real raster
output fails when safety conditions are missing, never falling back to characters.
The renderer never discovers cell pixels or reads terminal input. Native output
requires explicit capabilities and valid physical cell metrics: fullscreen uses
an alternate screen and `reliable_erase`; inline uses `reliable_inline_erase`.
**Automatic capability negotiation and real-terminal certification are not yet
implemented; enabling the feature alone does not enable native output.**
There is no alpha/transparency API or `Encoding::Sixel`.

See the [raster guide](docs/raster.md) for configuration and limits, and the
[design/validation notes](docs/raster-design.md) for remaining work.

Compare the same static procedural ringed planet in two draggable windows: native
RGB raster output versus existing `Canvas` character sampling. This is the root-package,
single-file [`sixel_windows`](examples/sixel_windows.rs) demo, not a standalone project:

```sh
cargo run --release --features sixel --example sixel_windows
```

The whole terminal/multiplexer path must support Sixel, graphics ED 2 erasure,
non-scrolling positioned output and stable palettes. The demo configures these
capabilities without automatically detecting them; unsupported output returns an error. Before starting **any input reader**, call `CellPixels::probe(timeout)`
and store the result as `RasterCapabilities.cell_pixels: Some(pixels)`. The probe
tries OS window metrics first, then a bounded `CSI 16 t` query on Unix/WSL with
stdin/stdout TTYs attached to the controlling terminal. It returns on reply; the
timeout is configurable, not a guaranteed elapsed time. `resolve(None)` retains
a 500 ms timeout for compatibility; `resolve(Some(pixels))` only validates.
Native Windows has no query fallback. Optional `--cell-pixels 8x18` bypasses
probing and must match **actual font metrics** (1..=1024 per dimension, at most
65,536 pixels). With no stored pixels, real raster rendering errors; frames without raster
content are unaffected.

Probing is permanently rejected after managed input initialization; external
readers cannot be detected, so standalone hosts must probe before starting theirs.
The probe may consume unrelated input; late replies can arrive after timeout,
and no input is flushed. Native crossterm `EventStream` handles all managed runtime
input. Stored pixels are a startup snapshot, not refreshed on font/DPI/resize
changes; update them explicitly or restart. `CellPixels::from_terminal()` remains
an optional OS-only read usable at runtime. Probing does not establish Sixel support.
Actual 10×20 cells are supported with `--cell-pixels 10x20`, without rounding the
height to 18 or 24. The terminal must honor transparent partial-band padding as
well as the erase/no-scroll assertions; this is not real-terminal certification.
Drag titles to move, drag bottom grips to resize, and click to bring a window forward.
`e` cycles Canvas Braille/HalfBlock/Quadrant, `r` restores starting rectangles, and
`q`/Esc quits. Prefer 100×32 cells; smaller sizes and terminal resizing are supported.
There is no live animation. No arguments starts the interactive demo; `--help`
shows help. `--headless` runs finite terminal-independent drag/occlusion
checks. See the [showcase guide](docs/raster.md#run-the-showcase) for validation commands.

## Develop with an AI coding agent

The [rsille skill](https://github.com/nidhoggfgg/rsille.skills/blob/main/rsille/SKILL.md) teaches coding agents the v3 API,
including widgets and layouts, background tasks, input handling, Canvas graphics,
animation, and headless testing. Its [component catalog](https://github.com/nidhoggfgg/rsille.skills/blob/main/rsille/references/components.md)
maps UI requirements to built-in components and examples. It also includes a
standalone application starter.

Install it from your application directory using the
[Skills CLI](https://github.com/vercel-labs/skills):

```bash
npx skills add nidhoggfgg/rsille.skills --skill rsille
```

Choose your coding agent when prompted; add `-g` to install across projects.
For a local checkout, replace `nidhoggfgg/rsille.skills` with the skills repository's
absolute path. This repository includes it as the `skills/` Git submodule; run
`git submodule update --init skills` to fetch it. You can also copy the complete
`skills/rsille` directory into your agent's skills directory.

Then ask your agent, for example:

> Use the rsille skill to build a terminal task manager with a filter input,
> keyboard navigation, and headless tests.

## Your first application, already animated

```rust,no_run
use std::time::Duration;
use rsille::tui::prelude::*;

fn main() -> WidgetResult<()> {
    let exit = App::new(0_u32)
        .mode(Mode::Inline)
        .update(|count, (), _ctx| *count += 1)
        .view(|count| {
            visual(
                button(format!("Count: {count}"))
                    .variant(ButtonVariant::Ghost)
                    .on_click(|| ()),
            )
                .gradient(
                    Color::Rgb(70, 220, 255),
                    Color::Rgb(240, 110, 255),
                    GradientDirection::Horizontal,
                )
                .highlight_sweep()
                .animation(looping_visual_spec(Duration::from_secs(2)))
        })
        .run()?;

    println!("Final count: {}", exit.state);
    Ok(())
}
```

<img src="imgs/readme-counter-en.gif" width="424" alt="Gradient counter button with a looping light sweep and Enter-driven count changes">

Exported from the code above at 32 × 3 terminal cells, replaying Enter to count from 0 to 2.

<img align="right" src="examples/ai-babies/preview-claude.gif" width="96" alt="">

A clickable counter with a cyan-to-magenta gradient and a looping light sweep.
The three chained calls add the look and motion; no manual frame loop is needed.
`visual(...)` wraps the interactive button, so it keeps its input behavior.

Tab moves focus, Enter or Space activates the button, and Esc exits by default.
`view` describes widgets from state; their messages reach `update`. The third update
argument, `UpdateCtx`, schedules work, emits messages and requests exit with
`ctx.exit()`.

Configure themes, hotkeys, subscriptions and timing before `.update()`. Fullscreen is
the default; `Mode::Inline` uses the normal terminal screen. `run()` returns an
`AppExit` containing the final state and exit reason after cleanup. Inside an existing
Tokio runtime with the time driver enabled, use `.run_async().await`.

## Test without a terminal

The same builder can produce an in-memory runtime:

```rust
use rsille::tui::prelude::*;

let mut runtime = App::new(0_u32)
    .update(|count, (), _ctx| *count += 1)
    .view(|count| button(format!("Count: {count}")).on_click(|| ()))
    .build()
    .size(40, 3);

runtime.handle().send(()).unwrap();
runtime.step(&[]).unwrap();
assert_eq!(*runtime.state(), 1);
assert_eq!(runtime.frame().size().width, 40);
```

Use `step(&events)` to replay input and `frame()` to inspect rendered cells.
`ManualClock` and `ManualExecutor`, imported from `rsille::tui::runtime`, make timers
and queued jobs controllable. The [workflow example](packages/tui/examples/basics/workflow.rs)
checks Unicode editing, debounce, background saving and exit using one application:

```bash
cargo run -p tui --example workflow -- --headless
```

## Draw with Canvas

**More detail in every character cell.** `Encoding::Quadrant` divides each terminal
cell into **2 × 2 sample positions**. Quadrant characters combine foreground and
background colors to draw finer shapes within a small terminal area.
120 × 20 cells give you a 240 × 40 sample grid.

[![Pixel characters rendered with 2 × 2 quadrant encoding in 120 × 20 terminal cells](imgs/readme-canvas-quadrant.png)](examples/ai-babies/README.md)

Actual Canvas output from [AI Babies' static mode](examples/ai-babies/README.md),
using just **20 terminal rows**. The preview is enlarged with terminal cell proportions
preserved so you can see the detail.

```bash
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --static --height 20
```

Use the renderer directly when a widget tree is unnecessary:

```rust,no_run
use rsille::{canvas::Canvas, render};

fn main() -> std::io::Result<()> {
    let mut canvas = Canvas::new();
    for x in 0..160 {
        let x = x as f64;
        canvas.set(x, 15.0 + (x / 10.0).sin() * 10.0);
    }
    render::Builder::new()
        .size((80, 10))
        .append_newline(true)
        .build_render(canvas, std::io::stdout())
        .render()
}
```

| Encoding | Samples per terminal cell | Use |
| --- | --- | --- |
| `Encoding::Braille` (default) | 2 × 4 | Dense lines, curves and colored scenes |
| `Encoding::HalfBlock` | 1 × 2 | Two independently colored vertical samples |
| `Encoding::Quadrant` | 2 × 2 | Quadrant shapes, with samples fitted to terminal colors |

Select an encoding with `Canvas::with_encoding(Encoding::Quadrant)`. Drawing coordinates are samples,
with y increasing upward; bounds and output sizes are terminal cells. `set_color`
colors samples and `set_background` fills uncovered areas. Terminal cells have at most
one foreground and one background color, so Braille and quadrant encoding may combine
sample colors.

The TUI `canvas(...)` widget embeds the same graphics in layouts. Its `.viewport(...)`
and `ctx.world(surface)` support world coordinates and matching pointer transforms.
Compare encodings with `cargo run -p tui --example canvas`.

## API and source map

This checkout contains the **v3 API** (`3.0.4` in `Cargo.toml`).

| Public module | Responsibility | Shared implementation |
| --- | --- | --- |
| `rsille::tui` | Applications, widgets, layout, input and effects | [src/tui](src/tui) |
| `rsille::render` | Buffers, styled Unicode text, drawing and terminal loops | [src/render](src/render) |
| `rsille::canvas` | Sample graphics and terminal encoding | [src/canvas](src/canvas) |

The private crates under `packages/` compile these same sources for separate tests and
examples. There is no duplicated implementation. Generate API documentation locally:

```bash
cargo doc -p rsille --all-features --no-deps --open
```

Text editing uses grapheme clusters; glyph widths and appearance still depend on the
terminal and font. Held-key applications can opt into `KeyboardMode::Auto` and inspect
`InputEvent::Capabilities`; accurate key releases and independent simultaneous holds
require terminal support. See [Minecraft's input notes](examples/minecraft/README.md).

## Development

```bash
cargo test --workspace
cargo test --workspace --all-features
cargo check --workspace --all-targets --all-features
```

Standalone projects in `examples/` are excluded from the root workspace. Run their
checks separately using `--manifest-path`; commands are in each example's README.

`python3 scripts/check-release.py --offline` checks the self-contained package archive
and an independent consumer without uploading anything. It requires Python 3.12+ and
cached dependencies; omit `--offline` to allow dependency downloads.

[Example gallery](examples/README.md) · [TUI examples](packages/tui/examples/README.md) ·
[MIT license](LICENCE)
