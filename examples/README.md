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

`basic_canvas.rs` is a root-package example. Each subdirectory below is a standalone
Cargo project with a path dependency on `rsille`; these projects are excluded from
the root workspace. Run them with `--manifest-path`, using the directory name in the
table. Prefer release builds for animations and 3D scenes.

| Example | Directory | What it demonstrates |
| --- | --- | --- |
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
