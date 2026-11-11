# Raster implementation and validation plan

This describes the implementation, not completion of every requirement in the
Sixel PRD. **Automatic graphics capability negotiation, real-terminal
certification, allocation profiling, and controlled performance acceptance remain pending.
Historical memory-writer [smoke measurements](raster-performance.md) are retained,
but their harness has been removed.** The library's safe default is character
fallback. The [`sixel_windows` showcase](../examples/sixel_windows.rs) instead
requires an explicit mode; no flags shows help. See [the user guide](raster.md)
for configuration and controls.

## Modules, features, and API boundaries

The published crate and internal workspace packages use shared source; there is
not a second raster implementation for the internal crates.

The raster subsystem lives in `src/render/raster/`, with `mod.rs` exposing the
canvas API and public `output` configuration while keeping presentation state and
Sixel encoding private. `rsille::raster` remains the public shortcut, and
`rsille::render::raster_output` re-exports `raster::output` for compatibility.
Renderer integration tests live in `raster/render_tests.rs` but are compiled as a
child of the renderer so they can exercise its private state. TUI widgets and
shared terminal probing remain in their respective layers.

| Source | Responsibility / principal API |
| --- | --- |
| `src/render/raster/canvas.rs` | `Rgb`, opaque `RasterCanvas`, identity/revision COW snapshots, clipped integer primitives, `RasterOptions`, full-target `RasterCell::sample`, explicit-color `fallback` |
| `src/tui/widgets/display/raster_canvas.rs` | `RasterCanvasWidget`, `raster_canvas`, fixed-resolution `from_draw`, persistent revision-keyed callback state |
| `src/render/buffer.rs`, `chunk.rs` | Cell ownership, stored fallback, current/previous raster metadata, `Chunk::raster`, `Buffer::raster_at` / `raster_cells` |
| `src/tui/offscreen.rs` | Copy original raster metadata with cell blits; flatten to stored text for character effects and propagate the unsupported-composition marker |
| `src/render/raster/output.rs` | `RasterOutput`, `RasterPolicy`, host-supplied `RasterCapabilities`, `CellPixels`, `RasterStats`; conservative policy resolution |
| `src/render/raster/presenter.rs` | Per-renderer terminal-visible state, resource preflight, invalidation/erase plan, commit after successful flush |
| `src/render/raster/sixel.rs` | Private, optional dependency-free RGB332 encoder and bounded encoded-tile LRU |
| `src/render/render.rs` | Character diff, fullscreen ED 2 or inline row-local EL 2 cleanup, cursor positioning and Sixel writes; `set_raster_output`, `raster_stats` |
| `src/tui/runtime/core.rs` | `frame()` retains graphics and text; `frame_text_only()` makes an explicit character-only copy |

The core model, widget, and text fallback do not require the `sixel` feature.
`--features sixel` enables the private native encoder without adding an external
encoding dependency; Unix terminal input/query handling uses `libc`. The feature
does not assert that the active terminal is supported. Existing `Canvas` and
`Encoding` semantics remain separate.

## Buffer ownership and order

A Buffer always has its character cells. When raster content is used, it lazily
allocates a dense `Option<RasterCell>` ownership table, one slot per buffer cell;
frame history can retain a second table. There is no full-screen RGB framebuffer.
Each owner shares a source snapshot and retains the full target size, original
cell offset, and presentation options. A character fallback is stored alongside.

Drawing/blitting order determines the surviving owner. Ordinary writes erase
intersecting raster ownership, including wide-grapheme footprints. Raster writes
replace text with an opaque fallback and then attach the owner. Overlay layout
orders its children; Buffer receives those actual writes rather than a separate
unordered graphics queue. The output phase can write surviving text followed by
surviving raster tiles because those ownership regions no longer overlap; this
is not a global rule that images are always above text.

