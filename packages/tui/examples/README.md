# TUI Examples

These examples are intentionally small. Each file should demonstrate one TUI
widget or one framework feature, including its common variants. Larger app-like
showcases belong in the repository-level `examples/` directory as separate
projects.

## Unified Application Entry

Use `App::new(state).mode(Mode::Inline).update(update).view(view).run()`.
The default mode is fullscreen. Every update receives `UpdateCtx`, including
simple updates that ignore it. Configure theme, subscriptions, keys and runtime
hooks before `.update()`. `ctx.exit()` requests exit; the returned `AppExit`
contains final state and an `ExitReason`. `build()` on the same builder creates
a headless runtime; its later `run()` uses the configured mode. `into_exit()`
recovers state after headless execution.

Use `.run_async().await` inside a Tokio runtime with I/O and time enabled.
Synchronous terminal entry reports an error inside Tokio before changing terminal
state. Terminal update/view closures may borrow local data; `build()` stores
`'static` closures. UI state need not be `Send`, `Clone`, or `Debug`, and messages
need only `Send + 'static`. Clock/executor tools live in `tui::runtime`, outside
the common prelude. Legacy `App::run*` / `build*` are compatibility adapters;
see [migration details](../../../MIGRATION_V3.md).

`workflow` runs a complete Unicode edit → debounce → background save → exit flow:

```bash
cargo run -p tui --example workflow
cargo run -p tui --example workflow -- --async
cargo run -p tui --example workflow -- --headless
```

Both modes use the same state, update and view. The headless branch injects a
manual clock/executor, checks focus, saved state, task completion and exit reason,
without sleeping. `cargo test -p tui --test api` runs this exact flow.

Default features are empty. For code highlighting, run
`cargo run -p tui --features syntax-highlighting --example code_viewer`.
The code viewer otherwise displays plain text with the same navigation.

## Categories

- `basics/`: minimal first-run examples.
- `layout/`: layout functions and containers such as flex, grid, overlay, scroll, split, and stack.
- `motion/`: animation and visual-effect wrappers.
- `themes/`: theme and styling system examples.
- `widgets/`: focused examples for built-in widgets.

## Running Examples

Example names stay flat even though files are grouped in subdirectories:

```bash
cargo run -p tui --example hello
cargo run -p tui --example headless
cargo run -p tui --example background
cargo run -p tui --example composition
cargo run -p tui --example flex
cargo run -p tui --example grid
cargo run -p tui --example button
cargo run -p tui --example text_input
cargo run -p tui --example data_table
cargo run -p tui --example command_palette
cargo run -p tui --example visual
```

Use this to check the full set:

```bash
cargo check -p tui --examples
```

## Driving An Application Without A Terminal

`headless` demonstrates `.update(update).view(view).build()`, `Runtime::step` and
`ManualClock`. It runs without a TTY. `step(&events)` dispatches input, processes
messages and due timers, updates animations and draws into `Runtime::frame()`.
The frame exposes cells and styles; `state()`, `focused_key()`, `bounds(path)`,
`widget_state(id)` and `tasks()` expose state for assertions. The default buffer
size is 80 × 24; use `.size(width, height)` or submit `Event::Resize` to change it.

`App::with_clock` injects scheduler time. `ManualClock::advance` only changes
time: call `step(&[])` afterwards to process it. Delays, debounce, ticks, frame
callbacks, retry waits, task timeouts and animations use this clock. For task
tests, inject a `ManualExecutor`, call `run_next()` to complete a queued job,
then call `step(&[])` to collect results. Task bodies, including `TaskContext::sleep`,
remain blocking code; virtual time controls the scheduler around them.

`background` obtains an `AppHandle` before calling `Runtime::run()`. External
messages and task results enqueue work and wake the same terminal event loop.
An idle app waits for input or a wake signal. Timers and animations provide
deadlines; frame callbacks request updates at up to 60 Hz. Existing `App::run*`
entry points use this same core. Use the default `SystemClock` with the terminal
adapter; manual clocks are intended for explicit stepping.

The ingress queue is currently unbounded. Each turn drains at most 256 ingress
events and handles at most 256 application messages, retaining the rest for the
next turn so input and rendering can proceed. Replacing a debounce drops its
previous message and deadline, without spawning a waiting thread.

