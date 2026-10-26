# Internal canvas crate

This private workspace crate is a separate development and testing target.
It has `publish = false`; users install only `rsille` and use `rsille::canvas`.

Implementation is shared with the public package under `../../src/canvas`.
`src/lib.rs` provides the internal crate aliases and includes those module declarations;
there is no duplicated implementation or dependency on a separately published component.

```bash
cargo check -p canvas
cargo test -p canvas
```

See [the root README](../../README.md) and [migration guide](../../MIGRATION_V3.md).

`Canvas` stores sample coverage and color until output. It defaults to
`Encoding::Braille` (2×4 samples per cell); construct it with
`Canvas::with_encoding(Encoding::HalfBlock)` for 1×2 color samples per cell.
`set_color` paints samples, `set_background` fills uncovered regions, and the
same `Draw` implementation outputs either encoding into the shared renderer.
Canvas bounds use terminal cells; drawing coordinates use samples, with y upwards.
