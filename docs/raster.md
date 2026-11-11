# Opaque raster canvases

`RasterCanvas` is a fixed-resolution RGB drawing resource. It is separate from
`Canvas` and its character encodings: there is no `Encoding::Sixel`. Raster terminal output uses Sixel only, under explicit safety conditions.
Widgets submit content to the renderer; they do not write terminal escapes.

**Current status:** Sixel is an opt-in, capability-gated implementation, not a certified terminal support
claim. Automatic capability negotiation, real-terminal validation, and controlled
performance acceptance are **pending**. Historical memory-writer measurements are
retained in [the performance notes](raster-performance.md); their harness has been
removed. See [the design and validation plan](raster-design.md).

## Migrating from character fallback

Raster terminal output is now Sixel-only. Remove `RasterPolicy`,
`RasterOutput.policy`, and `RasterOutput::text_only()`; configure
`RasterOutput { capabilities: ... }` instead. The optional `sixel` feature remains
required for encoding, but does not assert terminal support.

Remove `RasterFallback`, `RasterOptions.fallback`, and widget `.fallback(...)`
calls. There is no replacement character-image backend. `frame_text_only()` was
removed: use `frame()` and inspect its raster metadata in headless checks. Do not
export its placeholder character cells as an image. To render on a terminal,
supply valid capabilities and cell pixels or handle the resulting error.

The demos no longer accept `--text-only` or the AI Babies `--raster` preview.
AI Babies still defaults to Canvas and supports Canvas image exports; its Sixel
mode rejects image export commands. `Canvas` and its Braille/HalfBlock/Quadrant
encodings are unchanged.

## External pixel producers

No image or document-rendering dependency is required. Import opaque row-major
pixels with `RasterCanvas::from_pixels(width, height, background, Vec<Rgb>)`
(ownership transfer, no pixel copy), or
`RasterCanvas::from_rgb8(width, height, background, &[u8])` (one RGB conversion
allocation). Both validate nonzero dimensions, the existing 16,777,216-pixel
limit, and exact input length. RGB8 is tightly packed R/G/B without stride or
alpha; callers must remove padding and composite alpha themselves. Invalid
lengths return `RasterError::InvalidPixelLength`. The background argument is
canvas metadata, not a transformation of imported pixels. Fresh resources start
with a new identity and version zero, and retain normal COW snapshot semantics.

```rust
use rsille::render::raster_output::CellPixels;
use rsille::tui::prelude::*;

// A simulated external renderer's tightly packed RGB8 result.
let bytes = vec![128; 17 * 33 * 3];
let image = RasterCanvas::from_rgb8(17, 33, Rgb(0, 0, 0), &bytes).unwrap();
// Explicit host-provided physical metrics, not terminal discovery.
let size = CellPixels { width: 8, height: 16 }
    .size_for_pixels(image.width(), image.height()).unwrap();
let view = raster_canvas::<()>(image)
    .scale(RasterScale::Original)
    .align_x(RasterAlign::Start)
    .align_y(RasterAlign::Start)
    .fixed(size.width, size.height);
```

`CellPixels::size_for_pixels` rounds up independently on each axis and returns
`None` for zero cell metrics or a cell count exceeding `u16::MAX`. Zero image
axes yield zero cell axes. It is a pure checked conversion, not a Sixel capability
check or a layout policy; the application chooses constraints and when to update
explicit metrics.

`RasterScale::Original` does not rescale: one source pixel maps to one destination
sample. Smaller images are padded with `RasterOptions::background`; larger ones
are cropped. Start/Center/End alignment applies on each axis to both padding and
cropping. With Center, an odd extra padded or cropped pixel is at bottom/right.
Clipping, tiling and occlusion preserve the original full target and offsets;
visible fragments are never refitted. Native Sixel uses physical cell pixels
(and still quantizes colors to RGB332). There is no Raster character fallback.
Use Fit for an overview of the source.
The default remains centered Fit; Stretch is unchanged.