Timeouts begin when an attempt is submitted to the executor. At each update,
expired deadlines are processed before incoming results; a result not observed
before its deadline is rejected. Cancellation and timeout release scheduler
resources and reject later results. Blocking jobs receive a cancellation token
and must cooperate to stop executing. Shutdown and dropping the runtime signal
cancellation without joining arbitrary worker threads. The default executor
preserves the existing thread-per-task policy; an embedding host can inject an
executor with its own concurrency limits. Async task APIs and a shared default
pool are follow-up work beyond the current runtime. State-driven subscriptions use
the same blocking executor and cancellation contract described below.

## Widget Identity And State Lifecycle

The runtime reconciles descriptions before measuring or drawing. Widget state
matches by `WidgetId` and the concrete component's `TypeId`. Named keys are unique
within the nearest keyed ancestor, including descendants below unkeyed wrappers
and explicitly retained children. Duplicate identities panic with both tree paths.
Unkeyed children use positional identities. A keyed descendant preserves state
and focus when reordered or moved beneath unkeyed wrappers. Changing a key creates
a new lifetime; changing the root key resets the entire described tree.

The same identity and component type preserve local state; a different component
type gets fresh state, even if both components use the same state struct. Boxing a
widget transparently forwards its component type. One component currently owns
one state slot: use an aggregate struct for multiple pieces of local state.

Custom widgets implement `Widget::lifecycle(&mut StateCtx)`:

- `Lifecycle::Mount` initializes local state and synchronizes initial properties.
- `Lifecycle::Sync` synchronizes application-controlled properties before input,
  measurement and drawing. It can run multiple times and must be idempotent.
- `Lifecycle::Unmount` runs once at logical removal, type replacement, key change,
  shutdown or runtime drop. Release interactive resources here. State is dropped
  immediately unless an exit animation still needs it for drawing.

`StateCtx` exposes the widget's path, identity and typed state access. Text input
and text area use this protocol to synchronize controlled values, repair cursor
UTF-8 boundaries and clamp scrolling. Multiple edits in one input batch share the
local editing value; synchronization runs outside that batch. Applications can
accept, transform or reject edits in their update function. The former public
`modified_this_batch` fields have been removed; custom widgets should synchronize
through the lifecycle protocol instead of requiring runtime-specific resets.

Unmounted widgets stop receiving focus, input and component animation updates.
An exit renderer owns their retained state and frozen animation channels, with
an independent exit timeline. A remounted identity starts fresh and cannot
restart an older exit. Completing each exit drops its state and animation
channels. A keyed child moved to another live branch remains mounted; its old
ancestor does not duplicate it in an exit animation. The retained previous
description tree is released when its last exit finishes.

Hidden children are unmounted by default. To preserve a collapsed page's local
state, keep its child descriptions present and opt in explicitly:

```rust,ignore
collapsible("Editor")
    .expanded(state.show_editor)
    .retain_state(true)
    .child(textarea().key("editor").value(&state.text))
```

A collapsed retained subtree still synchronizes properties, but does not
participate in layout, focus, event routing, drawing or animation updates.
Retention is bounded by the children in the current description, without a
historical page cache, and ends when those children or their owner are removed.
Custom containers can provide the same policy through `Widget::retained_children`;
its children must be disjoint from `children()`, and their positional indices
follow the visible children. Prefer stable keys for children that switch between
the two lists. App tasks retain their application lifetime. Explicitly retained
scopes keep their tasks and subscriptions running; removal of the description
cancels them. Exit rendering keeps visual resources only, never live producers.

## Message Mapping And Component Scopes

`composition` opens an interactive terminal UI with two editor instances. Each
has an input, a 500 ms debounce save, a 3-second background load and a subscription
that updates once a second. Tab/Shift+Tab move focus; Enter in an input starts a
load. F1/F2 or the buttons remove/remount an instance, cancelling its resources;
remounting starts fresh. F3 swaps their positions while keeping focus and state.
Esc exits. Run it with `cargo run -p tui --example composition`.

The module defines its own state, messages, update returning `Effect<ChildMsg>`,
and view. Only the parent knows its top-level message enum:

