# TUI showcase

English · [中文](README_cn.md)

A seven-page gallery of the public `rsille::tui` API. One application connects controls,
data widgets, content viewers, layouts, motion and themes.

## Run

From the repository root:

```bash
cargo run --manifest-path examples/tui-showcase/Cargo.toml
cargo run --manifest-path examples/tui-showcase/Cargo.toml -- --inline
```

Fullscreen is the default. A 104 × 44 terminal is recommended; smaller windows scroll.
Syntax highlighting is enabled in this project's dependency declaration.

| Page | Contents |
| --- | --- |
| Overview | Navigation, progress variants, loading indicators and dialogs |
| Controls | Unicode editors, checkbox, switch, toggle, radio group, buttons, select and calendar |
| Data | Filterable/sortable table, column visibility, multi-selection, tree, list and file explorer |
| Content | Highlighted Rust, Markdown, diffs and interaction logs |
| Layout | Flex, Grid, resizable splits, scrolling, collapsible sections, Stack and Overlay |
| Motion | Animated Braille canvas, visual effects, layout transitions and playback controls |
| Themes | Five built-in themes, semantic colors, typography and borders |

## Controls

| Key | Action |
| --- | --- |
| Tab / Shift+Tab | Move focus; arrows switch pages when navigation is focused |
| F1 / F2 / F3 | Command palette / next theme / dialog |
| Esc | Close a popup first; exit from the main page |
| Ctrl+C | Exit |
| Arrows / Space / Enter in a table | Navigate / select / submit |
| Arrows / PageUp / PageDown / Enter in a calendar | Move date / previous month / next month / submit |

Drag split separators or focus them and use arrows. Scroll with the wheel or focus the
page viewport and use arrows. Controls-page notes demonstrate a 350 ms debounce and
background save with stale-result protection. Saves are in memory only; the file
explorer and initial table data are fixtures, not disk contents.

## Verify

```bash
cargo run --manifest-path examples/tui-showcase/Cargo.toml -- --headless
cargo test --manifest-path examples/tui-showcase/Cargo.toml
cargo clippy --manifest-path examples/tui-showcase/Cargo.toml --all-targets -- -D warnings
```

Headless checks exercise pages, themes, editing, debounce, tables, the calendar, modal
focus and resizing using a manual clock and executor. Application-controlled form
values survive modal changes; remounted widgets reinitialize local cursor/scroll state.

[main.rs](src/main.rs) defines the app; [pages.rs](src/pages.rs) contains widget
compositions; [verify.rs](src/verify.rs) demonstrates input replay without a TTY.

[All examples](../README.md)
