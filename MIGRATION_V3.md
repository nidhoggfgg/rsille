# Migrating to rsille v3

v3 is under development. These contracts describe this branch; `3.0.0` is the
release version configured for `rsille`. Package checks do not publish it.

## Application entry and exit

Set all application configuration first, then install the update and view:

```rust,no_run
use rsille::tui::prelude::*;

let exit = App::new(String::new())
    .mode(Mode::Inline)
    .update(|state, message: String, ctx| {
        if message == "quit" {
            ctx.exit();
        } else {
            *state = message;
        }
    })
    .view(|state| text_input().value(state).on_change(|text| text))
    .run()?;
assert!(matches!(exit.reason, ExitReason::Requested | ExitReason::QuitKey | ExitReason::InputClosed));
# Ok::<(), WidgetError>(())
```

| Earlier v3 entry | Preferred entry |
| --- | --- |
| `App::run(update, view)` | `.update(update_with_ctx).view(view).run()` |
| `App::run_inline(update, view)` | `.mode(Mode::Inline).update(update_with_ctx).view(view).run()` |
| `App::run_with_effects(update, view)` | `.update(update).view(view).run()` |
| `App::run_inline_with_effects(update, view)` | `.mode(Mode::Inline).update(update).view(view).run()` |
| `App::build(update, view)` | `.update(update_with_ctx).view(view).build()` |
| `App::build_with_effects(update, view)` | `.update(update).view(view).build()` |

Wrap a two-argument update as `|state, msg, _ctx| update(state, msg)`. The original
`App::run*` and `App::build*` methods remain compatibility adapters, using the same
core. Legacy terminal methods still return `WidgetResult<()>` and select their
named mode. The fluent entry and `Runtime::run` / `run_inline` / `run_async` return
`WidgetResult<AppExit<State>>`; callers discarding the result can use `.map(|_| ())`.
`Runtime::run` respects the mode configured before `build`.

`ctx.exit()` (or `Effect::Exit`, including mapped child effects) ends processing
after the current update. Remaining messages and effects in that batch are
discarded. Logical unmount runs once, tasks and subscriptions receive cancellation,
timers and animation resources are released, and late results are rejected.
`AppExit` moves out final state without a `Clone` or `Default` requirement.

Exit reasons distinguish update requests (`Requested`), the configured quit key
(`QuitKey`), host shutdown (`Shutdown`), and terminal input closure (`InputClosed`).
I/O/setup errors return `WidgetError`. `Runtime::exit_reason` inspects an exit;
`Runtime::shutdown` cancels without consuming state; `into_exit` finishes cleanup
and returns it, preserving an earlier exit reason. Dropping a runtime cancels
resources without returning state or waiting for arbitrary worker threads.

## Existing async runtimes and trait bounds

Use `.run_async().await` from an existing Tokio runtime with time and I/O drivers
enabled. It owns the same terminal session guard as the synchronous adapter and
restores terminal settings on completion, error, or future cancellation. It runs
on the calling thread, so do not spawn its future with `tokio::spawn` if the UI
contains non-`Send` state. Dropping an in-flight future drops the app; no final
state is returned in that case. One terminal session should run at a time.

Synchronous `.run()` creates a private runtime and returns an error before terminal
setup when called inside Tokio. Headless `.build()` / `.step()` can be embedded
without any Tokio runtime. This does not add async task bodies: `Task` and
`Subscription` still use the injected blocking `Executor`, with cooperative cancellation.

Application configuration has no blanket message bounds. Execution requires
`M: Send + 'static` for task/external ingress. Messages no longer need `Clone` or
`Debug`. State and UI closures do not need `Send`, `Clone`, or `Debug`. Direct
terminal entries accept borrowed update/view closures; `build` requires `'static`
closures because it stores them. Views still return owned widget descriptions.

## Imports, package names and features

Only `rsille` version `3.0.0` is published. Its public modules are `rsille::tui`,
`rsille::render`, and `rsille::canvas`. No separate rsille component packages are
needed or published. For example:

```toml
[dependencies]
rsille = { version = "3.0.0", features = ["syntax-highlighting"] }
```

The repository's `tui`, `render` and `canvas` crates remain separate workspace
members for development and tests, each with `publish = false`. Shared implementation
lives in `src/tui`, `src/render`, and `src/canvas`; internal crate adapters include
the same module declarations with local crate aliases. The public package compiles
these sources directly as modules, without a build script, generated source, or
private path dependencies. The package contains all implementation files.

These internal crates are development targets; application code should use types
consistently from `rsille`, rather than mixing independently compiled internal
crate types with public `rsille` types. v2 code using `rsille::Canvas` should use
`rsille::canvas::Canvas`. Old v2 Turtle and 3D examples are not v3 APIs.

The prelude provides application construction, common widgets and runtime handles.
Import clock/executor integration and test tools explicitly:

```rust
use rsille::tui::runtime::{Clock, Executor, ManualClock, ManualExecutor, SystemClock, ThreadExecutor};
```

Default features are empty. `syntax-highlighting` on `rsille` enables optional
Syntect/once_cell dependencies directly.
CodeViewer remains available without it, rendering plain text. Rendering, editing,
layout, navigation and all other widgets remain available by default.

## Component and text migration

The [example guide](packages/tui/examples/README.md) specifies the earlier v3
changes: generic lifecycle synchronization; keyed identities with type replacement;
explicit hidden-state retention; `map` and nested `scope`; task/subscription
cancellation; full grapheme cells and byte/column conversion; shared text editing;
independent layout and paint invalidation. Custom widgets should implement
`lifecycle`, `layout`, and context invalidation rather than relying on concrete
runtime control hooks or the removed `modified_this_batch` input flag.

## Release validation

Run from the repository root with Python 3.12+ and Cargo:

```bash
cargo fmt --all --check
cargo test --workspace
cargo test --workspace --all-features
cargo check --workspace --all-targets --all-features
cargo run -p tui --example workflow -- --headless
cargo check --manifest-path examples/rsille-showcase/Cargo.toml
python3 scripts/check-release.py --offline
# Linux terminal protocol and restoration smoke check:
python3 scripts/check-terminal.py
```

The script checks that `rsille` is the only publishable workspace member, builds
one `.crate` archive in `target/package` using Cargo's verification, and checks its
normalized manifest has no private workspace or path dependencies. The archive
contains all implementation sources, README, license and migration guide.

It then runs tests against the extracted package with default and all features.
An independent consumer depends only on the extracted `rsille` and third-party
libraries and runs the Unicode edit/save/exit workflow plus Canvas/TUI/render type
interoperability checks. No patches point back to repository crates. Remove
`--offline` when third-party dependencies need downloading.

Publishing is a separate operation: publish only `rsille` from the repository root.
Private workspace crate manifests are excluded from the archive. No publish or
other external write is performed by the validation commands.
