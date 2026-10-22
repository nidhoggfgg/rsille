use render::style::Stylized;

/// A single sampled terminal cell in a visual effect pipeline.
///
/// Coordinates are local to [`VisualCtx::area`]. `source_*` identifies the
/// child-rendered cell being sampled; `dest_*` is the current destination after
/// geometry effects have mapped it.
#[derive(Debug, Clone, PartialEq)]
pub struct CellSample {
    pub source_x: f64,
    pub source_y: f64,
    pub dest_x: f64,
    pub dest_y: f64,
    pub content: Stylized,
    pub visible: bool,
}

impl CellSample {
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
