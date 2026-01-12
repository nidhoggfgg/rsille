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

    /// Check if the application should quit
    ///
    /// This allows the application to signal a clean shutdown without
    /// calling std::process::exit(), which would bypass terminal cleanup.
    ///
    /// The event loop will gracefully stop after the current frame when
    /// this returns true, allowing proper cleanup via Drop.
    fn should_quit(&self) -> bool {
        false
    }

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
