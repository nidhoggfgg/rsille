# Function Plotter

English · [中文](README_cn.md)

A small inline terminal math REPL: `meval` evaluates expressions and rsille renders RGB function graphs using Sixel. No symbolic algebra or equation solver; dependencies are local to this example.

## Run

From the repository root:

```sh
cargo run --release --features sixel --manifest-path examples/function-plotter/Cargo.toml -- --sixel
cargo run --release --features sixel --manifest-path examples/function-plotter/Cargo.toml -- --sixel --expr 'sin(x); cos(x); x^2/8'
```

No arguments prints help. Recommended terminal: 100×32 cells. Without `--expr`, startup opens a blank, borderless `In[1]:=` editor with no graph, history entry, or commit. An explicit `--expr` evaluates automatically and commits its graph or error, followed by a blank `In[2]:=` prompt. Each subsequent evaluation commits its `In[n]:=` input and `Out[n]=` graph (or error) to terminal history, then opens a new blank prompt. The editor uses the existing borderless `TextArea`, constrained to one row with no outline or bottom rule; editable and committed expressions both start immediately after `In[n]:=` on the same row. The editor and surrounding text use the terminal's default background, with an underlined caret/selection instead of background highlighting. The graph retains its own intentional background. `Out[n]=` (or the preview heading) is followed by a read-only formula area above the graph; the editable input remains linear. Pasted CRLF, CR, and LF line endings become spaces, preserving single-line input. There is no alternate screen, scrolling editable notebook, or saved session. Old results cannot be edited in place.

For a single plot in native mode, the formula and graph share `min(available columns, max(70, formula content columns))`. Formula content width includes padding and is rounded up to whole character cells; available width excludes the one-column padding on each side. The graph stays 14 rows high. This replaces the deliberate formula-only width policy: short formulas such as `sin(x); cos(x)` now get a useful 70-column graph when space permits, rather than a narrow strip. Narrow viewports still shrink both areas to fit. Unused space to the right retains the terminal background. Layout determines the raster dimensions before functions are sampled: allocated columns/rows multiplied by actual physical cell pixels. A 70×14 allocation therefore draws directly at 560×224 for 8×16 cells. There is no intermediate image or downsampling. For Cartesian-only inputs, the plot intentionally fills the available rectangle: the sampled bounds stay fixed across resize, but pixel aspect and x/y scale proportions can change with terminal size and cell metrics; equal mathematical units need not have equal screen lengths. Margins and grid density adapt to the pixel budget. Numeric 3×5 pixel labels appear only at sizes of at least 160×80; smaller plots omit labels and margins to prioritize curves. Curves use an approximately 1.5-target-pixel antialiased round stroke, blended over the existing grid, axes, and earlier curves. Coverage is accumulated per function before blending, so adaptive subdivision joints do not become brighter. Stroke caps/fringes extend up to 1.25 pixels beyond accepted endpoints; gaps smaller than the stroke footprint cannot be represented. The AA stroke itself does not increase function sampling or supersample/rescale the image.

UI state holds an immutable parsed plot source, not a pre-layout image. Its single-entry cache is keyed by bounds and target pixel size (source identity is implicit in ownership). Ordinary redraws and submission of an unchanged preview reuse it; resize redraws at the new resolution without reparsing. Bound `meval` closures are not `Send`, so the source uses UI-thread-local `Rc` sharing and `RefCell` caching, not a background task. Replacing the source or advancing to the next prompt releases the old cache.

`--sixel` is an explicit **user assertion** that the terminal and any multiplexer support Sixel, ED 2 and **EL2 graphics erase**, graphics following text scrolling, stable palettes, and non-scrolling positioned output. These capabilities are not detected. It sets `reliable_inline_erase` and rejects unsupported raster output, never silently falling back. **Image retention in scrollback is terminal dependent**; committing a frame does not guarantee Sixel images survive scrolling.

Cell pixels are measured before input starts; failure is an error. Override with `--cell-pixels WxH` only using **actual pixels per character cell**, not terminal columns/rows. Native Windows has no query fallback. Restart after font/DPI changes. See the [raster guide](../../docs/raster.md).

## Interactive independent panels

