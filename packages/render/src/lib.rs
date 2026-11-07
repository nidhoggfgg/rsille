//! Internal workspace crate; the published API is `rsille::render`.
//!
//! [`buffer::Buffer`] stores terminal cells, and [`chunk::Chunk`] exposes bounded
//! drawing regions. [`Builder`] configures standalone [`Render`] instances and
//! [`event_loop::EventLoop`] execution. See [`text`] for Unicode width policies
//! and [`style`] for colors, attributes, and styled content.
//!
//! Sources are shared with the published module; examples in the API documentation
//! use the `rsille` import path.

extern crate self as render;

include!("../../../src/render/api.rs");
