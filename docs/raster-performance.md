# Historical raster performance smoke measurements

**The old `examples/raster_bench.rs` harness was removed at the user's request.**
The data below is preserved as historical evidence, not a currently runnable
benchmark. Its workload and timing descriptions refer to the removed harness;
recorded commands are provenance only and are unavailable in this checkout.
No new measurements were made for this replacement.

The current single-file root example is
[`examples/sixel_windows.rs`](../examples/sixel_windows.rs), a draggable two-window
Sixel-versus-Canvas showcase. See the [showcase guide](raster.md#run-the-showcase)
for startup, controls and finite headless checks. It is **not** a benchmark and
does not reproduce the historical workload or timings below.

The removed harness was a small, deterministic, dependency-free workload using
public `Draw`, `Canvas`, `RasterCanvas`, `RasterCell`, `Builder`, and
`Render::raster_stats()` APIs. It measured renderer-side work, **not terminal
performance**. All renderer output went to a counting memory writer; only plain
summary lines went to stdout. No Sixel DCS was forwarded to a terminal.

## Historical procedure (harness unavailable)

Each case had its own renderer, one measured cold frame, 15 unmeasured warmup
frames, and 1 through 10000 measured warm frames (default: 100). The default run
rendered 116 frames per case, 812 overall. Builds/runs used an external
120-second timeout; build elapsed time was separate from renderer measurements.
The finite harness had no internal wall-time deadline.

The feature-disabled run explicitly labeled **all raster cases as character
fallback**. Its `RasterStats` remained default/zero; those were not native Sixel
results.

## Workloads and timing boundaries

Every case draws into a `24×12` cell buffer. The same static text line and an
alternating `tick A` / `tick B` line are drawn below the image on every frame.
There is no randomness, input, sleeping, event loop, TUI layout, or terminal I/O.

| Case | Work per frame |
| --- | --- |
| `text` | Only the two text lines; one character changes each frame. |
| `canvas` | Existing Braille `Canvas`: fixed `40×32` sample grid with `(x+y)%3 == 0` set, drawn into `20×8` cells, plus text. |
| `static+text` | Unchanged RGB source and placement, plus changing text. |
| `moving+text` | Same source and full target; destination column cycles through 0, 1, 2. |
| `scroll+text` | Fixed `16×6` viewport into the same full `20×8` target. Original offsets cycle through `x=frame%5`, `y=frame%3`. |
| `updating+text` | Alternate the color of a `32×24` source rectangle each frame, including mutation and copy-on-write with old snapshots still alive. |
| `TextOnly+text` | Same static raster scene, explicitly forced to half-block character fallback. |

The RGB source is `160×96`: a two-color checkerboard and a diagonal line.
Presentation uses `Stretch` and a `20×8` full target. Native output uses
synthetic cell metrics `cw=8`, `ch=18`, so the full presentation is `160×144`
pixels. Moving changes only destination storage. Scrolling retains the same
source identity, version, full target, options, and original `RasterCell.x/y`
offsets; it does **not** resize the source or reinterpret the viewport as a new
target. The 15 warmup frames cover the entire scrolling offset cycle.

For the native cases, the memory-only fixture supplies `RequireSixel`, an
alternate-screen flag, and test-only assertions of Sixel, erase, and
non-scrolling behavior. These are **not verified capabilities of any real
terminal**. No terminal modes are entered, no capabilities are probed, and no
OS cell-metric lookup is needed. Do not reuse these assertions with stdout.

`Instant` measures **wall elapsed time**, not exact CPU time. The measured
region includes `Draw`, fallback sampling, buffer comparison, native planning,
encoding/cache work, serialization, byte counting, and renderer commit. Source
construction, renderer construction, printing CSV summaries, and final teardown
are outside it. “Cold” means a fresh renderer/cache, not a cold OS or CPU cache.
The `updating+text` mutation is inside the timed draw operation.

The writer observes each slice through `black_box`, counts its length, and
discards it. It has no output buffer growth, copying, system calls, or meaningful
flush cost. It cannot model terminal parsing, graphics rendering, transport
backpressure, SSH, multiplexers, or display latency.

## Output columns

- `wall_ms`: total wall time for the indicated phase.
- `wall_us_per_frame`: wall time divided by frames, not a latency percentile.
- `writer_bytes`: all bytes accepted by the writer during that phase, including
  characters, cursor moves, resets, clearing, and Sixel payloads.
- `bytes_per_frame`: writer bytes divided by frames.
- `encodes`, `cache_hits`, `tiles_sent`, `sixel_bytes`: phase deltas from public
  `RasterStats`, excluding the unmeasured warmup. `sixel_bytes` is the delta of
  `RasterStats.bytes_sent` and excludes cursor/character overhead.
- `cache_bytes_end`, `cache_entries_end`: **end-of-phase occupancy**, not deltas,
  peaks, or total process memory. Warmup-created cache entries remain present.

An unchanged visible raster can bypass the encoding cache entirely: zero hits
alongside zero encodes is expected for `static+text`. Moving or scrolling can
avoid re-encoding while still sending substantial bytes. Updating source
versions invalidates cached tile keys even where individual pixels did not
change. The current cache limits occupancy; it does not eliminate update work.

Allocation counts, allocated bytes, RSS, and peak memory are **unmeasured**.
There is no custom global allocator and no unsafe code in this harness. Source
construction is also excluded from the text/Canvas timings, although the common
scene constructor prepares both sources before timing.

## Historical smoke measurements: before the diff fast-path optimization

The original 100-frame measurements below are retained unchanged. Throughout
this historical section, “current” means the working raster implementation
**before** the later text-only diff fast-path optimization, not the optimized
working tree. One observed run, not a statistical benchmark or performance
guarantee:

- Environment: Linux x86_64 under WSL2,
  `6.18.33.2-microsoft-standard-WSL2`; Intel Core Ultra 7 255H, 16 logical CPUs
  reported to the guest. No CPU affinity, frequency control, or isolation.
- `rustc 1.97.1 (8bab26f4f 2026-07-14)`, host
  `x86_64-unknown-linux-gnu`, LLVM 22.1.6;
  `cargo 1.97.1 (c980f4866 2026-06-30)`.
- Existing release profile, optimized with `lto = true`; default features empty.
  Current measurements enable `sixel`.
- Current source: working raster implementation atop HEAD
  `eb7ca784060d8b9d1ecf91ae135ba13f9f48f65f`, **not a committed raster revision**.
- Historical recorded command (removed target; not runnable now):
  `cargo run --release --features sixel --example raster_bench -- 100`.
  Build completed in 9.58 s within a 120-second tool timeout; this is separate
  from the frame timings below. The existing working `Cargo.lock` was used.

### Cold: one frame per case

All cold cache hits were zero. Native tiles sent equal encodes here; cache bytes
and Sixel payload bytes are equal on these first frames.

| Case | Wall µs | Writer bytes | Encodes / tiles | Cache bytes | Cache entries |
| --- | ---: | ---: | ---: | ---: | ---: |
| text | 19.499 | 371 | 0 | 0 | 0 |
| canvas | 16.967 | 691 | 0 | 0 | 0 |
| static+text | 537.961 | 12,664 | 160 | 10,540 | 160 |
| moving+text | 485.070 | 12,664 | 160 | 10,540 | 160 |
| scroll+text | 289.916 | 8,461 | 96 | 6,273 | 96 |
| updating+text | 486.398 | 12,697 | 160 | 10,573 | 160 |
| TextOnly+text | 38.356 | 3,471 | 0 | 0 | 0 |

### Warm: 100 measured frames after 15 warmup frames

| Case | Wall ms total | Wall µs/frame | Writer bytes total | Bytes/frame |
| --- | ---: | ---: | ---: | ---: |
| text | 0.213 | 2.133 | 1,200 | 12.0 |
| canvas | 1.061 | 10.605 | 1,200 | 12.0 |
| static+text | 1.485 | 14.852 | 1,200 | 12.0 |
| moving+text | 2.332 | 23.320 | 1,266,400 | 12,664.0 |
| scroll+text | 2.330 | 23.302 | 865,258 | 8,652.6 |
| updating+text | 58.534 | 585.339 | 1,267,900 | 12,679.0 |
| TextOnly+text | 1.122 | 11.220 | 1,200 | 12.0 |

| Case | Encodes | Hits | Cache bytes at end | Entries at end | Tiles sent | Sixel bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| text | 0 | 0 | 0 | 0 | 0 | 0 |
| canvas | 0 | 0 | 0 | 0 | 0 | 0 |
| static+text | 0 | 0 | 10,540 | 160 | 0 | 0 |
| moving+text | 0 | 16,000 | 10,540 | 160 | 16,000 | 1,054,000 |
| scroll+text | 0 | 9,600 | 10,540 | 160 | 9,600 | 646,458 |
| updating+text | 16,000 | 0 | 67,633 | 1,024 | 16,000 | 1,055,500 |
| TextOnly+text | 0 | 0 | 0 | 0 | 0 | 0 |

### Historical 100-frame HEAD baseline: completed for text and old Canvas

A read-only `git archive HEAD` was extracted into ignored
`target/raster-baseline`; no branch, worktree, commit, or working source change
was used. A copy of the harness was adapted **only inside that archive**:

- Retained the identical memory writer, frame dimensions, Canvas sample loop,
  Canvas draw rectangle, text workload, frame sequence, and timing boundaries.
- Kept only text and Canvas cases; removed RGB construction, raster imports,
  raster draw branches, raster output configuration, and raster-specific tests.
- Removed `raster_stats()` calls and CSV statistics columns because HEAD has no
  raster API. Missing statistics were not presented as measured zero values.
- Copied the working `Cargo.lock` into the archive to align dependency versions.
  An initial run without that lock resolved newer dependencies and is excluded.

The historical archive procedure copied the then-existing harness and working
lockfile into `target/raster-baseline`, applied the baseline-only adaptation
above, and built 100 measured frames with `--locked --release` into that archive's
`build` directory. The source harness has since been removed, so this procedure
is no longer available from the current checkout. Ignored archive/build artifacts
are not a maintained or guaranteed available baseline runner.

The locked baseline build completed in 9.31 s within its 120-second bound. That
archived HEAD had no `sixel` feature, so none was enabled; the historical working-tree
comparison included it.

| Case | HEAD cold µs | Current cold µs | HEAD warm ms total | HEAD warm µs/frame | Current warm µs/frame |
| --- | ---: | ---: | ---: | ---: | ---: |
| text | 18.018 | 19.499 | 0.151 | 1.510 | 2.133 |
| canvas | 16.248 | 16.967 | 0.951 | 9.506 | 10.605 |

Byte counts matched: text cold 371, Canvas cold 691, and 1,200 bytes across
100 warm frames for either case. The observed warm current-minus-HEAD deltas
were +0.623 µs/frame for text and +1.099 µs/frame for Canvas. These single-run,
sub-millisecond-to-millisecond samples do **not** establish a regression size
or causality; they certainly do not establish zero overhead. Native raster has
no HEAD counterpart and therefore no historical native comparison.

## Later follow-up: text-only diff fast path

The buffer follow-up hoisted the check that **both raster arrays are absent**
outside the entire `DiffIterator::next` loop. The following supplied measurements
compare the working raster tree immediately before and after that optimization,
**not HEAD versus the working tree**. There were three pre-optimization and
three post-optimization runs with 10000 measured warm frames per case (the
removed harness used the Cargo argument `-- 10000`), retaining the same
15 warmup frames. These are wall µs/frame, not exact CPU times.

| Case | Pre run 1 | Pre run 2 | Pre run 3 | Post run 1 | Post run 2 | Post run 3 | Pre median | Post median |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| text | 2.069 | 2.033 | 2.045 | 1.895 | 2.074 | 1.960 | 2.045 | 1.960 |
| canvas | 10.618 | 10.596 | 10.721 | 10.740 | 10.318 | 10.923 | 10.618 | 10.740 |

The reported output counters were identical before and after. The observed text
median moved from **2.045 to 1.960 µs/frame**, while the Canvas median moved from
**10.618 to 10.740 µs/frame**. Run-to-run variability is visible, including
post-optimization values on both sides of the pre-optimization ranges. These
samples do not establish a causal speedup or slowdown, a performance acceptance
result, or zero overhead. They do not replace the historical 100-frame data.
Allocation counts remain unmeasured.

### Comparable-length HEAD follow-up: three 10000-frame runs

The already-built archived HEAD executable described above was available and
was run three times sequentially, without rebuilding or changing the archive,
harness, or production source. Each historical invocation used the positional
argument `10000` (not a `--10000` flag). The archived executable is not a maintained
runner and is not guaranteed to remain available.

All three completed within a single 120-second command bound. Exact output,
including the run labels printed by the command wrapper:

```text
HEAD baseline run 1
# HEAD baseline: text and Canvas only; raster statistics unavailable; allocations unmeasured
case,phase,frames,wall_ms,wall_us_per_frame,writer_bytes,bytes_per_frame
text,cold,1,0.017,17.241,371,371.0
text,warm,10000,15.716,1.572,120000,12.0
canvas,cold,1,0.016,16.163,691,691.0
canvas,warm,10000,103.902,10.390,120000,12.0
HEAD baseline run 2
# HEAD baseline: text and Canvas only; raster statistics unavailable; allocations unmeasured
case,phase,frames,wall_ms,wall_us_per_frame,writer_bytes,bytes_per_frame
text,cold,1,0.032,31.707,371,371.0
text,warm,10000,16.209,1.621,120000,12.0
canvas,cold,1,0.016,16.343,691,691.0
canvas,warm,10000,101.212,10.121,120000,12.0
HEAD baseline run 3
# HEAD baseline: text and Canvas only; raster statistics unavailable; allocations unmeasured
case,phase,frames,wall_ms,wall_us_per_frame,writer_bytes,bytes_per_frame
text,cold,1,0.018,17.529,371,371.0
text,warm,10000,15.277,1.528,120000,12.0
canvas,cold,1,0.026,25.510,691,691.0
canvas,warm,10000,97.524,9.752,120000,12.0
```

| Case | HEAD warm median µs/frame | Post-optimization working-tree warm median µs/frame |
| --- | ---: | ---: |
| text | 1.572 | 1.960 |
| canvas | 10.121 | 10.740 |

These medians use the displayed per-run values. The HEAD runs each emitted
120000 bytes over 10000 warm frames for each case, with cold counts of 371 bytes
for text and 691 for Canvas. HEAD raster statistics remain unavailable, not
measured zero. Matching the iteration count makes this a more useful comparison
than mixing 100-frame and 10000-frame samples, but the HEAD and supplied
post-optimization runs were not interleaved or controlled as paired trials.
HEAD also lacks the current raster implementation and `sixel` feature, so this
comparison cannot isolate the diff-loop optimization. The variability and
absence of performance acceptance thresholds still apply.

## Limits and follow-up

There are **no performance thresholds yet**. The removed harness's tests checked
the counting writer, scroll metadata, character-only output without DCS,
static-frame skips, and moving-frame cache reuse, not timing. Both feature
configurations and the feature-disabled examples check passed in the historical
measured environment; this is not a claim about the replacement showcase's tests.

Future measurements require a new controlled harness. Repeat measurements across
processes, use larger frame counts where useful,
alternate baseline/current run order, and record distributions before setting
budgets. More workloads are needed for large sources, different viewport sizes,
Unicode/style-heavy text, cache eviction patterns, layout, and terminal output.
Different cases deliberately do different work and are not quality-equivalent
rendering comparisons. A small checkerboard, synthetic capabilities, and a
memory sink provide no evidence of real-terminal compatibility, cleanup,
scrolling safety, or terminal-visible throughput.