Start the normal REPL with `--sixel`; **no `--grid` is needed**.
Type each command below into the input line and press Enter:

```text
:grid
sin(x); cos(x)
polar(2, 0, 2*pi)
surface(x*y, -2, 2, -2, 2)
:submit
:plot
```

- `:grid` switches to an **empty grid composer**. Repeating it in grid mode keeps the panels.
- **Enter adds** the current expression as an independent panel and clears the input.
  Up to six panels, each up to 4096 bytes; semicolons still overlay curves within one panel.
- The draft previews as the next panel after 500 ms idle. Added panels remain visible;
  3D panels keep rotating while you type. Each panel has its own formulas, bounds and cache.
- **F5** adds a nonempty draft, then freezes and commits the whole grid at the displayed angle.
  With a blank input it commits just the added panels. **`:submit`** does the latter too.
  Invalid expressions, oversized input, and attempts to add a seventh panel keep the draft
  editable and do not commit. Empty grids cannot be committed.
- **`:undo`** removes the last added panel; **`:clear`** removes all panels.
  To replace the last panel, undo it, recall its expression with Up, edit and press Enter.
- After committing, a new empty grid composer opens; committed output never animates.
  **`:plot`** discards the uncommitted grid and returns to ordinary single-plot submission.
- Commands are not added to history. Successfully added expressions share the ordinary
  24-input history; recalling one previews a new panel, not an edit to an existing panel.
  Esc quits without committing. Nothing is saved across sessions.

## Legacy independent demo grid

```sh
# Four presets: shaded surface, helix, trefoil knot and polar rose
cargo run --release --features sixel --manifest-path examples/function-plotter/Cargo.toml -- --sixel --grid
# Each repeated --expr is a separate panel; semicolons remain overlays
cargo run --release --features sixel --manifest-path examples/function-plotter/Cargo.toml -- --sixel --grid --expr 'sin(x); cos(x)' --expr 'polar(2, 0, 2*pi)' --expr 'surface(x*y, -2, 2, -2, 2)'
cargo run --manifest-path examples/function-plotter/Cargo.toml -- --headless --grid
```

`--grid` remains a read-only CLI demo, separate from the interactive `:grid` composer.
Without `--expr` it supplies four presets; otherwise supply one to six repeated
`--expr` arguments (4096 bytes each). Errors identify the panel and reject startup.
Each panel owns its formulas, sampled bounds and render cache. 2D and 3D may coexist
in separate panels; existing within-panel overlay restrictions still apply.
`--x-range` supplies the same Cartesian sampling domain to all panels, but their
y bounds are independent. Use explicit `param`/`polar`/3D ranges for independent
domains; there is no per-panel Cartesian CLI range option.

The inline grid requests 30 rows, uses two columns at 80+ terminal columns and
stacks panels below that width. Cells divide the available height, with a one-column
gutter and a heading. Formula heights use measured native renders (up to 12 rows)
aligned to the tallest formula in each grid row.
Allocation reserves at least three graph rows when space permits; smaller panels
prioritize the graph over formulas. Long formulas and very small
viewports may clip; there is no scrolling or panel editor. Unlike ordinary REPL
graphs, grid graphs fill their allocated cells rather than using the single-plot preferred width.
Raster sizes are allocated cells × measured native cell pixels,
without intermediate resampling. Resize redraws each cache at its new size.
3D panels rotate together at 0.6 radians/second; 2D panels remain static.
**Enter/F5 freezes the displayed angle, commits one static grid, and exits**;
Esc exits without explicitly submitting. Sixel scrollback retention still depends
on the terminal. These exit-on-submit controls apply only to the CLI demo;
the interactive composer stays in the REPL.

## Two-dimensional formula output

Sixel previews and submitted results display powers as raised, smaller superscripts, stacked fractions, square-root radicals with overbars, functions, and nested grouping parentheses. `pi` and `theta` appear as π and θ in parsed formulas. Layout uses `meval`'s RPN tokens, not regex substitution; redundant source parentheses and numeric spelling may be normalized, and grouping can be more explicit than in the input. This is Mathematica-inspired presentation, not Mathematica syntax or symbolic typesetting.