```rust,ignore
// View: map the entire subtree and establish an instance's lifetime namespace.
editor::view(&state.editor).map(Msg::Editor).scope("editor")

// Update: map messages, task results and request/task status callbacks.
ctx.dispatch(editor::update(&mut state.editor, message)
    .map(Msg::Editor).scope("editor"));

// App builder: describe the continuous sources currently wanted by the state.
App::new(state).subscriptions(|state| {
    editor::subscription(&state.editor).map(Msg::Editor).scope("editor")
})
```

Import `WidgetExt`, `Effect`, `Scope` and `Subscription` from the prelude. `map`
is transparent to widget identity and component type: it preserves measurement,
layout intent, retained children, focus, event capture/target/bubble, propagation
outcomes and animation declarations. Chained maps compose. It adds no node to
the tree and does not isolate keys by itself.

`WidgetExt::scope("name")` adds a named container and a resource namespace. Give
each module instance a distinct name, and use the same namespace for its effects
and subscriptions. Local widget keys, task keys, debounce keys and subscription
IDs then remain independent. A nested view `.scope("inner").scope("outer")`
matches effects/subscriptions with the same chain, or
`.scope(Scope::new("outer").child("inner"))`. Names are separate segments:
`"outer/inner"` is a single name, not that nested scope. Duplicate widget identities, scope paths and subscription
IDs produce diagnostics. Scope names appear in `TaskStatus::scope`; inspect a
local task with `UpdateCtx::task_status_in_scope(&scope, "load")`.

A scoped resource starts only while its view scope is mounted. Logical unmount,
owner replacement or a root key change cancels its tasks, retries, delays,
debounce waits and subscriptions immediately, including during exit animations.
Queued and late results from that lifetime are discarded even if the same scope
is remounted in the same update batch. Unscoped effects have application lifetime.
Cancellation remains cooperative for running blocking code; producers should
check `TaskContext::is_cancelled` or its token. `Effect::cancel_task_key` and
`cancel_debounce` resolve within their effect's scope. Explicit task IDs can be
cancelled from the application or their own scope.

`Subscription::run("id", producer)` runs on the configured executor. A producer
can emit repeatedly through its task context. The builder's resolver runs at
initial mount and after state updates. An unchanged `(scope, id)` keeps the
existing producer, rather than replacing its closure or restarting it on every
render. Completed/failed producers also remain matched: remove and re-add the
ID to restart, or change the ID when its source configuration changes. Removing
a description cancels its producer; shutdown/drop cancel all remaining work.
Subscriptions and one-shot task keys use separate namespaces. Use `batch`, `map`
and `scope` to compose subscriptions. `ManualExecutor` lets tests drive bounded
producers without threads; long-lived blocking loops need a suitable executor.

Mount/unmount and resource reconciliation happen after each application message
so a removal invalidates later queued results in that batch. Controlled widget
properties synchronize at batch end, preserving batched text editing. Due timer
messages retain the scheduler's existing ordering before newly received ingress;
an event processed before removal belongs to the still-mounted lifetime.

## Adding A New Example

1. Add one file for one widget or feature.
2. Show important variants in that file, but avoid building a full app.
3. Add a matching `[[example]]` entry in `packages/tui/Cargo.toml`.
4. If the example needs multiple screens, domain data, or orchestration, create
   a separate project under the repository-level `examples/` directory instead.


## Unicode text, cells, and editing

Stage four uses extended grapheme clusters for terminal cells and editing. A
combining accent (`e\u{301}`), skin-tone emoji (`👍🏽`), flag (`🇨🇳`), or ZWJ
sequence (`👩🏽‍💻`) is one editing unit. `Stylized::new(char, style)` remains
available for scalar glyphs; `Stylized::grapheme(text, style)` stores the first
complete cluster. `Chunk::set_string` writes all clusters and clips at a whole
cluster boundary. Empty, control, and standalone zero-column clusters do not
replace existing cells or send controls to the terminal.

`render::text` (re-exported as `tui::text`) provides `Span`, `Line`, `Text`,
`TextMetrics`, grapheme boundaries, and source-line/column conversions. Rich
text can be painted with `Chunk::set_line` or `Chunk::set_text`. A cluster may
cross span boundaries; its first byte determines the style. `Span` and `Line`
are aliases for the existing `StylizedText` and `StylizedLine` types.

