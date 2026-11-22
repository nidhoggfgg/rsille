# Examples / 示例

English · [中文](README_cn.md)

Runnable applications, games and graphics built with the local `rsille` package.
For small examples of individual widgets and runtime features, use the
[TUI example guide](../packages/tui/examples/README.md).

## Run an example

All commands in these guides assume the repository root as the working directory:

```bash
cargo run --example basic_canvas
cargo run --release --manifest-path examples/lumen/Cargo.toml
```

`basic_canvas.rs` and `raster.rs` are single-file root-package examples,
run with `--example`, not standalone projects. Each subdirectory below is a
standalone Cargo project with a path dependency on `rsille`; these projects are
excluded from the root workspace. Run them with `--manifest-path`, using the
directory name in the table. Prefer release builds for animations and 3D scenes.

| Example | File / directory | What it demonstrates |
| --- | --- | --- |
| [Raster window compatibility test](raster.rs) | `raster.rs` (root example) | Check movement, occlusion, stacking, resizing and repainting with Kitty and Sixel in the same scene. |
| [Function Plotter](function-plotter/README.md) | `function-plotter` | Mathematica-style input/output cells, compact Sixel function graphs, reevaluation and scrollable history. |
| [TUI showcase](tui-showcase/README.md) | `tui-showcase` | Seven pages of controls, data, content, layout, motion and themes. |
| [Lumen](lumen/README.md) | `lumen` | Interactive workspace with flows, a notebook and floating windows. |
| [Orbit Files](orbit-files/README.md) | `orbit-files` | File manager with selection, drag and drop, previews and undo. |
| [rsille showcase](rsille-showcase/README.md) | `rsille-showcase` | Repository browser, simulated process monitor, calculator and graphics. |
| [Neon Observatory](neon-observatory/README.md) | `neon-observatory` | Six procedural Braille scenes in an inline view. |
| [Folded Harbor](folded-harbor/README.md) | `folded-harbor` | A perspective puzzle with three chapters and nine crossings. |
| [Minecraft](minecraft/README.md) | `minecraft` | A first-person voxel sandbox with mining, building and flight. |
| [Clockwork](clockwork/README.md) | `clockwork` | 132 parts assemble into three animated mechanical creatures. |
| [Pokémon Battle](pokemon-battle/README.md) | `pokemon-battle` | Pikachu versus Blastoise in an interactive 3D turn-based battle. |
| [Jellyfish](jellyfish/README.md) | `jellyfish` | A self-playing inline aquarium with two luminous jellyfish. |
| [AI Babies](ai-babies/README.md) | `ai-babies` | Three selectable sprite characters, each with six actions. |

Interactive demos need a TTY. Use a True Color terminal and a Unicode monospace font;
mouse support is needed for pointer interactions. See each guide for window sizes,
keys, data persistence and export options. Orbit Files' complete filesystem operations
require Linux; it starts in a disposable demo directory unless given `--path`.

## Function Plotter

```sh
cargo run --release --manifest-path examples/function-plotter/Cargo.toml
cargo run --manifest-path examples/function-plotter/Cargo.toml -- --help
```

No arguments starts the interactive Sixel REPL; `--help` shows help.
The terminal and any multiplexer must support Sixel,
ED 2 and local EL 2 graphics erasure, graphics following text scrolling, stable
palettes and non-scrolling positioned output. These capabilities are not
automatically detected. See the [Function Plotter guide](function-plotter/README.md)
for controls, measured cell-pixel overrides and headless checks.

## Raster window compatibility test

```sh
cargo run --release --features kitty --example raster
cargo run --release --features kitty --example raster -- --backend kitty
cargo run --release --example raster -- --backend sixel
```

Two windows show the same planet using native RasterCanvas and character Canvas.
The default `auto` prefers Kitty, then Sixel, among compiled backends. The header
shows the selected protocol and physical cell size. To compare protocols, repeat
these checks with each explicit backend on the same terminal.

1. Drag the character window over the center of the planet. Check that its text, spaces and border cover the image.
2. Move it away and click the planet window's title to raise it. Check the exposed pixels and desktop for gaps or trails.
3. Drag a bottom grip to resize, then move the window to the bottom edge. Check the planet's aspect ratio, the bottom row and unexpected scrolling.
4. Press `e` to cycle character encoding, `r` to reset the windows and `q` or Esc to exit. Tab focuses a title, arrows move it and Shift+arrows resize it.

A 100×32 cell terminal is a convenient starting size. Unix (including WSL)
probes the selected protocols even with measured `--cell-pixels WxH`. Native
Windows requires an explicit host-verified backend and measured dimensions.
Fullscreen Sixel requires ED 2 graphics erasure, stable palettes, transparent
band padding and positioned rendering without scrolling.

```sh
cargo run --example raster -- --headless
cargo test --features kitty --example raster
python3 scripts/check-sixel-query.py --both-backends
```

Headless mode checks mouse interactions and visibility in memory. The Unix PTY
script checks negotiation, output and keyboard input. Use the interactive steps
above to inspect the display on the target terminal.

## Validate without a terminal

Every standalone project except `rsille-showcase` provides `--headless`:

```bash
cargo run --release --manifest-path examples/lumen/Cargo.toml -- --headless
cargo test --manifest-path examples/lumen/Cargo.toml
cargo check --manifest-path examples/rsille-showcase/Cargo.toml --all-targets
```

Root `cargo test --workspace` does not test these standalone projects. Their own
READMEs document additional snapshots and benchmarks; flags differ between projects.
Snapshots are reconstructed from rendered terminal buffers. CPU benchmarks exclude
terminal I/O and do not measure end-to-end display frame rates.

## Add an example

Keep single-feature demos under `packages/tui/examples/` and register their targets in
`packages/tui/Cargo.toml`. Put larger applications here with `publish = false` and a
local `rsille` dependency. Include English and Chinese guides, startup commands,
controls and a reproducible verification command.

[Project README](../README.md)
