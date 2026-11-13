# rsille-typst

[English](README.md) | [简体中文](README_cn.md)

[API docs](https://docs.rs/rsille-typst) · [crates.io](https://crates.io/crates/rsille-typst) · [Repository](https://github.com/nidhoggfgg/rsille)

Render **Typst math into an opaque RGB `RasterCanvas`**, ready for rsille's raster widget. Uses the real Typst compiler and `typst-render` in-process, without a CLI subprocess or LaTeX conversion.

Supply the **contents of math mode**: `x^2`, `frac(1, 2)`, `sqrt(x)`, or `sum_(k=1)^n k`. Do not wrap the input in `$ ... $`; LaTeX such as `\frac{1}{2}` is not supported.

## Quickstart

Add these dependencies to your application's `Cargo.toml`:

```toml
[dependencies]
rsille-typst = "0.1.0"
# Optional font source used by the example below; you can supply your own fonts.
typst-assets = { version = "=0.13.1", features = ["fonts"] }
```

```rust
use rsille_typst::{MathRenderer, RenderOptions, Rgb};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = MathRenderer::new(
        typst_assets::fonts(),
        "New Computer Modern Math",
    )?;
    let options = RenderOptions {
        font_size: 20.0,
        foreground: Rgb(230, 230, 230),
        background: Rgb(24, 24, 24),
        max_pixels: 4 * 1024 * 1024,
        ..RenderOptions::default()
    };
    let result = renderer.render_with_diagnostics(
        r"integral_0^1 x^2 dif x = frac(1, 3)",
        &options,
    )?;
    for warning in result.warnings {
        eprintln!("Typst warning: {}", warning.message);
    }
    let canvas = result.canvas;
    println!("{} × {} pixels", canvas.width(), canvas.height());
    Ok(())
}
```

This creates pixels, not terminal output. No TTY is needed. Reuse the renderer for its loaded fonts, and retain the resulting canvas rather than recompiling on every UI frame.

### Features and versions

The 0.1.0 release depends on `rsille` **3.1.0** with default features enabled. This extension defines no Cargo features; Sixel encoding is always available. The core does not depend on Typst.

`typst` and `typst-render` are pinned to **0.13.1**; use matching `typst-assets = "=0.13.1"` if choosing that font source. Keep your application's lockfile to pin transitive dependencies.

## Integrate with rsille

For terminal output, add a direct core dependency:

```toml
[dependencies]
rsille-typst = "0.1.0"
rsille = "3.1.0"
```

This helper accepts the rendered canvas and the actual backend's validated cell metrics:

```rust
use rsille::tui::prelude::*;

fn formula_widget(
    canvas: RasterCanvas,
    cell_pixels: CellPixels,
) -> Result<impl Widget<()>, Box<dyn std::error::Error>> {
    let size = cell_pixels
        .size_for_pixels(canvas.width(), canvas.height())
        .ok_or("invalid cell metrics or terminal size overflow")?;
    Ok(raster_canvas::<()>(canvas)
        .scale(RasterScale::Original)
        .fixed(size.width, size.height))
}
```

Attach the widget to your existing `App` and explicitly configure its `RasterOutput` capabilities; see the [complete demo](examples/formula.rs). Lower-level drawing is also available through `Chunk::raster` with `RasterOptions`.

Pixel dimensions are not terminal columns and rows. `Original` preserves the image's pixel size and permits clipping; use `Fit` when scaling is desired. Generating RGB pixels and inspecting headless raster metadata do not perform Sixel encoding. Real terminal raster output supports **Sixel only**: missing capabilities or invalid metrics produce errors, not a text/character fallback.

## Run the demo

From the repository root:

```sh
cargo run -p rsille-typst --example formula -- --help
cargo run -p rsille-typst --example formula
cargo run -p rsille-typst --example formula -- 'sum_(k=1)^n k = frac(n(n+1), 2)'
# No TTY: layout/metadata checks, without Sixel encoding.
cargo run -p rsille-typst --example formula -- --headless
# No TTY: additionally exercise Sixel encoding into memory.
cargo run -p rsille-typst --example formula -- --headless --cell-pixels 10x20
```

The interactive demo compiles once and displays a block equation inline on the normal screen. It does not use an alternate screen or capture the mouse; **q / Esc** exits. Pass one quoted formula; put formulas beginning with `-` after `--`.

It uses `RasterScale::Original`, reserving the formula's rows plus a title, with the framework's 50-row inline content cap and additional bottom guards. Small terminals clip rather than scale or reflow; enlarging the terminal reveals the image without recompilation. Very large formulas may not fit.

### Terminal compatibility and cell size

The demo assumes that **both the terminal and any multiplexer** support Sixel, local EL 2 graphics erasure, images scrolling with text, stable palettes, non-scrolling positioned output, and transparent padding in partial final six-pixel bands. It sets `reliable_inline_erase: true`, not full-screen `reliable_erase`. **Capabilities are not automatically detected.** See [`RasterCapabilities`](https://docs.rs/rsille/3.1.0/rsille/render/raster_output/struct.RasterCapabilities.html) for the contract.

Without an override, `CellPixels::resolve(None)` probes before the input reader starts: OS metrics first, then a Unix (including WSL) CSI 16 t query with a 500 ms timeout. Queries require controlling-terminal stdin/stdout TTYs; native Windows has no query fallback. If discovery fails, supply `--cell-pixels WxH` using **measured physical pixels per character cell**, not columns × rows, window pixels, or font points. For example, a 960×648 text area with 120 columns and 36 rows has 8×18 pixels per cell.

- An explicit override skips all probing. Each dimension must be `1..=1024`, with area at most 65536. An override matching the terminal grid additionally requires `--force-cell-pixels`.
- Metrics are captured at startup; restart after changing font or DPI. Queries do not detect Sixel support, may consume concurrent keystrokes, and late responses may reach the input reader.
- Inline Sixel padding can reserve blank guard rows, reducing usable height: 10×20 cells need one extra row; 8×18 cells need none.

`--headless` never starts terminal I/O or an input reader and never probes. Alone, it checks raster metadata using synthetic 10×20 layout metrics. Adding `--cell-pixels` selects real Sixel encoding into memory, which is always available without feature flags; invalid metrics fail rather than falling back to metadata checks. Synthetic metrics are suitable for this check only.

Both headless modes exercise the same Runtime's layout, resizing, and exit keys. The Sixel mode also checks DCS output, tile counts, unchanged-frame reuse, and absence of alternate-screen, mouse, or full-screen-clear sequences. **Neither mode verifies visual compatibility with a real terminal/multiplexer.** The repository's `python3 scripts/check-sixel-query.py --formula` checks PTY startup probing, input, and terminal-state restoration, not visual output either.

## Fonts and licensing

The public library bundles **no implicit fonts**, searches no system directories, and downloads nothing at runtime. `MathRenderer::new` accepts an iterator of font bytes (`AsRef<[u8]>`), copies and parses them once, and registers all readable faces in TTF/OTF/TTC collections. The selected family must contain a face with an OpenType **MATH** table.

Empty font sets, unreadable inputs, and missing or non-math families return errors. Additional text fonts can provide fallback glyphs; missing glyphs may produce warnings, so prefer `render_with_diagnostics`.

The quickstart and demo explicitly use `typst-assets` with its `fonts` feature, including New Computer Modern Math. Without that feature, its font iterator is empty. Alternatively, read your deployed fonts in application code or embed them with `include_bytes!`; this does not grant Typst filesystem access.

**rsille-typst is [MIT-licensed](LICENCE). Fonts have their own licenses.** Preserve the selected fonts' required license notices and attribution and follow their redistribution terms; this crate's MIT license does not replace them. See the [typst-assets 0.13.1 sources](https://github.com/typst/typst-assets/tree/v0.13.1).

## API and rendering behavior

| API | Purpose |
| --- | --- |
| `MathRenderer::new(font_bytes, math_family)` | Load reusable fonts, font book, and template source. |
| `render(formula, &options)` | Return `RasterCanvas`, discarding nonfatal warnings. |
| `render_with_diagnostics(formula, &options)` | Return `RenderedMath { canvas, warnings }`. |
| `RenderOptions` | Font size, pixel scale, RGB colors, and pixel budget; implements `Default`. |
| `Error` | Font/options errors, `Compile(Vec<SourceDiagnostic>)`, page count, raster limits, or `Raster(RasterError)`; implements `Display` and `std::error::Error`. |

`RasterCanvas`, `RasterError`, `Rgb`, and `SourceDiagnostic` are re-exported. This is a synchronous single-formula renderer, not a full-document API, dedicated widget, async task system, or formula-result cache.

The template uses automatic page dimensions, zero margins, and a block equation. Typst's math layout/line metrics remain; there is no pixel-level ink cropping. Empty formulas follow Typst layout, with each output axis at least one pixel.

Defaults are 18 pt, 2 pixels/pt (144 dpi), black foreground, white background, and a `16 × 1024 × 1024` pixel budget. One point is 1/72 inch. Colors are opaque 8-bit sRGB; formula styles may override the foreground. The transparent page's premultiplied RGBA is composited as `premultiplied_rgb + background * (1 - alpha)`, rounded without multiplying alpha twice or unpremultiplying. The output has no alpha; the background is also canvas metadata.

### Diagnostics

Compilation errors preserve Typst's messages, hints, spans, and traces. `Display` includes messages and hints; inspect `Error::Compile` for full diagnostics. Because input is passed through `eval(..., mode: "math")`, spans point to the template call: **there is no exact original-formula line/column mapping**.

`warnings` preserves what Typst returns, but Typst 0.13.1's immediate `eval` uses a separate sink and discards its warnings (for example, an unknown font in a direct `text` call). Layout/deferred-context warnings can be retained. This API cannot recover warnings discarded upstream.

## Resource and security limits

- Font size must be finite and in `0.1..=4096`; pixels/pt must be finite and in `0.01..=16`. `max_pixels` must be in `1..=16 * 1024 * 1024`. Invalid options fail before compilation.
- Compilation must produce exactly one page. Before rasterization, both axes are checked using the pinned renderer's f32 multiplication and `round().max(1)` rules: each at most 16384 pixels, with total pixels within both `max_pixels` and the core limit. You may lower the budget, not raise the core limit.
- RGB allocation uses `try_reserve_exact`, followed by `RasterCanvas::from_pixels` validation. The pixel budget is **not a process memory limit**: RGBA and RGB buffers coexist, alongside fonts, compilation, and layout data. Upstream allocation failures or internal panics are not guaranteed to become `Error` values.

The Typst World exposes only its in-memory main source and supplied fonts. All other source requests (including packages) and all file requests return `AccessDenied`. There is no path resolution, file reading, package download, system-font search, or current date. Input is passed as a `sys.inputs` string, not concatenated into the template, so closing `$` delimiters cannot escape it. Valid Typst `#` expressions can still execute.

**File-access restrictions are not a strong sandbox.** Loops, recursion, large expressions, embedded data/images, and complex layouts can exhaust CPU or memory before size checks or during rendering. There is no timeout, cancellation, execution-step budget, or process isolation. For untrusted formulas, use a separate worker process with host-enforced time/memory limits and termination. Do not repeatedly compile in a UI frame loop.

This crate does not cache formula results, but Typst/comemo and interners use process-global caches/storage. Dropping a renderer does not clear all upstream caches, and processing indefinitely many distinct formulas has no promised fixed memory bound. Rotate worker processes when strict bounds matter.

## Development checks

```sh
cargo test -p rsille-typst
cargo test -p rsille-typst --all-targets
cargo build -p rsille-typst --examples
cargo fmt -p rsille-typst -- --check
```

Tests cover real rendering, colors/alpha, fonts, diagnostics, denied resources, size limits, and rsille widget integration. Typst is a substantial build dependency; on memory-constrained machines, try a single build job and disable debug information (POSIX shell):

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p rsille-typst -j 1
```