`rsille-typst` typesets and antialiases formulas using Typst and the bundled New Computer Modern Math font (`typst-assets` 0.13.1). Font size follows cell height (12–40 pixels); formula rasters are copied at their original size, without rescaling. No external Typst/TeX executable, system font discovery, or runtime network access is needed. The first build is heavier because it includes Typst and font assets. Rows match semicolon curve colors. Polar output labels `r(θ)`; parametric output groups the `x(t)` and `y(t)` equations under a large left brace, with the typeset parameter interval to the right of the system. Polar intervals likewise follow the equation on the same line. The single-plot formula area and graph share a preferred width of at least 70 columns, expanding for wider native formulas and capped by available terminal space. Formula glyphs remain at native scale; extra width is background, not stretched text. Native output uses Typst's standard `cases` layout. Cartesian sampling still uses `--x-range`.

Native mode never falls back to source-expression text: successfully rendered formulas are copied at native scale and clipped to the available width/height. Expressions over 128 RPN tokens or Typst rendering/allocation failures show `[Formula rendering unavailable]` without rejecting valid plotting input. Each formula row has a 500,000-pixel budget. Formula height is capped at 12 terminal rows in native mode (grid panels may allocate less to preserve graphs); output beyond the viewport is clipped. There is no formula wrapping or horizontal scrolling. Input remains `meval` syntax, not Typst or LaTeX: only generated math from parsed tokens reaches Typst, with identifiers quoted to prevent code execution. Formula rows (including failed renders) are cached by cell height; ordinary resize only recomposes the target canvas, without recompiling. Unchanged preview submission reuses the canvas. Opening the next prompt releases per-source caches, but Typst maintains its own process-level caches.

## Controls

- The input uses the remaining row width. Longer expressions scroll horizontally to keep the caret visible; **Home / End** reveal the beginning/end. Hidden text remains part of the draft and is used in full for preview and submission. Widening the terminal reveals the full input when it fits (including a caret column).
- Editing or recalling history shows a live preview below the input after 500 ms idle, without submitting or adding history. Edits clear the old graph/error while waiting; empty input clears the preview. Preview errors do not block submission.
- In single-plot mode, **Enter / F5** immediately evaluates and submits the current input, without waiting for preview. Separate up to six functions with `;`; curve colors follow input order (cyan, orange, green, pink, purple, yellow).
- **Up / Down** recalls the newest 24 submitted inputs (including errors). Down past the newest entry restores your unfinished draft. Recalled inputs can be edited and submitted as new results; historical output is unchanged.
- **Esc** quits. Mouse capture is disabled; use the terminal's normal scrollback controls, not application scrolling or mouse buttons.
- Inputs are limited to 4096 bytes. Oversized edits show an error rather than evaluating a silently truncated prefix. Edit to correct the input, or submit the error and start afresh.
- Only the current result and bounded text input history are held by the application. After the next prompt opens, the old graph is released. Nothing is persisted after exit.

Expressions include `sin(x)`, `cos(x)`, `tan(x)`, `sqrt(x)`, `exp(x)`, `ln(x)`, `abs(x)`, powers `x^2`, and constants `pi`, `e`. Write multiplication explicitly: `2*x`. Try `1/x; tan(x)`, `sqrt(x); ln(x)`, or `sin(x)*exp(-x^2/20)`.

The graph includes grid lines, zero axes and numeric ticks. Budgeted adaptive midpoint subdivision skips nonfinite and unresolved intervals instead of joining suspected discontinuities. Finite sampling cannot prove continuity: narrow features and high-frequency oscillations may be missed. This is not a root finder, integrator, implicit plotter or symbolic solver. The x domain defaults to `[-10, 10]`; choose it explicitly with `--x-range MIN MAX`, e.g. `--sixel --x-range -2 4 --expr '0.05*sin(x)'`. Negative arguments are supported; endpoints must be finite and increasing with a finite span. Arbitrary expressions do not imply an x domain.

