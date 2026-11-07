//! Internal workspace crate; the published API is `rsille::canvas`.
//!
//! [`Canvas`] stores samples in Cartesian coordinates and renders terminal cells
//! using [`Encoding`]. [`Paint`] allows reusable drawing routines; [`braille`]
//! and [`utils`] expose lower-level conversion and geometry helpers.
//!
//! Sources are shared with the published module; examples in the API documentation
//! use the `rsille` import path.

extern crate render;
extern crate self as canvas;

include!("../../../src/canvas/api.rs");
