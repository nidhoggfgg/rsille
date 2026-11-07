# Internal canvas crate / 内部 canvas crate

English · [中文](README_cn.md)

Colored sample graphics with Braille (2 × 4), Half-block (1 × 2) and Quadrant
(2 × 2) encoding. Drawing coordinates use samples with y increasing upward; bounds
and output sizes use terminal cells.

This is a private workspace target (`publish = false`). In an application, depend on
`rsille` and import `rsille::canvas`. The adapter in [src/lib.rs](src/lib.rs) compiles
the shared implementation under [src/canvas](../../src/canvas); it does not duplicate it.

Run from the repository root:

```bash
cargo check -p canvas
cargo test -p canvas
```

[Project README](../../README.md) · [Migration guide](../../MIGRATION_V3.md)