The public `Chunk::raster` API accepts the same resource and options for custom
drawing. `tests/raster_external.rs` exercises both public widget and chunk paths
without a terminal or an external rendering library. Adding enum variants means
external exhaustive matches on `RasterScale` / `RasterError` need updating.

## Run the showcase

[`examples/sixel_windows.rs`](../examples/sixel_windows.rs) is a single-file
root-package example, not a standalone Cargo project. Two `floating_window`
widgets in a `window_host` show the **same static procedural ringed planet**:
one uses opaque RGB `RasterCanvas` with native Sixel output, the other the existing
`Canvas` character-sampling API. Native output is quantized to RGB332; it is not
lossless 24-bit display. This floating-window example is fullscreen-only, using
the alternate screen; it has no live animation or `--inline` option. For inline
animation, use [AI Babies](#inline-sixel-animation) below.

From the repository root, after verifying the terminal safety contract below:

```sh
cargo run --release --features sixel --example sixel_windows -- --sixel
```

In `sixel_windows`, `--sixel` is an **explicit user assertion**, not capability
detection: the entire terminal/multiplexer path must support Sixel,
non-scrolling positioned output (including the last row), stable colors when
later tile palettes are sent, and alternate-screen ED 2 graphics erasure.
These capabilities must be verified on the actual terminal/multiplexer path;
no terminal is certified here.
Raster output is strict, so failure is an error,
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
pixels; its four padding rows remain unset. Transparent padding prevents painting
adjacent cells on conforming terminals, but does not by itself prevent bottom-edge
scrolling: Windows Terminal can check the full band height. Inline reserves a
bottom margin for this padding. Fullscreen instead anchors bottom-edge tiles
higher with transparent prefix pixels, without moving or cropping their visible
pixels. For 20-pixel cells, the last-row DCS starts two rows earlier with 40
transparent pixels followed by the original 20 pixels: all ten bands fit exactly.
The encoder starts at 1:1 (`P1=7`), avoiding an initial 12-pixel segment on Windows
Terminal. A viewport too short for any safe anchor fails before output. Transparent prefixes,
positioned output and `reliable_erase` still require host support. Accepting these metrics is
**not real-terminal certification**;
do not run native output if the terminal cannot satisfy that contract.

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

No flags, or `--help`, shows help without entering the interactive demo.
`--headless` runs finite, terminal-independent mouse-drag and occlusion
checks and exits:

```sh
cargo run --example sixel_windows -- --help

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

### Inline Sixel animation

[`examples/ai-babies`](../examples/ai-babies/README.md) demonstrates animated
inline Sixel on the normal screen:

```sh
cargo run --release --features sixel --manifest-path examples/ai-babies/Cargo.toml -- --sixel
```

All AI Babies animation modes use inline output, without an alternate screen or
mouse capture. `--sixel` needs no extra `--inline` argument and rejects unsupported output. It asserts Sixel support, stable palettes,
non-scrolling positioned Sixel output, reliable row-local EL 2 graphics erasure,
and graphics following text scrolling during row reservation. Its configuration
uses `reliable_inline_erase: true` and `reliable_erase: false`. The `no_scroll`
assertion forbids scrolling caused by Sixel emission itself, not controlled text
scrolling to reserve inline rows. None of these assertions is capability detection;
fullscreen ED 2 validation does not establish inline safety. Inline owns the full
width of its occupied rows, not just the image columns.

`--height` still accepts only 24 or 32 (default 24). Native inline content height is
also capped by the cell rows needed for the 320-pixel image and the terminal height
remaining after guard rows are reserved. With enough space, actual 10×20-pixel
cells use **16 content rows + 1 blank guard row**, not 16 rows total; 8×18 cells
need no extra guard row. The image is left-aligned, with a native area up to
approximately 320×320 physical pixels; smaller terminals scale it to fit. Cell-pixel discovery
happens before input starts and does not detect Sixel support. See the
[AI Babies guide](../examples/ai-babies/README.md#raster-and-sixel) for measured
cell overrides, character selection, controls and headless checks.

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

Resize, scale, alignment, and unrelated UI updates do not themselves
rerun the callback. A retained widget's `invalidate()` changes its revision but
does not schedule a frame; rebuilt views should use application-owned revisions.
`from_draw` validates dimensions immediately but defers pixel allocation/drawing
until visible. Deferred allocation failures display an opaque diagnostic and are
cached until dimensions, background, or revision change. Use `RasterCanvas::new`
first if allocation errors must be handled before rendering.

## Presentation and composition

`RasterOptions` defaults to a black presentation background, `Fit`, centered
alignment on both axes. Raster has no character fallback option.

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

Clipping and scrolling crop the original scaled target rather than rescaling the
visible fragment. Each `RasterCell` retains its full `target` size and original
`x`, `y` cell offsets. Its `sample(pixel_x, pixel_y, cell_width, cell_height)` takes
**cell-local** pixel coordinates and full physical cell metrics. Blitting moves
where a cell is stored without changing that metadata.

Drawing order resolves ownership per cell. A later opaque text panel covers the
raster; a later raster submission replaces text. Do not rely on transparent text
or alpha blending over native raster content. Character-based visual effects cannot represent raster content;
real output rejects frames marked with unsupported raster composition.

## Headless and embedded frames

Headless rendering does not require a terminal or emit Sixel:

```rust
use rsille::tui::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let image = RasterCanvas::new(16, 12, Rgb(40, 100, 160))?;
    let mut runtime = App::new(())
        .build(|_, ()| {}, move |_| raster_canvas::<()>(image.clone()))
        .size(20, 8);
    runtime.step(&[])?;
    assert!(runtime.frame().has_raster());
    for (x, y, cell) in runtime.frame().raster_cells() {
        // x/y are buffer coordinates; cell.x/cell.y are original target offsets.
        let _ = (x, y, cell.canvas.id(), cell.canvas.version());
    }
    Ok(())
}
```

`Runtime::frame()` borrows the retained `Buffer`, including raster ownership and
source metadata. It is blank before the first step and immediately after resize,
until another step draws. `Buffer::raster_at(position)` inspects a particular owner.
Headless stepping does not invoke the terminal renderer: it needs neither Sixel
capabilities nor terminal pixel discovery. It is not a text-only output backend.
There is no character snapshot conversion; hosts must consume raster metadata
rather than silently dropping it. To test Sixel encoding, explicitly supply
synthetic capabilities and cell pixels to a renderer writing to memory. This does
not certify a real terminal.

## Terminal output policy

Configure `App::raster_output(config)`, `render::Builder::raster_output(config)`,
or change a renderer with `Render::set_raster_output(config)`:

`RasterOutput` contains only host-supplied `capabilities`; its default does not
assert Sixel support. Any real frame containing raster content must satisfy all
native safety conditions, or rendering returns an error before frame output.
There is no `RasterPolicy`, text-only backend, or automatic character fallback.
Frames containing only text or existing `Canvas` graphics need no Sixel support.

Build with `--features sixel` to include the native encoder. The encoder uses no
external encoding library; the Unix startup pixel query uses `libc`.
It uses a fixed 256-color **RGB332** palette (3 red, 3 green, 2 blue bits), not
lossless 24-bit output or adaptive image quantization.

Native output additionally requires all of the following:

1. Either an active alternate screen with `RasterCapabilities.reliable_erase`,
   or inline mode with `RasterCapabilities.reliable_inline_erase`. Inline does
   **not** require the alternate screen or `reliable_erase`. Native appended-
   newline output remains unsupported in either mode; inline's controlled row
   reservation is distinct from appending a newline after a native frame.
2. Explicit `RasterCapabilities.sixel` and `.no_scroll` assertions for the
   **whole terminal/multiplexer path**, plus the mode-specific erase assertion.
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
        capabilities: RasterCapabilities {
            sixel: true,
            reliable_erase: true,
            reliable_inline_erase: false, // Fullscreen contract only.
            no_scroll: true,
            // Store validated explicit dimensions or the startup probe result.
            cell_pixels: Some(pixels),
        },
    }
}
```

