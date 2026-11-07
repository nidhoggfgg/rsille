//! Mutable cell state passed through a visual effect pipeline.

use crate::render::style::Stylized;

/// A single sampled terminal cell in a visual effect pipeline.
///
/// Coordinates are local to [`VisualCtx::area`](super::VisualCtx::area).
/// `source_*` identifies the child-rendered cell being sampled; `dest_*` is the
/// current destination after geometry effects have mapped it.
#[derive(Debug, Clone, PartialEq)]
pub struct CellSample {
    /// Original horizontal cell position; geometry effects should leave it unchanged.
    pub source_x: f64,
    /// Original vertical cell position; geometry effects should leave it unchanged.
    pub source_y: f64,
    /// Transformed horizontal position, rounded to the nearest cell when blitted.
    pub dest_x: f64,
    /// Transformed vertical position, rounded to the nearest cell when blitted.
    pub dest_y: f64,
    /// Glyph or grapheme and terminal style to write at the destination.
    pub content: Stylized,
    /// Whether the sample should be written; hidden or out-of-area samples are skipped.
    pub visible: bool,
}

impl CellSample {
    /// Create a visible sample whose destination initially matches its source position.
    pub fn new(source_x: u16, source_y: u16, content: Stylized) -> Self {
        let source_x = source_x as f64;
        let source_y = source_y as f64;
        Self {
            source_x,
            source_y,
            dest_x: source_x,
            dest_y: source_y,
            content,
            visible: true,
        }
    }
}