Scroll/offscreen blits move destination cells but preserve each source cell's
original target offsets. Clipping therefore crops a presentation instead of
fitting the source into the remaining viewport. The raster widget submits only
visible cells instead of allocating its entire unclipped target. The reusable
offscreen pool releases frame contents/source references after reading and retains
at most eight buffers / 8 MiB of cell storage per thread. Character-only visual effects
consume the stored fallback and mark the frame as having unsupported raster
composition. Auto can retain native tiles elsewhere; RequireSixel rejects the
marked frame, even if later drawing covers the affected region. No alpha blending
or transparent text-over-raster contract is provided.

## Three distinct caches/state layers

1. **Source content:** COW pixel storage, stable lineage `id`, unique mutation
   `version`. Equality is O(1), including safely divergent clones. Procedural
   widget state caches by source dimensions, background, and application revision
   at the widget identity. Presentation-only changes do not rerun drawing.
2. **Encoded tiles:** bounded LRU keyed by source id/version, original cell offset,
   full target dimensions, presentation options, and physical cell dimensions.
   Destination screen coordinates are intentionally absent, permitting byte reuse
   after movement. Entries retain encoded `Arc<[u8]>`, not source snapshots.
3. **Terminal-visible state:** previous visible owners/positions, metrics, origin,
   size, and an uncertain-I/O flag. A cache hit is not evidence that the terminal
   still displays that tile.

Removing/changing old coverage or changing origin, size, metrics, or frame-history
validity can require cleanup and a full redraw: ED 2 in fullscreen, or EL 2 only
on the `max(old, new)` occupied extents (including guard rows) in inline mode.
Inline owns the full width of those rows and must not clear unrelated rows. Merely adding disjoint coverage
need not erase old tiles. The presenter preflights a plan before frame output and
commits visible state only after flush succeeds. It marks native I/O uncertain
before writing; recoverable tile-write failure attempts DCS termination and
cursor/style restoration, and a subsequent frame clears uncertain coverage.
This is best-effort error handling, not a guarantee that an unavailable terminal
can be repaired.

Native emission requires explicit `sixel` and `no_scroll` host assertions and
stored valid cell pixels. Fullscreen additionally requires the alternate screen
and `reliable_erase`; inline instead requires `reliable_inline_erase`, without
requiring either an alternate screen or `reliable_erase`. Native appended-newline
output remains unsupported in both modes.

`reliable_inline_erase` defaults to **false**. It asserts that normal-screen EL 2
locally erases graphics without damaging other rows, and graphics move with text
scrolling when reserving rows. Inline reserves content and blank guard rows before
drawing images and uses relative cursor positioning. `SavePosition` anchors the
bottom of the reserved extent, including guard rows; after a successful frame,
the cursor returns to the last content row, allowing an exit newline to land in
a guard row. Changed old graphics trigger EL 2 over only `max(old, new)` inline
extents, including their guard rows, and a text/raster redraw, never ED 2.
Row reservation may scroll text; `no_scroll` requires safe positioned output,
transparent padding that does not erase adjacent cells, and stable palettes.
Inline's bottom margin keeps the final band from extending past the scrolling
boundary; fullscreen still requires safe output at the screen's last row.

ED 2 and EL 2 are relied on only under their respective contracts; neither space
overwrite nor line erasure is a portable image deletion primitive. The terminal
guard's ordinary alternate-screen entry clear is separate from the presenter's
graphics invalidation logic. Exit uses the existing mode-restoration lifecycle,
not a universal Sixel image-delete protocol. Both screen modes still need
real-terminal validation.

## Input and metric discovery

There is **no DA or Sixel capability negotiation** in this release. The `sixel`,
`reliable_erase`, `reliable_inline_erase`, and `no_scroll` flags remain host
assertions, not detection results.
Terminal names/environment heuristics and measured cell dimensions are not
positive evidence of Sixel support.

`CellPixels::probe(timeout: std::time::Duration) -> std::io::Result<CellPixels>`
is an explicit startup operation, never a renderer operation:

1. Read OS `window_size` first and divide text-area pixels by columns/rows.
2. If dimensions are unavailable, zero, non-integral, or backend-unsupported,
   Unix/WSL falls back to `CSI 16 t`. Stdin/stdout must be TTYs attached to the
   controlling terminal. `CSI 6 ; height ; width t` reports pixels **per cell
   directly**; do not divide by the grid. Validate the same backend limits.
3. Return immediately on a valid reply. The caller-configured timeout bounds
   waiting, is not extended by unrelated input, and does not guarantee exact
   elapsed milliseconds. Native Windows has no query fallback. Return an error
   if neither source supplies usable dimensions.
4. The caller must probe **before any input reader starts** and store the result
   as `RasterCapabilities.cell_pixels: Some(pixels)`. This is a startup snapshot,
   not a live cache: font, DPI, and resize changes do not refresh it. Supply
   updated dimensions explicitly or restart.

`CellPixels::resolve(Some(pixels))` validates only, without OS reads or queries;
`resolve(None)` calls `probe(Duration::from_millis(500))` for compatibility.
`CellPixels::from_terminal()` remains a raw optional OS-only read callable at
runtime, without backend-limit validation, terminal input, or query fallback.

The renderer **never discovers pixels or reads terminal input**. Eligible native
raster frames require stored valid pixels. `None` means character fallback for
`Auto` or an error for `RequireSixel`; text-only and unused raster paths are
unaffected. Invalid dimensions never request discovery either.

Probing is permanently rejected after managed input initialization, even after
that loop exits. External reader state cannot be detected: standalone/embedded
hosts can probe before starting their readers, but must enforce that ordering.
The startup query may consume unrelated input; late replies can arrive after
timeout, and no input is flushed. Event preservation is not promised. Native
crossterm `EventStream` handles all managed runtime input, with no runtime pixel
queries. Pixel probing does not establish Sixel support.

## Encoder and budgets

The encoder creates one independently positioned DCS tile per surviving raster
cell. Nearest-neighbor sampling uses full-target physical dimensions. Quantization
uses RGB332 (256 fixed colors); used palette definitions are emitted per tile,
with six-row bands and run-length compression. Black and letterbox pixels are
explicitly painted. Sixel `P2=1` leaves unset pixels transparent, raster attributes
declare the exact tile height, and there is no extra final band advance past the
tile. Transparent *unset band bits* are an encoding detail, not alpha in the
content model.