`reliable_erase` asserts that ED 2 clears graphics as well as characters in the
alternate screen. `reliable_inline_erase` (default **false**) is a separate contract:
normal-screen EL 2 clears graphics on each owned inline row without damaging
other rows, and graphics move with text scrolling during row reservation. For
validated inline output, set `reliable_inline_erase: true`, select
`App::mode(Mode::Inline)` (or `Builder::inline_mode(true)`), and leave
`reliable_erase: false` if the alternate-screen contract has not been validated.
Inline owns each occupied row across its full width; do not share those rows
with unrelated host content. Newly reserved rows are initialized even when their
buffer contents are unchanged spaces. Managed loops track the physical viewport
height so a terminal shrink cannot leave an off-screen bottom-row anchor.
Standalone hosts should call `Render::set_inline_viewport_height(height)` at
startup and on terminal resize, separately from buffer resizing.

`no_scroll` asserts safe, stable positioned output, transparent partial-band
padding that does not erase adjacent cells, and retained colors when subsequent
tile palettes are sent. Inline keeps the final band's padding within a reserved
bottom margin to avoid scrolling; fullscreen still requires safe Sixel output at
the screen's last row. It does not forbid text scrolling to reserve inline rows.
These flags do not enable terminal modes. A standalone `Render` does not establish
terminal modes itself; the caller must establish the configured screen lifecycle,
or use the managed event-loop/TUI lifecycle.

