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

`basic_canvas.rs` and `sixel_windows.rs` are single-file root-package examples,
run with `--example`, not standalone projects. Each subdirectory below is a
standalone Cargo project with a path dependency on `rsille`; these projects are
excluded from the root workspace. Run them with `--manifest-path`, using the
directory name in the table. Prefer release builds for animations and 3D scenes.

| Example | File / directory | What it demonstrates |
| --- | --- | --- |
| [Sixel vs Canvas windows](../docs/raster.md#run-the-showcase) | `sixel_windows.rs` (root example) | Two draggable, resizable windows showing the same static ringed planet as native RGB raster and Canvas character sampling. |
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

## Sixel vs Canvas windows

```sh
cargo run --release --features sixel --example sixel_windows -- --sixel
cargo run --release --example sixel_windows -- --text-only
cargo run --example sixel_windows -- --help
```

Two `floating_window` widgets in a `window_host` show the same procedural ringed
planet, without live animation. Drag a title to move, drag a bottom grip to resize,
and click to bring a window forward. `e` cycles Canvas Braille/HalfBlock/Quadrant;
`r` restores the starting window rectangles; `q`/Esc quits. Ideally use 100×32 cells;
smaller terminals and terminal resizing are supported.

`--sixel` is your explicit assertion that the entire terminal/multiplexer path
supports Sixel, graphics ED 2 erasure, no scrolling and stable palettes. It uses
`RequireSixel`, with no silent fallback. Startup probing uses OS window metrics first;
unusable OS dimensions trigger a `CSI 16 t` query on Unix (including WSL), requiring
stdin/stdout TTYs attached to the controlling terminal and a 500 ms absolute timeout.
Native Windows has no query fallback and still needs explicit dimensions. Optional
`--cell-pixels 8x18` skips both sources and must match **actual font metrics**, not
an arbitrary size. Measured dimensions are stored as a startup snapshot and are
not refreshed on resize; restart after font or DPI changes. Probing precedes all
input readers and may consume unrelated input. Runtime input uses native crossterm.
This query does not negotiate Sixel support. `--text-only` sends no cell-pixel query.
The feature and backend limits (1..=1024 per dimension, at most 65,536 pixels)
are checked before entering the managed terminal session. Actual 10×20 cells are supported with
`--cell-pixels 10x20`; do not round the height to 18 or 24. The terminal must honor
transparent partial-band padding and the erase/no-scroll assertions; accepting
these metrics does not certify real-terminal behavior.
No flags shows help, not a fallback demo;
`--text-only` explicitly selects a character preview. See the
[raster guide](../docs/raster.md) for the full safety contract.

Finite, terminal-independent mouse-drag and occlusion checks and example tests:

```sh
cargo run --example sixel_windows -- --headless
cargo test --example sixel_windows --features sixel
cargo test --example sixel_windows --no-default-features
```

These do not certify real-terminal graphics behavior or measure performance.

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