```rust
use tui::prelude::*;

let metrics = TextMetrics {
    ambiguous_width: AmbiguousWidth::Narrow,
    tab_width: 4,
};
let theme = Theme::dark().with_text_metrics(metrics);
// Pass theme to App::with_theme, or configure a standalone Buffer before drawing.
assert_eq!(metrics.width("e\u{301}👩🏽‍💻中"), 5);
assert_eq!(metrics.truncate("e\u{301}👩🏽‍💻中", 3), "e\u{301}👩🏽‍💻");
assert_eq!(metrics.clip("中x", 1, 2), " x");
```

The default policy uses narrow ambiguous characters and four-column tab stops.
A zero tab interval is treated as one. Tabs expand relative to the start of the
text line; column positions inside tabs or wide clusters map to the leading
byte. `TextMetrics::wrap` returns source byte ranges and widths for visual
lines. `byte_to_line_column` / `line_column_to_byte` refer to explicit source
lines; use the ranges from `wrap` to locate wrapped visual lines. Width tables
come from `unicode-width`; segmentation comes from `unicode-segmentation`.
Terminal-specific emoji rendering still depends on the terminal/font. Set
`AmbiguousWidth::Wide` when appropriate for the terminal. Borders use ASCII
when the selected box-drawing glyphs would occupy more than one column.

The runtime passes the theme policy to its buffer and to scroll/visual
buffers. Labels, lists, tables, and both editors share the same policy for
width, truncation, painting, and cursor placement. Custom widgets can read
`MeasureCtx::theme().text_metrics`, `RenderCtx::theme().text_metrics`,
`EventCtx::text_metrics`, or `Chunk::text_metrics`. Override
`Widget::layout_style_with` when intrinsic constraints depend on the active
policy; its default delegates to `layout_style`. Changing a buffer's policy
clears its content and invalidates diff history, so configure it before painting.

Both editors use `EditBuffer`. Public cursor and selection-anchor positions
are UTF-8 **byte offsets at grapheme boundaries**. Lifecycle synchronization
repairs both positions before input or painting after an external value change.
Left/Right, Backspace/Delete, Home/End, Shift+navigation, and Ctrl+A operate on
this shared model. Selection replacement is atomic. TextArea's Up/Down and
mouse clicks use terminal columns; horizontal viewports preserve partial wide
cells as spaces. Password inputs show one mask per grapheme.

`Event::Paste` inserts one edit and never submits. CRLF and CR normalize to LF;
a single-line input replaces LF with spaces, while TextArea keeps LF. Other
control characters are removed, with tabs retained. Raw terminal sessions
enable bracketed paste and disable it on cleanup, including partial setup
failure. Word movement and undo/redo remain future editing extensions.

Migration notes:

- Replace `cell.content.c` with `cell.content.symbol()` to read full clusters,
  or `as_char()` for scalar-only access. Use `set_char` / `set_grapheme` to
  replace content; `as_char()` returns `None` for a multi-scalar cluster.
- `Stylized` and `CellSample` now implement `Clone` without `Copy`. Single
  scalars are inline; composite clusters use `Arc<str>` so cloning frames and
  samples shares their bytes. `StylizedText::len` now counts graphemes.
- `Buffer` records the actual width and owner of each continuation cell.
  Replacing a lead or continuation clears the entire previous glyph, including
  obsolete tails. A wide glyph cannot cross a row. Both cell and inline line
  diffs repaint vacated columns. Scrolling and blitting turn partial edge
  glyphs into styled spaces.

Run `cargo test -p render --test unicode` and `cargo test -p tui --test unicode`
for headless regressions. `cargo bench -p render --bench buffer_bench --
unicode_cell` compares scalar/composite construction, cloning, an always-shared
allocation baseline, and an 80×24 mixed-text draw/diff scenario. The baseline
only measures allocation; the grapheme constructor also performs segmentation.
A short local run measured approximately 12 ns for ASCII construction, 1.3 ns
for its clone, 24 ns for a composite clone, and 70 µs for the draw/diff scenario;
these are observations, not performance guarantees or cross-machine baselines.

## Layout phases and invalidation

The runtime reconciles the description and widget state, resolves the target
layout tree, then paints it. `Widget::layout(area, &MeasureCtx) -> Layout`
allocates children without drawing. `ChildLayout::index` addresses
`Widget::children()`, and the order of allocations is paint/hit order.
`Layout::with_data` can retain immutable container geometry for painting,
such as a scroll viewport or split divider. Flex, Grid, Panel, Dialog,
Collapsible, Stack, Overlay, Split and ScrollView use this protocol.