**No DA or Sixel capability negotiation is implemented.** The cell-pixel query
below discovers dimensions only; all capability flags remain host assertions.
Terminal names and environment variables are not proof of support.

**The renderer never discovers pixels or reads terminal input.** Eligible native
raster rendering requires `RasterCapabilities.cell_pixels: Some(valid_pixels)`.
Missing or invalid metrics cause an error for raster output, not a discovery
request. Frames without raster content are
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

Fullscreen graphics cleanup uses **ED 2 plus full redraw**, under the explicit
alternate-screen safety assertions above. Changed/removed coverage, movement,
metric changes, resize, or switching to text may require a full screen clear and
repaint.

Inline uses **relative cursor positioning** and reserves content and blank guard
rows **before** drawing images. For cell height `h`, final-band padding is
`padding = (6 - h % 6) % 6` pixels, requiring `ceil(padding / h)` guard rows:
10×20 needs one, 8×18 needs none, and even a tiny `h = 1` needs five. When viewport
height is known, guard rows are deducted first and content height is limited to
the remainder. `SavePosition` anchors the bottom of the reserved extent, including
guard rows; a successful frame returns the cursor to the last content row, so an
exit newline can land in a guard row.

When old graphics change, inline clears only the `max(old, new)` occupied extents
with **EL 2**, including each extent's guard rows, and redraws the current text and
raster frame, cleaning up rows released by shrinking. It never uses ED 2. This is
permitted only by `reliable_inline_erase`, not by assuming ordinary line erasure
is a portable image-delete operation. Content on other rows must survive.

The normal terminal guard restores modes and leaves the alternate screen when
applicable; actual graphics persistence on exit/errors still requires real-terminal
testing in each mode. There is no portable Sixel resource-delete command or
retained image layer here.

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
| Native frame | At most 32,768 visible raster cells and 16,777,216 sampled physical pixels |
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

With 10×20-pixel character cells, the cell-count budget allows up to 6,553,600
sampled physical pixels per frame. These are safety budgets, not throughput
or terminal frame-rate guarantees; the pixel and encoded-payload limits still
apply independently.

Native frame budget/coordinate-limit violations return an error. Encoder allocation/encoding and writer failures
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
