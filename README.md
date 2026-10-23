# rsille v3

A Rust terminal application framework with composable widgets, Unicode editing,
layout, animation, background tasks, and braille graphics. v3 is under development;
this branch uses the API below. [中文](README_cn.md) · [Migration](MIGRATION_V3.md)

```toml
[dependencies]
rsille = "3.0.0"
```

```rust,no_run
use rsille::tui::prelude::*;

fn main() -> WidgetResult<()> {
    let exit = App::new(0)
        .mode(Mode::Inline)
        .update(|count, (), _ctx| *count += 1)
        .view(|count| button(format!("Count: {count}")).on_click(|| ()))
        .run()?;
    println!("Final count: {}; exit: {:?}", exit.state, exit.reason);
    Ok(())
}
```

Configure theme, subscriptions, hotkeys, clock, and executor before `.update()`.
All updates receive `UpdateCtx`: emit messages, schedule tasks or debounce, and
call `ctx.exit()` to finish. Fullscreen is the default; `Mode::Inline` uses the
normal terminal screen. Esc quits by default, after widget event propagation.
`run()` returns final state and an `ExitReason` after unmounting and cancellation.

Use `.update(update).view(view).build()` for a runtime without a terminal. Submit
input with `step(&events)`, send external messages with `handle().send(message)`,
and inspect `state()`, `frame()`, focus, widget state, tasks and layout geometry.
Use `runtime.into_exit()` to recover the final state. Testing clocks and executors
are explicit imports from `rsille::tui::runtime`.

Inside an existing Tokio runtime with I/O and time enabled, use
`.update(update).view(view).run_async().await`. The synchronous entry creates its
own runtime and reports an error if called inside Tokio. UI state and callbacks
can stay on the calling thread; they do not need `Send`, `Clone`, or `Debug`.
Messages need `Send + 'static` for runtime ingress, but not `Clone` or `Debug`.
`build()` stores owned callbacks and requires them to be `'static`.

Run examples from the repository:

```bash
cargo run -p tui --example hello
cargo run -p tui --example workflow
cargo run -p tui --example workflow -- --headless
cargo run -p tui --example composition
cargo run --manifest-path examples/rsille-showcase/Cargo.toml
cargo run --release --manifest-path examples/neon-observatory/Cargo.toml
```

The workflow example uses the same application for interactive editing and a
headless Unicode paste → debounce → background save → exit check, without sleeps.
See [widget examples and runtime contracts](packages/tui/examples/README.md).

Only **`rsille`** is published. Use its three public modules:

| Module | Purpose |
| --- | --- |
| `rsille::tui` | Application, widgets, layout and effects |
| `rsille::render` | Terminal buffers, Unicode text and rendering |
| `rsille::canvas` | Braille graphics, including `Canvas` |

The repository retains private `tui`, `render` and `canvas` workspace crates for
independent development and testing. All three have `publish = false`. Their thin
adapters compile the same implementation under `src/tui`, `src/render` and
`src/canvas` that the public `rsille` crate compiles directly as modules. There
are no internal crate dependencies in the published package and no generated or
duplicated implementation. v2 root-level imports have moved into these modules.

Default features are empty. Add `features = ["syntax-highlighting"]` to enable
Syntect highlighting in code viewers. Without it the same widgets display plain
text, retaining layout and keyboard navigation. The showcase opts into highlighting.

A standalone braille drawing uses the shared renderer:

```rust,no_run
use rsille::{canvas::Canvas, render};

let mut canvas = Canvas::new();
for x in 0..160 {
    canvas.set(x as f64, 15.0 + (x as f64 / 10.0).sin() * 10.0);
}
render::Builder::new().size((80, 10)).append_newline(true)
    .build_render(canvas, std::io::stdout()).render()?;
# Ok::<(), std::io::Error>(())
```

Validate the workspace and publish archives:

```bash
cargo test --workspace
cargo test --workspace --all-features
cargo check --workspace --all-targets --all-features
python3 scripts/check-release.py --offline
```

The release check uses Cargo's package verification for the single `rsille`
archive, tests its extracted contents, then runs an independent consumer with
default and all features. That consumer depends only on the extracted `rsille`
package and third-party libraries, and verifies the TUI workflow plus canvas/render
type interoperability. Remove `--offline` if dependencies are not cached. It never
uploads. See [migration and release details](MIGRATION_V3.md).

[MIT License](./LICENCE)