Y auto-scales once per parsed input from 1025 evenly spaced samples per function across that domain, independent of raster size. Nonfinite samples are ignored. Each function's sampled extrema are clipped to fences three interquartile ranges beyond its first/third quartiles, then the function ranges are combined and padded by 5% on each side. A zero interquartile range uses the quartiles themselves. This limits pole spikes but can also clip real narrow peaks or skewed tails; it does **not** find all global extrema. Constants receive 5% relative padding (zero uses ±1); all-nonfinite inputs use `[-1, 1]`. Zero is not forced into the y range. Ticks use range-dependent 1/2/5 steps and decimal/scientific labels for small scales. No additional y-range UI is provided. Mathematica's `Plot[...]` syntax is not supported.

## Polar and parametric curves

Mix these forms with `;` in one graph (up to six curves, colored in input order):

| Kind | Syntax | Free variable |
| --- | --- | --- |
| Cartesian | `sin(x)` | `x` |
| Polar | `polar(1+cos(theta), 0, 2*pi)` | `theta`, in radians |
| Parametric | `param(cos(t), sin(t), 0, 2*pi)` | `t` |

`polar(radius, start, end)` evaluates `(radius*cos(theta), radius*sin(theta))`, including negative radii. `param(x, y, start, end)` draws a 2D trajectory. Ranges are required: endpoints accept constant expressions such as `2*pi`, must be finite and increasing with finite span, and cannot contain free variables. Nested multi-argument functions work, e.g. `polar(max(1, cos(theta)), 0, 2*pi)`. `--x-range` controls Cartesian sampling only, not `theta` or `t`.

Paste these into the REPL for a rose, Archimedean spiral, cardioid, Lissajous curve, heart, lobed loop, and mixed plot respectively:

```text
polar(2*cos(5*theta), 0, 2*pi)
polar(0.12*theta, 0, 8*pi)
polar(1+cos(theta), 0, 2*pi)
param(sin(3*t+pi/2), sin(2*t), 0, 2*pi)
param(16*sin(t)^3, 13*cos(t)-5*cos(2*t)-2*cos(3*t)-cos(4*t), 0, 2*pi)
param((1+0.35*cos(7*t))*cos(t), (1+0.35*cos(7*t))*sin(t), 0, 2*pi)
sin(x); polar(2, 0, 2*pi); param(t, t^2, -2, 2)
```

For example:

```sh
cargo run --release --features sixel --manifest-path examples/function-plotter/Cargo.toml -- --sixel --expr 'polar(2*cos(5*theta), 0, 2*pi)'
```

When any polar/parametric curve is present, both axes are surveyed using 1025 samples per curve, accepting only points with two finite coordinates and applying the outlier fences and padding described above. The visible bounds then expand to give equal pixel-unit axes at the final plot size. Resizing may change visible bounds, but never parameter intervals or the Cartesian sampling domain in mixed plots. Circles stay circular using measured physical cell metrics. The grid remains Cartesian, not polar.

Geometric curves use budgeted adaptive 2D subdivision and clipping against all four viewport edges, rejecting nonfinite and unresolved intervals. Finite sampling can still miss high-frequency details, narrow peaks or tiny valid domains; this is not implicit plotting or symbolic solving.

## 3D surfaces and space curves

Use `surface(z_expr, xmin, xmax, ymin, ymax)` for a height field with variables `x,y`, or `param3(x_expr, y_expr, z_expr, tmin, tmax)` for a space curve with variable `t`. Bounds accept constant expressions and must be finite, increasing, with finite spans. Combine up to six 3D objects with `;`; mixing 2D and 3D is rejected. `--x-range` does not affect 3D sampling.

```sh
cargo run --release --features sixel --manifest-path examples/function-plotter/Cargo.toml -- --sixel --expr 'surface(sin(x)*cos(y), -pi, pi, -pi, pi)'
```

Try a saddle, Gaussian hill, helix, trefoil knot, or a curve passing through a plane:

```text
surface((x^2-y^2)/4, -3, 3, -3, 3)
surface(3*exp(-(x^2+y^2)/4), -4, 4, -4, 4)
param3(cos(t), sin(t), t/4, 0, 6*pi)
param3((2+cos(3*t))*cos(2*t), (2+cos(3*t))*sin(2*t), sin(3*t), 0, 2*pi)
surface(0, -2, 2, -2, 2); param3(cos(t), sin(t), t/4, -2*pi, 2*pi)
```

