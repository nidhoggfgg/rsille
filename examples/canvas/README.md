# Canvas debugging

English · [中文](README_cn.md)

Run all commands from the repository root. The example builds Kitty and Sixel by default.

```sh
cargo run --manifest-path examples/canvas/Cargo.toml -- --basic
cargo run --release --manifest-path examples/canvas/Cargo.toml
cargo run --release --manifest-path examples/canvas/Cargo.toml -- --backend kitty
cargo run --release --manifest-path examples/canvas/Cargo.toml -- --backend sixel
```

Use `--basic` to print a character Canvas sine wave. In the default fullscreen scene,
two windows show the same planet using native RasterCanvas and character Canvas.
The default `auto` prefers Kitty, then Sixel. The header
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
cargo run --manifest-path examples/canvas/Cargo.toml -- --headless
cargo test --manifest-path examples/canvas/Cargo.toml
python3 scripts/check-sixel-query.py --both-backends
```

Headless mode checks mouse interactions and visibility in memory. The Unix PTY
script checks negotiation, output and keyboard input. Use the interactive steps
above to inspect the display on the target terminal.
