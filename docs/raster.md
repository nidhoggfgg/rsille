# Opaque raster canvases

`RasterCanvas` is a fixed-resolution RGB drawing resource. It is separate from
`Canvas` and its character encodings: there is no `Encoding::Sixel`. The same
source can be presented as text or, under explicit safety conditions, Sixel.
Widgets submit content to the renderer; they do not write terminal escapes.

**Current status:** character fallback is the default usable path. Native Sixel
is an opt-in, capability-gated implementation, not a certified terminal support
claim. Automatic capability negotiation, real-terminal validation, and controlled
performance acceptance are **pending**. Historical memory-writer measurements are
retained in [the performance notes](raster-performance.md); their harness has been
removed. See [the design and validation plan](raster-design.md).

## Run the showcase

[`examples/sixel_windows.rs`](../examples/sixel_windows.rs) is a single-file
root-package example, not a standalone Cargo project. Two `floating_window`
widgets in a `window_host` show the **same static procedural ringed planet**:
one uses opaque RGB `RasterCanvas` with native Sixel output, the other the existing
`Canvas` character-sampling API. Native output is quantized to RGB332; it is not
lossless 24-bit display. There is no live animation.

From the repository root, after verifying the terminal safety contract below:

```sh
cargo run --release --features sixel --example sixel_windows -- --sixel
```

`--sixel` is an **explicit user assertion**, not capability detection: the entire
terminal/multiplexer path must support Sixel, ED 2 erasure of graphics,
non-scrolling positioned output (including the last row), and stable colors when
later tile palettes are sent. The demo uses `RequireSixel`, so failure is an error,
not a silent character fallback. The feature and current backend cell-metric
requirements are checked **before entering terminal mode**: each dimension must
be 1..=1024, the product at most 65,536, and cell height a multiple of six.

By default, cell pixels come from the OS window-size API. If those measurements
are unavailable, an explicit override is possible:

```sh
cargo run --release --features sixel --example sixel_windows -- --sixel --cell-pixels 8x18
```

**Use `8x18` only if it matches your actual terminal font/cell metrics.** It is not
an arbitrary rendering resolution or a way to bypass the height restriction.
Measure the real cell size and keep overrides current after font/terminal changes.
Neither measured pixels nor the feature flag proves graphics support.

Prefer a 100×32-cell terminal with mouse reporting; smaller sizes and terminal
resizing are supported. Controls:

| Input | Action |
| --- | --- |
| Drag a window title | Move the window |
| Drag a bottom grip | Resize the window |
| Click a window | Bring it forward, changing occlusion |
| `e` | Cycle the Canvas window through Braille / HalfBlock / Quadrant |
| `r` | Restore the starting window rectangles |
| `q` / Esc | Quit |

No flags, or `--help`, shows help without entering the interactive demo. Choose
`--text-only` explicitly for a character preview; this does not demonstrate native
Sixel. `--headless` runs finite, terminal-independent mouse-drag and occlusion
checks and exits:

```sh
cargo run --example sixel_windows -- --help
cargo run --release --example sixel_windows -- --text-only
cargo run --example sixel_windows -- --headless
cargo run --features sixel --example sixel_windows -- --headless
cargo test --example sixel_windows --features sixel
cargo test --example sixel_windows --no-default-features
```

Headless checks and unit tests do **not** validate real-terminal graphics,
erasure, palette behavior, scrolling safety, or negotiation. The showcase replaces
the earlier separate raster demos and benchmark harness; it is not a benchmark.

## Source coordinates and opacity

Import the model from `rsille::render::raster`, or use the TUI prelude:

```rust
use rsille::tui::prelude::*;

fn make_image() -> Result<impl Widget<()>, RasterError> {
    let mut image = RasterCanvas::new(160, 96, Rgb(12, 18, 30))?;
    image.set_pixel(0, 0, Rgb(255, 255, 255));
    image.line(-20, 70, 180, 10, Rgb(220, 230, 255));
    image.rect(4, 4, 152, 88, Rgb(100, 150, 220));
    image.fill_rect(12, 14, 43, 25, Rgb(255, 172, 72));
    image.circle(111, 43, 26, Rgb(255, 172, 72));
    image.fill_circle(111, 43, 13, Rgb(80, 140, 240));
    Ok(raster_canvas::<()>(image).fixed(40, 12))
}
```

