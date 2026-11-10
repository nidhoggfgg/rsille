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
requirements are checked **before entering the managed terminal session**: each dimension must
be 1..=1024 and the product at most 65,536. Cell height need not be divisible by six.

Without an override, startup explicitly probes cell pixels before any input reader
starts and stores the result as `Some(pixels)` in the raster configuration. The
probe tries the OS window-size API first, then a bounded `CSI 16 t` query on
Unix/WSL if OS measurements are unusable. The query requires stdin/stdout TTYs
attached to the controlling terminal and returns on reply. The API accepts a
configurable timeout (`resolve(None)` uses 500 ms for compatibility); this is not
a guaranteed elapsed time. Native Windows has no query fallback. If probing
fails, supply a measured override:

```sh
cargo run --release --features sixel --example sixel_windows -- --sixel --cell-pixels 8x18
```

**Use `8x18` only if it matches your actual terminal font/cell metrics.** It is not
an arbitrary rendering resolution or a way to bypass resource limits.
Measure the real cell size and keep overrides current after font/terminal changes.
Neither measured pixels nor the feature flag proves graphics support.

`--cell-pixels` is **not terminal columns × rows**. For example, a 960×648-pixel
text area containing 120 columns and 36 rows has 8×18-pixel cells, not 120×36.
Using 120×36 for that terminal makes each tile too wide and tall; adjacent tiles
then overlap, compressing the visible image and spilling beyond window borders.
Prefer omitting the override when the startup probe supplies correct metrics.
Stored pixels are a startup snapshot: font, DPI, and resize changes do not refresh
them automatically. Update the configured dimensions explicitly or restart; the
renderer never discovers pixels or reads terminal input. The example rejects an
override equal to the terminal grid unless
`--force-cell-pixels` explicitly confirms an unusual, measured cell size. Explicit
overrides skip both OS metric lookup and terminal queries, so they are not compared
against automatic measurements.
Actual **10×20-pixel cells are supported**: use the measured metrics or
`--cell-pixels 10x20`, without rounding the height to 18 or 24. Rounding would
misalign tiles with the real cell grid.

The encoder handles partial six-row bands: Sixel `P2=1` leaves unset pixels
transparent, raster attributes declare the exact height, and there is no extra
final band advance. For a 20-pixel tile, only two rows of the final band contain
pixels; its four padding rows remain unset. The real terminal/multiplexer path
must honor this transparent padding without painting outside the tile, including
at the bottom edge, and still satisfy the `no_scroll` and `reliable_erase`
assertions. Accepting these metrics is **not real-terminal certification**;
use `--text-only` if the terminal cannot satisfy that contract.

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

