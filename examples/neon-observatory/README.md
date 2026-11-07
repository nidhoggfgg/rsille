# Neon Observatory / 霓虹观测站

English · [中文](README_cn.md)

Six procedural scenes rendered with rsille's colored Braille canvas: a ringed planet,
photon tunnel, neon terrain, spiral galaxy, metallic torus and lakeside aurora.
Scenes are computed on the CPU without image assets or network services.

## Run

From the repository root:

```bash
cargo run --release --manifest-path examples/neon-observatory/Cargo.toml
```

The app runs inline, using at most 140 columns and 40 rows, shrinking to the terminal's
available size. Use True Color and a font with Braille glyphs. It preserves previous
terminal output and returns to the shell on exit.

## Controls

| Input | Action |
| --- | --- |
| 1–6 | Select Orbit, Warp, Terrain, Galaxy, Torus or Aurora |
| p / c / s | Pause / next palette / cycle 1×, 2× and 0.5× speed |
| ? | Toggle help |
| Tab / Shift+Tab, Enter / Space | Focus and activate playback controls |
| Mouse click | Activate scene or playback buttons |
| q / Esc | Exit |

All scenes share simulation time. Scene buttons accept clicks but skip keyboard focus;
number keys provide direct selection. Small windows may clip controls, while shortcuts
remain available.

## Verify and export

```bash
cargo test --manifest-path examples/neon-observatory/Cargo.toml
cargo run --release --manifest-path examples/neon-observatory/Cargo.toml -- --headless
cargo run --release --manifest-path examples/neon-observatory/Cargo.toml -- --snapshot /tmp/neon.svg 4 12
cargo run --release --manifest-path examples/neon-observatory/Cargo.toml -- --benchmark
```

`--snapshot FILE.svg [1-6] [SECONDS]` exports the actual 140 × 40 buffer; defaults are
scene 1 at 12 seconds. `--headless` checks playback, scenes, controls, focus, resizing
and exit. Benchmarks measure update, layout and drawing, excluding terminal I/O.

[scenes.rs](src/scenes.rs) defines scene geometry and formulas; [raster.rs](src/raster.rs)
accumulates light, bloom and exposure; [main.rs](src/main.rs) connects the TUI and checks.
The demo implements the 3D scenes; rsille supplies canvas encoding, layout and input.
Colors and aspect ratio depend on the terminal and font.

[All examples](../README.md)