The software renderer uses an orthographic camera, equal world-unit scaling, opaque two-sided shaded triangles, a shared depth buffer for surfaces and curves, and bounding-box reference edges. Live 3D previews rotate around vertical z at 0.6 radians/second (approximately 20 fps); submission freezes the currently displayed angle permanently. Editing clears the old draft animation; already-added grid panels keep rotating. Clearing all panels or opening the next prompt cancels the old animation; 2D previews remain static. Framing stays fixed throughout rotation. Curves and reference edges use approximately 1.5-pixel antialiased round strokes with unioned joint coverage, retaining surface occlusion and nonfinite sampling gaps. Raster output requires Sixel. Formula output shows both surface domains or a three-equation parameter system.

Sampling is bounded: 65×65 vertices per surface and 2049 points per curve, evaluated at parse time and reused on resize. Nonfinite samples create gaps; finite discontinuities can still be bridged, narrow/high-frequency features missed, and large finite pole values can dominate framing. Coplanar depth ties depend on draw order. There are no camera controls, perspective, numeric axis labels, transparency, parametric surfaces, or implicit 3D equations yet.

## Stroke diagnosis

A thin non-antialiased diagonal can look periodically dotted even when its pixels are 8-connected (touching at edges or corners). Before the stroke change, `y=x` passed source-raster and presentation-sampler connectivity tests at a 70×14-cell target, including 18×40 and 20×40 native cell metrics (1260×560 and 1400×560 pixels). The library line routine advances every dominant-axis pixel; equal-size raster presentation does not resample. Its semantics are unchanged.

The Sixel-feature regression test reconstructs that original rounded-line path and also renders the new AA path through the actual renderer into memory. It decodes all 980 Sixel tiles, checks that every pixel is emitted exactly once and matches source RGB332 quantization, and verifies an 8-connected cyan curve at both metrics. AA connectivity uses substantial cyan coverage, not equality to the full-cyan color: partially covered pixels legitimately occupy other palette bins. Palette quantization can reduce fringe smoothness, but did not remove connectivity in these tests.

This evidence supports a one-pixel staircase/coverage explanation for the reported appearance, not missing source or encoded pixels at the tested dimensions. It is **not a reproduction of the screenshot**: its actual cell metrics and terminal behavior are unknown. Headless decoding cannot certify physical display behavior.

## Validate

```sh
cargo test --manifest-path examples/function-plotter/Cargo.toml --features sixel
cargo test --manifest-path examples/function-plotter/Cargo.toml --no-default-features
cargo run --manifest-path examples/function-plotter/Cargo.toml --no-default-features -- --headless
cargo run --manifest-path examples/function-plotter/Cargo.toml --no-default-features -- --headless --grid
cargo run --manifest-path examples/function-plotter/Cargo.toml --features sixel -- --headless
```

Evaluation sets the final result state and calls `ctx.commit_frame(Msg::NextPrompt)`. Headless `Runtime::step` reports `committed` and retains the result frame, including raster metadata; only the next step resumes the continuation and opens the blank prompt. Terminal execution renders and commits historical output before continuing.

Grid ManualClock tests cover mode transitions, independent panels and source reuse, Enter-add/F5-submit, `:submit`, invalid/oversized/full-grid drafts, history, undo/clear/quit, stale timers and frozen raster snapshots across commits. Headless mode also exercises interactive grid composition and returning to single plots.

ManualClock tests cover the 499/500 ms preview deadline, rapid-edit coalescing, error recovery, history recall, and cancellation across submission/next prompt. Tests also cover blank/explicit startup, the borderless editor, x-range validation, small positive/negative amplitudes, constants, combined functions, nonfinite samples, small tick labels, expressions, domains, poles, repeated commits, input queued across a boundary, typing/focus after reset, bounded input history, Unicode limits, committed errors, graph release, exact target raster dimensions, Sixel presentation without resampling, curve connectivity and resolvable gaps, resize invalidation and cache reuse. Headless mode checks repeated success/error commits and prompt recovery without terminal probing or Sixel output; it does not certify terminal compatibility. The transitive `nom 1.2` dependency of `meval 0.2` currently emits a future-compatibility warning.
