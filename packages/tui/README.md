# Internal tui crate / 内部 tui crate

English · [中文](README_cn.md)

Applications, composable widgets, layout, focus, pointer gestures, animation and
background work. The same runtime supports terminal interaction and headless stepping.

This is a private workspace target (`publish = false`). In an application, depend on
`rsille` and import `rsille::tui`. The adapter in [src/lib.rs](src/lib.rs) compiles
the shared implementation under [src/tui](../../src/tui); it does not duplicate it.

Run from the repository root:

```bash
cargo check -p tui
cargo test -p tui
```

See the [small example guide](examples/README.md) for runnable widget and runtime examples.

[Project README](../../README.md) · [Migration guide](../../MIGRATION_V3.md)