On Unix, `python3 scripts/check-sixel-query.py` is the PTY transport check entry
point. The startup-probe validation targets are explicit/OS/query selection,
fragmented replies, return-on-reply, configurable timeout, invalid replies,
terminal identity, raw-mode restoration, and rejection after managed input starts.
Do not expect preservation of unrelated input or automatic refresh after resize;
see the [manual checklist](raster-design.md#manual-checklist-for-a-prospective-native-configuration).

Headless checks, unit tests and PTY transport checks do **not** validate real-terminal
graphics, erasure, palette behavior, scrolling safety, or negotiation. The showcase replaces
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
| `RasterPolicy::Auto` (default) | Native only when all safety conditions, including stored valid pixels, are known; otherwise character fallback. |
| `RasterPolicy::TextOnly` | No cell-pixel queries or Sixel output. Use `RasterOutput::text_only()`. |
| `RasterPolicy::RequireSixel` | Reject unsupported raster frames before frame output, rather than silently falling back. Text-only frames need not satisfy native conditions. |

Build with `--features sixel` to include the native encoder. The encoder uses no
external encoding library; the Unix startup pixel query uses `libc`.
It uses a fixed 256-color **RGB332** palette (3 red, 3 green, 2 blue bits), not
lossless 24-bit output or adaptive image quantization.

Native output additionally requires all of the following:

1. An active alternate screen; inline native output and appended-newline output
   are unsupported (an ordinary linefeed can scroll even when Sixel does not).
2. Explicit `RasterCapabilities.sixel`, `.reliable_erase`, and `.no_scroll`
   assertions for the **whole terminal/multiplexer path**.
3. Stored `cell_pixels: Some(pixels)` with actual dimensions in 1..=1024 and
   product at most 65,536.
   Heights not divisible by six, including 10×20 cells, are supported without
   rounding; the terminal must honor transparent padding in partial bands.
4. Supported composition and the resource budgets below.

There is **no certified preset** to copy. For a host that has independently
completed validation, the shape of explicit configuration is:

```rust
use rsille::tui::prelude::*;

// Call only AFTER validating the active terminal, font, modes, and multiplexer.
// These true values are host assertions, NOT detection results or mode setters.
fn output_after_host_validation(pixels: CellPixels) -> RasterOutput {
    RasterOutput {
        policy: RasterPolicy::RequireSixel,
        capabilities: RasterCapabilities {
            sixel: true,
            reliable_erase: true,
            no_scroll: true,
            // Store validated explicit dimensions or the startup probe result.
            cell_pixels: Some(pixels),
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

**No DA or Sixel capability negotiation is implemented.** The cell-pixel query
below discovers dimensions only; the three capability flags remain host assertions.
Terminal names and environment variables are not proof of support.

**The renderer never discovers pixels or reads terminal input.** Eligible native
raster rendering requires `RasterCapabilities.cell_pixels: Some(valid_pixels)`.
Missing or invalid metrics mean text fallback for `Auto` or an error for
`RequireSixel`, not a discovery request. Text-only and unused raster paths are
unaffected by missing pixels.

`CellPixels::probe(timeout: std::time::Duration) -> std::io::Result<CellPixels>`
is an explicit startup operation for both standalone hosts and TUI apps:

- Read OS `window_size` first, dividing text-area pixels by columns/rows. Missing,
  zero, non-integral, or backend-unsupported dimensions trigger query fallback
  rather than guessing or rounding.
- On Unix/WSL, fallback sends `CSI 16 t` only when stdin/stdout are TTYs attached
  to the controlling terminal. `CSI 6 ; height ; width t` reports **pixels per
  cell directly**, not window pixels. Results must satisfy the backend limits.
- The query returns immediately on a valid reply; the caller's timeout bounds
  waiting and is not extended by unrelated input. It is not a guarantee of exact
  wall-clock completion time. Native Windows has no query fallback. If no source
  supplies usable dimensions, the probe returns an error.
- `CellPixels::resolve(Some(pixels))` validates only, without OS reads or queries.
  `resolve(None)` calls `probe(Duration::from_millis(500))` for compatibility.

Probe **before starting any input reader**, and **store the result as `Some`**:

```rust
use std::time::Duration;
use rsille::render::raster_output::{CellPixels, RasterCapabilities, RasterOutput};

// Before managed input initialization or starting an external reader.
let pixels = CellPixels::probe(Duration::from_millis(500))?;
// Alternatively, validate measured dimensions without probing:
// let pixels = CellPixels::resolve(Some(CellPixels { width: 10, height: 20 }))?;
let output = RasterOutput {
    capabilities: RasterCapabilities {
        cell_pixels: Some(pixels),
        ..Default::default() // No Sixel support asserted by this measurement.
    },
    ..Default::default()
};
// Pass output to the builder/app; assert native capabilities only after validation.
# Ok::<(), std::io::Error>(())
```

Stored pixels are a **startup snapshot**, not a live cache. Font, DPI, and resize
changes do not update the configuration automatically. Supply updated dimensions
explicitly or restart. `CellPixels::from_terminal()` remains an optional OS-only
read callable at runtime, without backend-limit validation or query fallback;
validate any result before storing it. It does not read terminal input.

**Input ownership:** probing is permanently rejected after managed input
initialization, even after that loop exits. External reader state cannot be
detected: standalone/embedded hosts may probe, but must do so before starting
their own readers. The startup query may consume unrelated input, does not flush
input, and can leave late replies arriving after timeout. Do not rely on event
preservation. Native crossterm `EventStream` handles all managed runtime input;
there is no runtime pixel-query reader. Calling the probe is independent of
renderer policy and never establishes Sixel support.

Graphics cleanup uses **ED 2 plus full redraw**, not assumptions that spaces or
line erasure delete images. This path is permitted only after the explicit native
safety assertions above. Changed/removed coverage, movement, metric changes,
resize, or switching to text may require a full screen clear and repaint. The
normal terminal guard restores modes and leaves the alternate screen on exit;
actual graphics persistence on exit/errors still requires real-terminal testing.
There is no portable Sixel resource-delete command or retained image layer here.

Native frames and graphics-cleanup frames wrap the entire clear/text/tile repaint
in synchronized output (DEC mode 2026), ending before the final flush. Supporting
terminals can present a completed frame instead of exposing blank screens or
partially redrawn rows during dragging. Writer errors trigger best-effort protocol
cancellation and synchronization release; retries repeat recovery and fully repaint.
This is not capability negotiation: terminals or multiplexers without synchronized
output support may still show progressive repainting. A final `flush()` alone does
not make a terminal update atomic, and synchronization does not reduce tile count
or guarantee a particular frame rate.

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