A custom container only needs to allocate its children and call
`ctx.render_children(chunk, self)` in `render`. The framework applies keys,
scopes, layout animations, clipping and hit routing:

```rust,ignore
fn layout(&self, area: Area, _ctx: &MeasureCtx) -> Layout {
    Layout::new([ChildLayout::new(0, area.shrink_saturating(1, 1, 2, 2))])
}

fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
    ctx.render_children(chunk, self);
}
```

The default `layout` stacks all exposed children in the allocated area. Leaves
need no change. A legacy custom container that computes geometry in `render`
continues to work, but must adopt this protocol to reuse its own layout.
`Box<dyn Widget<_>>` and message mapping forward the layout hook transparently.
Direct rendering with `RenderCtx::new` computes layout on demand and requires
no runtime or terminal session.

The runtime caches each node's layout output by path and allocation. A new
view description, changed theme (including text metrics), or explicit layout
request clears the cache. Resizing recomputes allocations whose constraints
changed. Time/frame metadata is available during measurement, but a custom
widget that depends on it must request layout when those dependencies change.
There is no general property hashing, persistent Taffy tree, or measurement
memoization yet. Application messages conservatively rebuild the description
and invalidate layout, including messages that change only colors.

| Context | Request | Behavior |
| --- | --- | --- |
| `EventCtx`, `StateCtx`, `AnimationCtx` | `request_layout()` | Remeasure/layout and paint |
| `EventCtx`, `StateCtx`, `AnimationCtx` | `request_paint()` | Paint using existing allocations |
| `EventCtx`, `AnimationCtx` | `request_next_frame()` | Schedule another update; no implicit paint/layout |
| headless `Runtime` | `request_layout()` / `request_paint()` | Wake and refresh on the next `step` |

`EventCtx::state_mut` requests paint. If internal state changes intrinsic size
or child allocations, call `request_layout` as well. Lifecycle synchronization
has the same explicit requirement for layout-dependent internal state. Existing
application-controlled properties are covered by description rebuilding.
Animation hooks should use the three independent requests and return `false`;
returning `true` keeps the legacy behavior of paint plus another update.
Color animations and cursor movement reuse layout. Geometry animations refresh
affected descendant allocations when their displayed size/position changes;
this refresh may happen during paint after the target layout phase.

`Runtime::bounds(path)` reports the last visible screen bounds.
`Runtime::layout_geometry(path)` additionally exposes the full target and
displayed allocations, visible clip and hit area. Target/display use the
parent's layout space; clip/hit are screen coordinates. Scroll content includes
`coordinate_offset` to map its offscreen layout space to the screen.
`EventCtx::bounds()` preserves the complete displayed allocation, so partial
clipping does not shift local input positions. Use `local_mouse_position` for
widget-relative coordinates or `layout_mouse_position` for coordinates in the
same space as `bounds`; `mouse_position` remains the absolute screen position.
`bounds_for` reports visible screen bounds. Hits are clipped to ancestor
viewports and respect the animation's `HitTestMode`.

`Runtime::layout_stats()` exposes cumulative layout computation/cache-hit
counts (including leaves). Run `cargo test -p tui --test layout` for headless
regressions covering custom/mapped containers, independent animation requests,
color animation reuse, resize/wrapping, split resizing, scroll clicks, animation
geometry and clipping. Run `cargo bench -p tui --bench layout` to compare the
same 64-label scene over 2,000 warmed frames: measurement count, layout count,
allocation calls (including reallocations), and mean frame time. The benchmark
compares explicit paint requests against explicit full layout requests; it
excludes terminal output and view rebuilding. Use it as a local baseline before
adding more cache machinery, not as a cross-machine performance guarantee.

A local optimized run on 2026-10-22 produced the following means; these are
observations, not thresholds:

| Update | Measurements/frame | Layout nodes/frame | Allocations/frame | µs/frame |
| --- | ---: | ---: | ---: | ---: |
| Paint only | 0 | 0 | 256 | 183.2 |
| Layout + paint | 320 | 73 | 3,115 | 851.3 |