Coordinates are signed `i32` source pixels: top-left `(0, 0)`, x increases right,
y increases down. Lines include their endpoints. Rectangles take
`(x, y, width, height, color)` and cover the half-open rectangle
`[x, x + width) × [y, y + height)`. Non-positive rectangle extents and negative
circle radii do nothing; radius zero draws one pixel. Out-of-bounds pixels are
clipped. Work is bounded by the canvas dimensions even with extreme coordinates.
Circles and lines are integer rasterizations, without antialiasing.

`Rgb(pub u8, pub u8, pub u8)` has **no alpha**. Every source pixel is opaque,
including pixels never drawn over. `clear(color)` replaces all pixels and updates
`background()`. Later drawing replaces earlier colors, rather than blending them.
Source width and height are fixed and must both be nonzero.

`width()`, `height()`, `background()`, and row-major `pixels() -> &[Rgb]` expose the
source without a copy. There is no image-file decoding, text rasterization,
world-coordinate adapter, or arbitrary vector path API in this model.

## Snapshots and callback caching

`RasterCanvas::clone()` shares pixels in O(1). The first actual pixel mutation of
a shared buffer copies it; no-op writes do not detach it. `id()` identifies a
resource lineage and `version()` identifies its revision. Independently mutated
clones receive distinct revisions. Equality compares identity and revision, not
all pixels; equal-looking separately constructed resources need not be equal.
Keep a canvas or `Arc<RasterCanvas>` in application state instead of rebuilding
an identical source on every view call.

`RasterCanvasWidget::new(image)` and `raster_canvas(image)` accept an owned canvas
or `Arc<RasterCanvas>`. Procedural sources can instead use:

```rust
use rsille::tui::prelude::*;

// revision is owned by the application and tracks its drawing inputs.
fn diagram(revision: u64) -> Result<impl Widget<()>, RasterError> {
    Ok(RasterCanvasWidget::<(), _>::from_draw(160, 96, |canvas, ctx| {
        let x = (ctx.pixel_width() / 2) as i32;
        let y = (ctx.pixel_height() / 2) as i32;
        canvas.fill_circle(x, y, 20, Rgb(255, 172, 72));
    })?
    .background(Rgb(12, 18, 30))
    .revision(revision)
    .key("diagram"))
}
```

The callback receives fixed source-pixel dimensions, not terminal dimensions.
Its result is cached at the widget identity across view rebuilds. Change
`.revision(...)` when captured inputs change: **a new closure alone does not
invalidate the cached source**. Dimensions and background also invalidate it.
Each cache miss starts with a newly background-filled canvas. A stable `.key()`
helps preserve state when siblings are reordered.

Resize, scale, alignment, fallback, and unrelated UI updates do not themselves
rerun the callback. A retained widget's `invalidate()` changes its revision but
does not schedule a frame; rebuilt views should use application-owned revisions.
`from_draw` validates dimensions immediately but defers pixel allocation/drawing
until visible. Deferred allocation failures display an opaque diagnostic and are
cached until dimensions, background, or revision change. Use `RasterCanvas::new`
first if allocation errors must be handled before rendering.

## Presentation and composition

`RasterOptions` defaults to a black presentation background, `Fit`, centered
alignment on both axes, and `HalfBlock` fallback.

- `Fit` preserves aspect ratio using the **full target's physical pixel size**,
  rounds fitted dimensions down with a one-pixel minimum, and explicitly paints
  letterboxing with `options.background`.
- `Stretch` scales the axes independently to fill the target.
- `RasterAlign::{Start, Center, End}` position the fitted image in unused space.
  Center gives an odd extra pixel to the bottom/right.
- Both use nearest-neighbor sampling at destination pixel centers.

The source background and presentation background are independent. The widget's
`.background(...)` also initializes procedural sources, but does not recolor an
existing shared source. Layout methods such as `.fixed(40, 12)` use terminal cells,
not source pixels. Surrounding containers own borders and padding.

