# Internal tui crate

This private workspace crate is a separate development and testing target.
It has `publish = false`; users install only `rsille` and use `rsille::tui`.

Implementation is shared with the public package under `../../src/tui`.
`src/lib.rs` provides the internal crate aliases and includes those module declarations;
there is no duplicated implementation or dependency on a separately published component.

```bash
cargo check -p tui
cargo test -p tui
```

See [the root README](../../README.md) and [migration guide](../../MIGRATION_V3.md).
