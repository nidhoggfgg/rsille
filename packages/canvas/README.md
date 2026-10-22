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
