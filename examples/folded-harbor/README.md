# Folded Harbor / 折叠星港

English · [中文](README_cn.md)

A perspective puzzle: rotate floating architecture until two separated doorways overlap
on screen, then fold space to cross between them. Three chapters contain nine crossings.

## Run

From the repository root:

```bash
cargo run --release --manifest-path examples/folded-harbor/Cargo.toml
cargo run --release --manifest-path examples/folded-harbor/Cargo.toml -- --demo
```

Default mode is fullscreen; `--inline` uses the normal terminal screen. Use True Color
and Braille support. Recommended size is 144 × 44; 80 × 24 is playable. The observation
panel appears at 116 × 34 and above. All scenery is generated at runtime.

## Play

Find the glowing rings marked A / DEPARTURE and B / ARRIVAL. Rotate until they overlap
and hold the view for about 0.4 seconds. When the rings turn white, press Space or click
the FOLD button. Complete three crossings to advance to the next chapter.

| Input | Action |
| --- | --- |
| Arrows / w, a, s, d / left drag | Rotate the camera |
| Wheel / + / - | Zoom |
| Space / Enter | Fold, advance chapter or restart after completion |
| h / t | Assist alignment / toggle auto-play |
| p / Esc | Pause or resume; Space also resumes |
| ? | Help; Space, Esc or ? closes it |
| r / q / Ctrl+C | Restart from chapter one / quit / quit |

Manual rotation takes over from auto-play. Folding locks camera rotation but allows
zoom. Pause and help freeze simulation. Alignment tolerance is independent of window
size and zoom. Auto-play repeats after completing the journey.

## Verify and export

```bash
cargo test --manifest-path examples/folded-harbor/Cargo.toml
cargo run --release --manifest-path examples/folded-harbor/Cargo.toml -- --headless
cargo run --release --manifest-path examples/folded-harbor/Cargo.toml -- --snapshot /tmp/harbor.svg --at 4.5
cargo run --release --manifest-path examples/folded-harbor/Cargo.toml -- --benchmark
```

`--snapshot FILE.svg --at SECONDS` captures the auto-play journey at a chosen time
(0–3600 seconds). Headless checks exercise the full journey, projected buttons, camera
input, focus and resizing. Benchmarks measure CPU update/layout/render time at 144 × 44,
excluding terminal I/O.

[world.rs](src/world.rs) implements projection and puzzle rules; [scene.rs](src/scene.rs)
and [raster.rs](src/raster.rs) draw the world; [main.rs](src/main.rs) positions ordinary
TUI labels and buttons in projected space; [verify.rs](src/verify.rs) drives checks.

[All examples](../README.md)
