# rsille showcase

English · [中文](README_cn.md)

Four terminal surfaces in one application: a repository workbench, a simulated process
monitor, a calculator and animated Braille graphics.

## Run

Run from the repository root so the workbench can find the expected files:

```bash
cargo run --release --manifest-path examples/rsille-showcase/Cargo.toml
```

The project uses the local `rsille` package with syntax highlighting enabled.

| Tab | What is real and what is simulated |
| --- | --- |
| Workbench | Reads local project files; shows source previews, a generated demonstration diff, notes and logs |
| Top | Filters and selects simulated process rows; metrics and timelines are synthetic |
| Calculator | Evaluates arithmetic expressions from an input or clickable keypad and keeps session history |
| Showoff | Draws particle tunnels, wireframes and signal fields on a Braille canvas |

The workbench starts from `Cargo.toml`, `README.md`, `src`, `packages` and `examples`
relative to the working directory. The tree is deliberately bounded and source
previews show at most 120 lines. The displayed diff is not a Git working-tree diff.

## Controls

Tab moves focus. Arrow keys navigate tabs, trees, lists and tables. Enter or Space
activates focused controls. The Cmd button opens the command palette. Esc exits.
Use the on-screen controls to change scenes, filter data and pause the live display.

## Check and read the source

```bash
cargo check --manifest-path examples/rsille-showcase/Cargo.toml --all-targets
```

This demo has no `--headless` or snapshot command. Its state, widget composition,
calculator and drawing routines are in [src/main.rs](src/main.rs). For an application
with automated interaction replay, see [Lumen](../lumen/README.md) or
[TUI showcase](../tui-showcase/README.md).

[All examples](../README.md)
