//! Internal workspace crate; the published API is `rsille::tui`.

extern crate render;
extern crate rsille_canvas as canvas;
extern crate self as tui;

include!("../../../src/tui/api.rs");