Text fallback assumes a **1:2 cell aspect ratio**, not measured terminal pixels.
HalfBlock samples a 1×2 grid and writes an upper-half block with explicit RGB
foreground/background. Braille samples 2×4 and approximates the eight samples
with two colors. Both are opaque, including uniform cells and letterboxing.
If the configured text-width policy makes a fallback glyph wider than one cell,
Buffer substitutes a styled space to preserve cell geometry.

Clipping and scrolling crop the original scaled target rather than rescaling the
visible fragment. Each `RasterCell` retains its full `target` size and original
`x`, `y` cell offsets. Its `sample(pixel_x, pixel_y, cell_width, cell_height)` takes
**cell-local** pixel coordinates and full physical cell metrics. Blitting moves
where a cell is stored without changing that metadata.

Drawing order resolves ownership per cell. A later opaque text panel covers the
raster; a later raster submission replaces text. Do not rely on transparent text
or alpha blending over native raster content. Character-based visual effects
flatten their affected raster content before applying the effect; strict native
policy rejects frames marked with unsupported raster composition.

## Headless and embedded frames

Headless rendering does not require a terminal or emit Sixel:

```rust
use rsille::tui::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let image = RasterCanvas::new(16, 12, Rgb(40, 100, 160))?;
    let mut runtime = App::new(())
        .raster_output(RasterOutput::text_only())
        .build(|_, ()| {}, move |_| raster_canvas::<()>(image.clone()))
        .size(20, 8);
    runtime.step(&[])?;
    assert!(runtime.frame().has_raster());
    for (x, y, cell) in runtime.frame().raster_cells() {
        // x/y are buffer coordinates; cell.x/cell.y are original target offsets.
        let _ = (x, y, cell.canvas.id(), cell.canvas.version());
    }
    let text = runtime.frame_text_only();
    assert!(!text.has_raster());
    Ok(())
}
```

`Runtime::frame()` borrows the retained `Buffer`, including raster ownership and
stored character fallbacks, **regardless of terminal output policy**. It is blank
before the first step and immediately after resize, until another step draws.
`Buffer::raster_at(position)` inspects a particular owner.
`frame_text_only()` allocates a character-only copy using already-rendered
fallbacks, preserving colors and text metrics. It neither redraws the source nor
changes the retained frame. An embedding host must explicitly choose which
representation it consumes rather than silently dropping graphics metadata.

## Terminal output policy

Configure `App::raster_output(config)`, `render::Builder::raster_output(config)`,
or change a renderer with `Render::set_raster_output(config)`:

| Policy | Behavior |
| --- | --- |
| `RasterPolicy::Auto` (default) | Native only when all safety conditions are known; otherwise character fallback. |
| `RasterPolicy::TextOnly` | No raster capability queries or Sixel output. Use `RasterOutput::text_only()`. |
| `RasterPolicy::RequireSixel` | Reject unsupported raster frames before frame output, rather than silently falling back. Text-only frames need not satisfy native conditions. |

Build with `--features sixel` to include the native encoder. The feature adds no
new dependencies; it uses a fixed 256-color **RGB332** palette (3 red, 3 green,
2 blue bits), not lossless 24-bit output or adaptive image quantization.

Native output additionally requires all of the following:

1. An active alternate screen; inline native output and appended-newline output
   are unsupported (an ordinary linefeed can scroll even when Sixel does not).
2. Explicit `RasterCapabilities.sixel`, `.reliable_erase`, and `.no_scroll`
   assertions for the **whole terminal/multiplexer path**.
3. Valid `cell_pixels`: dimensions 1..=1024, product at most 65,536, and
   **`height % 6 == 0`**. The height restriction prevents unverified six-row band
   allocation from leaking beyond cell clipping or the bottom edge.
4. Supported composition and the resource budgets below.

There is **no certified preset** to copy. For a host that has independently
completed validation, the shape of explicit configuration is:

