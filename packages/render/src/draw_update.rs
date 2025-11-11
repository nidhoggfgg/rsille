use crate::{area::Size, chunk::Chunk, DrawErr};

/// core trait for Render
/// in Render.render, it will call this trait's draw method to draw the content
pub trait Draw {
    fn draw(&mut self, chunk: Chunk) -> Result<Size, DrawErr>;
}

/// core trait for EventLoop
/// in every frame, call on_events -> update -> required_size -> Render.render(draw)
pub trait Update {
    fn on_events(&mut self, events: &[crossterm::event::Event]) -> Result<(), DrawErr>;
    fn update(&mut self) -> Result<bool, DrawErr>;

    /// Get required size for next render (optional)
    ///
    /// Returns None to keep current size, or Some(Size) to request a resize.
    /// This is called after update() and before render(), allowing components
    /// to adjust their size based on current state.
    ///
    /// Primary use case: inline mode dynamic height adjustment
    fn required_size(&self, _current_size: Size) -> Option<Size> {
        None // Default: no size change requested
    }
}

// this trait is for making trait object
pub trait DrawUpdate: Draw + Update {}

impl<T: Draw + Update> DrawUpdate for T {}
