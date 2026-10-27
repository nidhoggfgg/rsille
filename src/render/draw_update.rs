use crate::render::{area::Size, chunk::Chunk, DrawErr};

/// core trait for Render
/// in Render.render, it will call this trait's draw method to draw the content
pub trait Draw {
    fn draw(&mut self, chunk: Chunk) -> Result<Size, DrawErr>;
}

/// core trait for EventLoop
/// in every frame, call on_events -> update -> required_size -> Render.render(draw)
pub trait Update {
    /// Called after terminal setup, before input. Headless callers may inject
    /// capabilities explicitly; no terminal probing happens during headless steps.
    fn on_input_capabilities(&mut self, _capabilities: crate::render::InputCapabilities) {}

    /// Opt into event-driven scheduling. Producers must wake after enqueueing work.
    /// Implementations returning `None` retain the builder's frame polling behavior.
    fn wake_handle(&self) -> Option<crate::render::WakeHandle> {
        None
    }

    /// Next scheduled update for an event-driven application, in real clock time.
    /// `None` means wait for input or a wake signal without a periodic poll.
    fn next_update_at(&self) -> Option<std::time::Instant> {
        None
    }

    fn on_events(&mut self, events: &[crossterm::event::Event]) -> Result<(), DrawErr>;

    /// Dispatch raw terminal events into an Inline viewport. The default drops
    /// outside pointer input; capture-aware applications may retain signed
    /// outside positions without splitting the input batch.
    fn on_events_in_viewport(
        &mut self,
        events: &[crossterm::event::Event],
        origin: (u16, u16),
        viewport: Size,
    ) -> Result<bool, DrawErr> {
        use crossterm::event::Event;
        let mut outside = false;
        let events = events
            .iter()
            .filter_map(|event| match event {
                Event::Mouse(mouse) => {
                    let x = i32::from(mouse.column) - i32::from(origin.0);
                    let y = i32::from(mouse.row) - i32::from(origin.1);
                    outside = x < 0
                        || y < 0
                        || x >= i32::from(viewport.width)
                        || y >= i32::from(viewport.height);
                    if outside {
                        None
                    } else {
                        let mut mouse = *mouse;
                        mouse.column = x as u16;
                        mouse.row = y as u16;
                        Some(Event::Mouse(mouse))
                    }
                }
                event => Some(event.clone()),
            })
            .collect::<Vec<_>>();
        self.on_events(&events)?;
        if outside {
            self.on_pointer_leave();
        }
        Ok(!events.is_empty())
    }

    /// Called after an input batch when its last mouse event was outside the
    /// inline rendering area. Clear pointer hover without changing keyboard focus.
    /// Outside mouse events are not forwarded to `on_events`.
    fn on_pointer_leave(&mut self) {}

    /// Advance application state.
    ///
    /// Return `true` when the new state should be rendered, `false` when no redraw is needed.
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
