# TUI examples / TUI 小示例

English · [中文](README_cn.md)

Small, runnable examples for individual widgets and framework features. Larger tools,
games and graphics live in the [application gallery](../../../examples/README.md).
These files import the private `tui` workspace crate; applications outside this
repository should import `rsille::tui` instead.

## Start here

Run from the repository root. Cargo target names stay flat even though source files
are grouped in directories:

```bash
cargo run -p tui --example hello
cargo run -p tui --example workflow
cargo run -p tui --example workflow -- --async
cargo run -p tui --example workflow -- --headless
cargo run -p tui --example windows
cargo run -p tui --features syntax-highlighting --example code_viewer
```

| Example | Learn |
| --- | --- |
| `hello` | A minimal inline application; Esc exits |
| `workflow` | Unicode editing → debounce → background save → exit, interactively or headlessly |
| `headless` | Input replay, frame inspection and manual time without a TTY |
| `background` | External messages through an `AppHandle` |
| `composition` | Independent component instances, message mapping and scoped resources |
| `extensions` | Virtual logs, an anchored command palette and a world-coordinate canvas |
| `windows` | Floating windows, move/resize gestures and retained window state |

## All targets

| Directory | Example targets and source |
| --- | --- |
| `basics/` | [workflow](basics/workflow.rs), [hello](basics/hello.rs), [headless](basics/headless.rs), [background](basics/background.rs), [composition](basics/composition.rs), [extensions](basics/extensions.rs) |
| `layout/` | [flex](layout/flex.rs), [grid](layout/grid.rs), [overlay](layout/overlay.rs), [scroll](layout/scroll.rs), [split](layout/split.rs), [stack](layout/stack.rs), [windows](layout/windows.rs) |
| `widgets/` · Controls / 输入控件 | [button](widgets/button.rs), [calendar](widgets/calendar.rs), [checkbox](widgets/checkbox.rs), [radio_group](widgets/radio_group.rs), [select](widgets/select.rs), [switch](widgets/switch.rs), [text_input](widgets/text_input.rs), [textarea](widgets/textarea.rs) |
| `widgets/` · Collections / 数据组件 | [data_table](widgets/data_table.rs), [file_explorer](widgets/file_explorer.rs), [list](widgets/list.rs), [selection](widgets/selection.rs), [tree](widgets/tree.rs) |
| `widgets/` · Content / 内容展示 | [canvas](widgets/canvas.rs), [code_viewer](widgets/code_viewer.rs), [diff_viewer](widgets/diff_viewer.rs), [label](widgets/label.rs), [loading_indicator](widgets/loading_indicator.rs), [log_viewer](widgets/log_viewer.rs), [markdown_viewer](widgets/markdown_viewer.rs), [progress](widgets/progress.rs) |
| `widgets/` · Navigation / 导航 | [command_palette](widgets/command_palette.rs), [menu](widgets/menu.rs), [tabs](widgets/tabs.rs) |
| `widgets/` · Containers / 容器与装饰 | [collapsible](widgets/collapsible.rs), [dialog](widgets/dialog.rs), [divider](widgets/divider.rs), [panel](widgets/panel.rs), [spacer](widgets/spacer.rs) |
| `motion/` | [visual](motion/visual.rs) |
| `themes/` | [theme](themes/theme.rs) |

## Application and testing pattern

Use `App::new(state).update(update).view(view).run()`. Configure mode, theme, hotkeys,
subscriptions, clock and executor before `.update()`. Every update receives `UpdateCtx`;
`ctx.exit()` requests exit. Fullscreen is the default; `.mode(Mode::Inline)` uses the
normal screen. `.run_async().await` runs inside an existing Tokio runtime with I/O and
time enabled.

Use `.build().size(width, height)` for a runtime without a terminal. `step(&events)`
processes input, messages, timers and drawing. Inspect `state()`, `frame()`,
`focused_key()`, widget state and bounds to check behavior. For deterministic timing,
import `ManualClock` and `ManualExecutor` from `rsille::tui::runtime` (or `tui::runtime`
inside these examples). Advance the clock or execute a queued task, then call
`step(&[])` to collect changes.

```bash
cargo run -p tui --example headless
cargo run -p tui --example extensions -- --headless
cargo check -p tui --examples --all-features
cargo test -p tui
```

## Patterns worth carrying into your app

- **Stable identity:** give persistent editors and reordered items stable `.key(...)`
  values. Keys are unique within their keyed ancestor; unkeyed children use position.
- **Component composition:** `.map(...)` adapts child messages. Match view, effects and
  subscriptions with the same `.scope(...)` to cancel resources when an instance unmounts.
- **Retention:** a retained collapsed subtree preserves local state while remaining
  outside layout and input. Removing its description ends its lifetime.
- **Controlled input:** handle text/table changes in application state. Unicode editing
  uses grapheme clusters; layout and pointer positions share the text-width policy.
- **Pointer input:** hit testing follows clipping, overlays and modal isolation.
  Hover does not move keyboard focus; use `drag_region` for captured gestures.
- **Background work:** cancellation rejects stale results, but running blocking jobs
  must cooperate with cancellation. Manual time controls the scheduler, not arbitrary
  blocking code inside a task.
- **Graphics:** Canvas sample dimensions depend on the encoding. Use an explicit
  viewport and `ctx.world(surface)` when switching encodings without changing world extents.
- **Keyboard capabilities:** use `.keyboard_mode(KeyboardMode::Auto)`, `.focus_events(true)`
  and `.on_input(...)` for held-key applications. Track holds only when capabilities
  report key releases; clear them on focus loss. Legacy input is the default.

For the detailed contracts, see the source documentation in [app](../../../src/tui/app.rs),
[runtime](../../../src/tui/runtime.rs), [widgets](../../../src/tui/widget.rs),
[effects](../../../src/tui/effect.rs), [subscriptions](../../../src/tui/subscription.rs)
and the [migration guide](../../../MIGRATION_V3.md).

## Add a focused example

Add one source file under the appropriate category and register a `[[example]]` target
in [Cargo.toml](../Cargo.toml). Keep the example centered on a widget or feature; put
applications with multiple screens or domain workflows under root `examples/`.

[Project README](../../../README.md)
