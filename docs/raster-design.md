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

| Source | Responsibility / principal API |
| --- | --- |
| `src/render/raster.rs` | `Rgb`, opaque `RasterCanvas`, identity/revision COW snapshots, clipped integer primitives, `RasterOptions`, full-target `RasterCell::sample`, explicit-color `fallback` |
| `src/tui/widgets/display/raster_canvas.rs` | `RasterCanvasWidget`, `raster_canvas`, fixed-resolution `from_draw`, persistent revision-keyed callback state |
| `src/render/buffer.rs`, `chunk.rs` | Cell ownership, stored fallback, current/previous raster metadata, `Chunk::raster`, `Buffer::raster_at` / `raster_cells` |
| `src/tui/offscreen.rs` | Copy original raster metadata with cell blits; flatten to stored text for character effects and propagate the unsupported-composition marker |
| `src/render/raster_output.rs` | `RasterOutput`, `RasterPolicy`, host-supplied `RasterCapabilities`, `CellPixels`, `RasterStats`; conservative policy resolution |
| `src/render/raster_presenter.rs` | Per-renderer terminal-visible state, resource preflight, invalidation/erase plan, commit after successful flush |
| `src/render/sixel.rs` | Private, optional dependency-free RGB332 encoder and bounded encoded-tile LRU |
| `src/render/render.rs` | Character diff, ED 2/full-redraw cleanup, cursor positioning and Sixel writes; `set_raster_output`, `raster_stats` |
| `src/tui/runtime/core.rs` | `frame()` retains graphics and text; `frame_text_only()` makes an explicit character-only copy |

The core model, widget, and text fallback do not require the `sixel` feature.
`--features sixel` enables the private native encoder without adding an external
encoding dependency. It does not assert that the active terminal is supported.
Existing `Canvas` and `Encoding` semantics remain separate.

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
validity can require ED 2 and a full redraw. Merely adding disjoint coverage need
not erase old tiles. The presenter preflights a plan before frame output and
commits visible state only after flush succeeds. It marks native I/O uncertain
before writing; recoverable tile-write failure attempts DCS termination and
cursor/style restoration, and a subsequent frame clears uncertain coverage.
This is best-effort error handling, not a guarantee that an unavailable terminal
can be repaired.

Native emission is alternate-screen-only, disallows appended newlines, and requires explicit `sixel`,
`reliable_erase`, and `no_scroll` host assertions. ED 2 is relied on for graphics
cleanup only under that contract; neither space overwrite nor EL is treated as a
portable image deletion primitive. The terminal guard's ordinary alternate-screen
entry clear is separate from the presenter's graphics invalidation logic. Exit
uses the existing mode-restoration/alternate-screen lifecycle, not a universal
Sixel image-delete protocol.

## Input and metric discovery

There is **no active DA/CSI raster capability probe** in this release. Crossterm
does not route the required replies into this layer, and an independent stdin
reader would race the application event stream. No competing reader is started.
Terminal names/environment heuristics are not accepted as positive capability
evidence. Automatic negotiation needs a future coordinated input-parser design,
bounded timeouts, and tests for interleaved ordinary input before it can ship.

Physical cell metrics can come from an explicit `CellPixels` override or the OS
window-size API (`CellPixels::from_terminal`). The latter is a lazy syscall/API
lookup, not terminal escape-sequence negotiation. Automatic lookup is restricted
to raster frames with explicit Sixel capability and a non-TextOnly policy when no
metric override was supplied. It can refresh on later frames after resize.
Unavailable, zero, or non-integral metrics remain unknown and trigger fallback or
a strict error. Knowing pixel size does not establish Sixel support.

## Encoder and budgets

The encoder creates one independently positioned DCS tile per surviving raster
cell. Nearest-neighbor sampling uses full-target physical dimensions. Quantization
uses RGB332 (256 fixed colors); used palette definitions are emitted per tile,
with six-row bands and run-length compression. Black and letterbox pixels are
explicitly painted. Transparent *unset band bits* are an encoding detail, not
alpha in the content model. There is no final band advance past the tile.

The presenter nevertheless requires `cell_height % 6 == 0`: partial-band allocation
and bottom-edge behavior have not been validated on real terminals. Setting host
capabilities does not bypass this restriction or the other resource limits.

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
| Negotiation | No competing stdin reader and no active graphics queries by design | Coordinated response routing and timed negotiation |

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

Do not turn on the three capability assertions merely because a candidate appears
in this table. A missing or failed condition keeps Auto on text; RequireSixel must
reject an unsupported configuration.

### Manual checklist for a prospective native configuration

- [ ] Establish the exact environment and alternate-screen lifecycle; verify
      Sixel DCS/RGB palettes reach the terminal through every intermediary.
- [ ] Measure cell pixels, verify integral nonzero dimensions and the six-row
      height restriction; repeat after resize and font changes.
- [ ] Compare Fit/Stretch, all alignments, letterboxing, and corner pixels with
      the source; check black pixels are opaque and no background bleeds through.
- [ ] Exercise nested clipping, both scroll axes, screen edges, and the last row;
      ensure no painting outside cells, cursor-origin drift, or scrolling.
- [ ] Cover with opaque text and another raster, uncover, move, shrink, remove,
      and replace the source; verify old graphics are actually erased by ED 2 and
      the full text/raster scene is restored.
- [ ] Send subsequent tiles with different used palettes; confirm older tiles
      retain their colors. This is part of the `no_scroll` assertion's contract.
- [ ] Switch native to text and back; check full cleanup and stable source/layout.
      Inline must use text or return a strict error, never native output.
- [ ] Verify empty/unknown capability settings stay on text, and strict mode
      rejects missing safety conditions before frame output.
- [ ] Exercise visual effects and explicit text snapshots; do not silently drop
      content or retain unsupported native ownership under a character effect.
- [ ] Exit normally, interrupt execution, and inject recoverable writer failures;
      check cursor/style/modes, alternate-screen restoration, and graphics residue.
- [ ] Confirm ordinary keyboard/paste/mouse input is not consumed by discovery.
      Automatic graphics-response negotiation itself remains unimplemented.

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
is no live animation. No flags shows help; `--text-only` is an explicit preview.

`--sixel` asserts that the entire terminal path supports graphics, ED 2 graphics
erasure, non-scrolling positioned output and stable palettes. It selects
`RequireSixel`, never silent fallback. Startup checks the feature and current
backend cell-metric limits, including height divisible by six, before entering
terminal mode. The default uses OS-measured cell pixels. Optional
`--cell-pixels 8x18` must describe **actual font metrics**, not arbitrary dimensions;
see the [startup and safety contract](raster.md#run-the-showcase). These assertions
are user responsibility, not automatic negotiation or terminal certification.

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