The presenter and example accept cell heights not divisible by six, within the
unchanged limits of 1..=1024 per dimension and at most 65,536 pixels. Actual 10×20
cells are supported without rounding to 18 or 24: the final band contains two
pixel rows and four unset padding rows. Transparent padding must not erase
adjacent cells, but transparency and exact raster attributes alone do not prevent
bottom scrolling. In the independent [Windows Terminal source investigation](https://github.com/microsoft/terminal/blob/5f7dc283013152f89bfb76950515c33f45157ad7/src/terminal/adapter/SixelParser.cpp#L861-L906),
`_maybeFlushImageBuffer` checks the full segment/band height against available
height and can schedule text scrolling even when the final band's unused bits
are transparent. This source finding is not real-terminal certification.

Inline therefore reserves `ceil(padding / h)` blank guard rows for cell height
`h`, where `padding = (6 - h % 6) % 6` pixels. At 10×20 this is one row: the
320-pixel AI Babies image uses **16 content rows + 1 blank guard row** when space
permits. At 8×18 no guard row is needed; at tiny `h = 1`, five are needed.
A known viewport height is reduced by the guard rows before limiting content
height. This inline margin does not fix fullscreen bottom-edge behavior:
fullscreen still requires safe positioned output at the last row and
`reliable_erase`; inline requires `reliable_inline_erase`. Both retain the shared
`no_scroll` contract and need real-terminal validation. Setting host capabilities
does not bypass resource limits.

- Source: at most 16,777,216 pixels, about 48 MiB RGB per distinct source buffer.
- Tile: nonzero dimensions at most 1024, at most 65,536 pixels, at most 1 MiB
  encoded payload. Dimension products/allocation sizes are checked.
- LRU: at most 1,024 entries, approximately 4 MiB including an overhead reserve.
  Reported `cache_bytes` counts payload only; returned Arcs may outlive eviction.
- Native frame: at most 4,096 visible raster cells and 16,777,216 physical pixels;
  staged encoded payload at most 32 MiB. Coordinate addition is checked too.

Auto chooses text when policy/preflight frame limits fail; RequireSixel reports
unsupported output. Encoder allocation/encoding errors and I/O errors may still
propagate. These are individual budgets, not a global memory limit for all source
snapshots, offscreen buffers, caches, and staged frames combined.

Per-cell tiles favor precise clipping and simple ownership over transport
throughput. There is no adaptive quantization or merged-rectangle optimization.
`Render::raster_stats()` separates encodes, cache hits/occupancy, and successful
sent tile/payload-byte counts; bytes exclude positioning and character output.

## Validation matrix

Automated tests and headless checks establish model/composition behavior and
expected bytes. They cannot certify a real terminal's erase, palette, cursor, or
scroll behavior. No real-terminal configuration is currently certified.

| Layer | Automated checks / expected evidence | Remaining work |
| --- | --- | --- |
| Source model | Dimensions/overflow, COW branching, no-op revisions, clipped and extreme primitives | Broader performance measurements |
| Sampling / fallback | Physical aspect, Fit/Stretch/alignment, original crop offsets, explicit colors, glyph width handling | Visual quality assessment across fonts |
| TUI composition | Callback cache/invalidation, resize, borders/padding, scroll blits, opaque coverage, text snapshots, effect flattening | Interactive checks with representative layouts |
| Policy / renderer | Unknown Auto fallback, TextOnly, strict rejection, feature/mode/metric gates, injected-writer bytes and invalidation | Actual terminal behavior, interrupted output, recovery |
| Encoder / cache | RGB332 tile bytes, bounds, deterministic output, cache keys/eviction | Terminal decoding and palette interaction |
| Startup metrics / input | OS-first probe, bounded Unix query, return-on-reply, configurable timeout, permanent managed-input guard, renderer uses stored pixels only | Real-terminal late replies, unrelated-input consumption, and explicit snapshot updates; DA/Sixel capability negotiation remains unimplemented |

### Terminal candidates — all UNVERIFIED

These are investigation candidates, **not supported presets**, and not assertions
that any particular build implements Sixel. Record exact versions, build flags,
OS, font/cell size, screen modes, and every multiplexer hop for each test run.

| Candidate path | Graphics/erase/palette/last-row status |
| --- | --- |
| xterm, direct (exact build/configuration to be recorded) | **UNVERIFIED** |
| foot, direct (exact build/configuration to be recorded) | **UNVERIFIED** |
| WezTerm, direct (exact version/configuration to be recorded) | **UNVERIFIED** |
| Windows Terminal, direct (exact version/configuration to be recorded) | **UNVERIFIED** |
| Each candidate through tmux or another multiplexer | **UNVERIFIED** independently of direct-terminal results |

Do not turn on capability assertions merely because a candidate appears
in this table. A missing or failed condition keeps Auto on text; RequireSixel must
reject an unsupported configuration.

### Manual checklist for a prospective native configuration

- [ ] Establish the exact environment and selected fullscreen/inline lifecycle; verify
      Sixel DCS/RGB palettes reach the terminal through every intermediary.
- [ ] Before any reader starts, probe or validate measured cell pixels in
      1..=1024 with product at most 65,536, then store `Some(pixels)`. Do not round
      heights to complete six-row bands. After font/DPI/resize changes, verify
      that no automatic refresh occurs; update dimensions explicitly or restart.
- [ ] With actual partial-band metrics such as 10×20, verify exact raster height
      and transparent padding without adjacent-cell erasure. Check inline's blank
      guard rows (one at 10×20, none at 8×18, five at `h = 1`) prevent bottom
      scrolling; separately validate fullscreen's unchanged last-row requirement.
- [ ] Compare Fit/Stretch, all alignments, letterboxing, and corner pixels with
      the source; check black pixels are opaque and no background bleeds through.
- [ ] Exercise nested clipping, both scroll axes, screen edges, and the last row;
      ensure no painting outside cells, cursor-origin drift, or scrolling.
- [ ] In fullscreen, cover with opaque text and another raster, uncover, move,
      shrink, remove, and replace the source; verify old graphics are actually
      erased by ED 2 and the full text/raster scene is restored.
- [ ] Independently validate inline on the normal screen: put sentinel text and
      graphics above/below the owned rows, then move, shrink, remove, and replace
      raster content. EL 2 must erase only the `max(old, new)` inline extents,
      including guard rows, restore current text/raster content, and leave other
      rows intact; no ED 2 is allowed.
- [ ] Start inline near the bottom of the screen and grow its reserved height.
      Verify reservation precedes graphics output, existing graphics move with
      text scrolling, and relative cursor positioning does not drift. Inline owns
      the entire occupied rows, so do not place unrelated content beside it.
- [ ] Send subsequent tiles with different used palettes; confirm older tiles
      retain their colors. This is part of the `no_scroll` assertion's contract.
- [ ] Switch native to text and back in each mode; check mode-appropriate cleanup
      and stable source/layout. Without `reliable_inline_erase`, inline must use
      text for Auto or return a strict error for RequireSixel. With that assertion
      and all shared native gates satisfied, inline may emit Sixel without an
      alternate screen or `reliable_erase`. Native appended-newline output must
      remain rejected (or fall back under Auto).
- [ ] Verify empty/unknown capabilities stay on text. With other native conditions
      satisfied but `cell_pixels: None`, `Auto` must fall back and `RequireSixel`
      must error before frame output, without OS reads or queries. Text-only and
      unused raster paths must remain unaffected.
- [ ] Exercise visual effects and explicit text snapshots; do not silently drop
      content or retain unsupported native ownership under a character effect.
- [ ] Exit normally, interrupt execution, and inject recoverable writer failures;
      check cursor/style/modes, alternate-screen restoration, and graphics residue.
- [ ] In isolated startup-probe runs, exercise OS success, Unix/WSL fallback,
      fragmented replies, invalid replies, and no reply. Confirm prompt return
      on a valid reply and configurable timeout without assuming exact elapsed
      milliseconds. Check terminal/raw-mode restoration on success and failure.
- [ ] Exercise unrelated input and delayed replies: input may be consumed,
      no flush occurs, and replies may arrive after timeout. Do not assert event
      preservation. Start external readers only after probing completes.
- [ ] Verify probes are rejected after managed input initialization and remain
      rejected after loop exit. Exercise normal keyboard/paste/mouse input via
      native crossterm `EventStream`; rendering must never read input or re-probe.
      DA/Sixel capability negotiation remains unimplemented.

### Repeatable automated commands

```sh
cargo check -p rsille --examples
cargo check -p rsille --examples --features sixel
cargo test -p rsille --lib
cargo test -p rsille --lib --features sixel
cargo test --example sixel_windows --features sixel
cargo test --example sixel_windows --no-default-features
cargo run --example sixel_windows -- --headless
cargo run --features sixel --example sixel_windows -- --headless
cargo run --features sixel --manifest-path examples/ai-babies/Cargo.toml -- --sixel --headless
```

The finite `sixel_windows` headless mode checks mouse dragging and occlusion
without a terminal. Building it with `sixel` does not validate terminal behavior.
Treat recorded unit-test results and terminal-validation results as separate evidence.

### Interactive showcase

```sh
cargo run --release --features sixel --example sixel_windows -- --sixel

cargo run --release --example sixel_windows -- --text-only

cargo run --example sixel_windows -- --help
```

Two `floating_window` widgets in a `window_host` compare the same static procedural
ringed planet as native RGB raster and existing Canvas character sampling. Drag
titles to move and bottom grips to resize; click to bring forward. `e` cycles
Canvas Braille/HalfBlock/Quadrant, `r` restores starting rectangles, and `q`/Esc
quits. Prefer 100×32 cells; small sizes and terminal resizing are supported. There
is no live animation. This example is fullscreen-only and has no `--inline`
option. No flags shows help; `--text-only` is an explicit fullscreen preview.

For `sixel_windows`, `--sixel` asserts that the entire terminal path supports
graphics, non-scrolling positioned output and stable palettes, plus ED 2 graphics
erasure in fullscreen mode. Native output selects `RequireSixel`, never silent
fallback. Startup checks the feature and current
backend cell-metric limits (1..=1024 per dimension, at most 65,536 pixels) before
entering the managed terminal session. Without an override, startup explicitly
probes OS metrics, then the bounded Unix/WSL cell-pixel query, before input starts
and stores `Some(pixels)`. Native Windows has no query fallback. The query returns
on reply; its configurable timeout is not a guaranteed elapsed time. Optional
`--cell-pixels 10x20` bypasses probing and must describe
**actual font metrics**, not arbitrary dimensions;
there is no need to round the height to 18 or 24. Transparent partial-band padding
must be honored along with the erase/no-scroll assertions above;
see the [startup and safety contract](raster.md#run-the-showcase). These assertions
are user responsibility, not automatic negotiation or terminal certification.

For inline animation, use [AI Babies](../examples/ai-babies/README.md):

```sh
cargo run --release --features sixel --manifest-path examples/ai-babies/Cargo.toml -- --sixel
```

All its animation modes use the normal screen with inline output, no alternate
screen and no mouse capture. No additional `--inline` argument is needed.
`--sixel` selects `RequireSixel` and asserts reliable local EL 2 graphics erasure
and graphics moving with text during row-reservation scrolling, using
`reliable_inline_erase: true` and `reliable_erase: false`, alongside the shared
Sixel, palette and positioned-output requirements. `no_scroll` prohibits the
Sixel emission itself from scrolling, not controlled text scrolling for row
reservation. These are host assertions, not detection; `reliable_inline_erase`
remains false by default in the library, and fullscreen validation does not
establish inline safety.

`--height` remains 24/32 (default 24). Native inline content height is additionally
limited to the rows needed for the 320-pixel image and the terminal height after
deducting guard rows. With enough space, actual 10×20-pixel cells use **16 content
rows + 1 blank guard row**; 8×18 needs no extra row. The image is left-aligned and
scaled down for smaller terminals. The AI Babies
headless check above does not validate actual EL 2 cleanup, scrolling or terminal
compatibility; use the independent inline manual checklist above.

## Benchmark follow-up

Historical memory-writer results and a HEAD text/Canvas baseline are recorded in
[raster-performance.md](raster-performance.md). The old harness was removed at the
user's request; these are retained observations, not a currently runnable benchmark,
statistically controlled acceptance, or allocation profiling. `sixel_windows` is
an interaction showcase, not a replacement measurement harness, and no new
measurements are claimed. Future benchmarking needs a new controlled harness for
static raster beside changing text, moving but unchanged sources, scrolling,
continuous source changes, opaque overlays, and forced fallback. Repeat both cold
and warm cache runs at recorded dimensions and terminal/multiplexer versions.

Record source callback count/time, allocations and retained memory, sampling and
encoding CPU time, `raster_stats` counters, total transport bytes, and end-to-end
frame latency. Payload-only counters do not measure all terminal traffic, and
headless timings do not measure terminal parsing or display smoothness. Confirm
static unaffected tiles avoid re-encoding/resending without equating cache hits
with free movement or cleanup. Until controlled measurements and real-terminal
checks exist, the implementation makes no Sixel speedup, high-frame-rate, or
full-PRD completion claim.
