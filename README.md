# rsille

English · [中文](README_cn.md)

A Rust framework for terminal applications and graphics. Build interactive tools with
widgets, layouts and background tasks, or draw directly into terminal cells with the
renderer and sample canvas.

This checkout contains the **v3 API** (`3.0.0` in `Cargo.toml`). For changes from v2,
see the [migration guide](MIGRATION_V3.md).

![Lumen, a workspace built with rsille](examples/lumen/preview.png)

## What you can build

- **Applications:** forms, editable Unicode text, tables, trees, file browsers,
  command palettes and content viewers.
- **Interactive layouts:** Flex and Grid, scrolling, resizable splits, modal portals,
  draggable windows, pointer capture and keyboard focus.
- **Motion and graphics:** transitions, visual effects, frosted backdrops and colored
  Canvas samples encoded as Braille, half blocks or quadrants.
- **Testable workflows:** messages, background tasks, debounce, scoped subscriptions
  and a runtime you can drive without a terminal.

The [example gallery](examples/README.md) includes working tools, games and animations.
Start with [Lumen](examples/lumen/README.md) for an application,
[TUI showcase](examples/tui-showcase/README.md) for widgets, or
[the small TUI examples](packages/tui/examples/README.md) for individual APIs.

## Try it

Run these commands from the repository root with a current stable Rust toolchain:

```bash
cargo run -p tui --example hello
cargo run --release --manifest-path examples/lumen/Cargo.toml
cargo run --release --manifest-path examples/pokemon-battle/Cargo.toml
```

Interactive examples need a terminal. For graphics, use a Unicode monospace font and
True Color support; each example documents its preferred window size and controls.
A first run compiles dependencies. Use `--release` for the graphics demos.

## Add rsille to an application

To use this checkout from another project, add a path dependency, replacing the path
with the location of your clone:

```toml
[dependencies]
rsille = { path = "../rsille" }
```

The public package is `rsille`. The repository's `tui`, `render` and `canvas` crates
are private development targets, all with `publish = false`.

Default features are empty. Enable `syntax-highlighting` for Syntect highlighting in
code viewers:

```toml
[dependencies]
rsille = { path = "../rsille", features = ["syntax-highlighting"] }
```

Without that feature, code viewers still support plain text and navigation.

## Your first application

```rust,no_run
use rsille::tui::prelude::*;

fn main() -> WidgetResult<()> {
    let exit = App::new(0_u32)
        .mode(Mode::Inline)
        .update(|count, (), _ctx| *count += 1)
        .view(|count| button(format!("Count: {count}")).on_click(|| ()))
        .run()?;

    println!("Final count: {}", exit.state);
    Ok(())
}
```

Tab moves focus, Enter or Space activates the button, and Esc exits by default.
`view` describes widgets from state; their messages reach `update`. The third update
argument, `UpdateCtx`, schedules work, emits messages and requests exit with
`ctx.exit()`.

Configure themes, hotkeys, subscriptions and timing before `.update()`. Fullscreen is
the default; `Mode::Inline` uses the normal terminal screen. `run()` returns an
`AppExit` containing the final state and exit reason after cleanup. Inside an existing
Tokio runtime with I/O and time enabled, use `.run_async().await`.

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

Select an encoding with `Canvas::with_encoding(...)`. Drawing coordinates are samples,
with y increasing upward; bounds and output sizes are terminal cells. `set_color`
colors samples and `set_background` fills uncovered areas. Terminal cells have at most
one foreground and one background color, so Braille and quadrant encoding may combine
sample colors.

The TUI `canvas(...)` widget embeds the same graphics in layouts. Its `.viewport(...)`
and `ctx.world(surface)` support world coordinates and matching pointer transforms.
Compare encodings with `cargo run -p tui --example canvas`.

## API and source map

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
[Migration guide](MIGRATION_V3.md) · [MIT license](LICENCE)
