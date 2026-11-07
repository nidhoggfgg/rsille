//! Internal workspace crate; the published API is `rsille::tui`.
//!
//! Build applications with [`app::App`], compose [`widgets`] with [`layout`], and
//! schedule background work through [`effect::UpdateCtx`]. [`runtime`] supports
//! both terminal execution and deterministic headless stepping. [`widget`] is
//! the extension point for custom widgets; [`prelude`] collects common imports.
//!
//! Sources are shared with the published module; examples in the API documentation
//! use the `rsille` import path.

extern crate render;
extern crate rsille_canvas as canvas;
extern crate self as tui;

include!("../../../src/tui/api.rs");