```rust
use rsille::tui::prelude::*;

// Call only AFTER validating the active terminal, font, modes, and multiplexer.
// These true values are host assertions, NOT detection results or mode setters.
fn output_after_host_validation(measured: CellPixels) -> RasterOutput {
    RasterOutput {
        policy: RasterPolicy::RequireSixel,
        capabilities: RasterCapabilities {
            sixel: true,
            reliable_erase: true,
            no_scroll: true,
            cell_pixels: Some(measured),
        },
    }
}
```

`reliable_erase` asserts that ED 2 clears graphics as well as characters in the
alternate screen. `no_scroll` also asserts stable positioned origins, no scrolling
at the last row, and retained colors when subsequent tile palettes are sent.
These flags do not enable terminal modes. A standalone `Render` does not establish
terminal modes itself; the caller must establish the configured alternate screen,
or use the managed event-loop/TUI lifecycle.

**No active DA/CSI graphics capability probe is implemented.** Crossterm's input
stream does not route those replies to this layer. Starting a competing stdin
reader would steal application input, so the implementation deliberately does
not do that. Terminal names and environment variables are not proof of support.

`CellPixels::from_terminal()` uses the OS window-size API/syscall, not terminal
query escapes or an input reader. Missing, zero, or non-integral metrics yield
`None`. With explicit Sixel support and no metric override, the renderer looks up
OS metrics lazily on raster frames, allowing resize updates. TextOnly, unused
raster paths, and default unknown-capability Auto do not trigger this lookup.
Metrics alone never establish graphics support; explicit overrides must be kept
current after font/terminal changes.

Graphics cleanup uses **ED 2 plus full redraw**, not assumptions that spaces or
line erasure delete images. This path is permitted only after the explicit native
safety assertions above. Changed/removed coverage, movement, metric changes,
resize, or switching to text may require a full screen clear and repaint. The
normal terminal guard restores modes and leaves the alternate screen on exit;
actual graphics persistence on exit/errors still requires real-terminal testing.
There is no portable Sixel resource-delete command or retained image layer here.

## Budgets and performance limits

| Resource | Current bound |
| --- | --- |
| Source canvas | 16,777,216 pixels (`MAX_RASTER_PIXELS`), about 48 MiB RGB per distinct buffer |
| Native cell tile | 1..=1024 pixels per dimension; at most 65,536 pixels and 1 MiB encoded payload |
| Encoded tile LRU | At most 1,024 entries; approximately 4 MiB including reserved entry overhead |
| Native frame | At most 4,096 visible raster cells and 16,777,216 sampled physical pixels |
| Staged native frame payload | At most 32 MiB of encoded Sixel payload |
| Reusable offscreen pool | At most 8 buffers and 8 MiB of cell-storage capacity per thread; source snapshots released after reading |

Canvas construction reports `EmptyDimensions`, `TooManyPixels`, or
`AllocationFailed`. COW mutation allocation follows ordinary allocator failure
behavior. Aggregate application-owned snapshots are not globally budgeted.
Cache-returned byte Arcs can outlive eviction; frame staging and external owners
are not counted as cache occupancy. Offscreen buffers also allocate character
cells and, when used, a dense raster-ownership table. The raster widget itself
submits only visible cells, even for a huge clipped target; other containers may
still allocate full-content offscreens, so large scroll content is not free.

Auto falls back to text for native frame budget/coordinate-limit violations;
RequireSixel returns an error. Encoder allocation/encoding and writer failures
can still propagate as errors, rather than promising recovery from every failure.

Native output uses **one positioned Sixel tile per visible terminal cell**.
Clipping and overlap are simple, but many small DCS payloads can be expensive.
Unchanged native tiles are not resent unless invalidated by cleanup; moving a
resource can reuse encoded bytes, but does not avoid erasing old screen coverage.
`Render::raster_stats()` exposes encode/cache-hit counts, retained payload bytes
and entry count, and sent tile/payload-byte counts. Payload bytes exclude cursor
positioning and text output. See [historical performance smoke measurements](raster-performance.md)
for results from the removed memory-writer harness and its text/Canvas comparison. These are not
real-terminal throughput measurements or a high-frame-rate guarantee; allocation
profiling and controlled regression thresholds remain pending.
