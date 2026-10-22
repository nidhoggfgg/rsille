//! Internal workspace crate; the published API is `rsille::canvas`.

extern crate render;
extern crate self as canvas;

include!("../../../src/canvas/api.rs");
