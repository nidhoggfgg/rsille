# Internal render crate / 内部 render crate

English · [中文](README_cn.md)

Terminal buffers, styled Unicode text, bounded drawing and terminal event loops.
Coordinates and dimensions use terminal columns and rows.

This is a private workspace target (`publish = false`). In an application, depend on
`rsille` and import `rsille::render`. The adapter in [src/lib.rs](src/lib.rs) compiles
the shared implementation under [src/render](../../src/render); it does not duplicate it.

Run from the repository root:

```bash
cargo check -p render
cargo test -p render
```

[Project README](../../README.md) · [Migration guide](../../MIGRATION_V3.md)
