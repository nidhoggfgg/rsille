//! Contracts for drawing application state and processing input.

use crate::render::{area::Size, chunk::Chunk, DrawErr};

/// Paints application state into a terminal-cell buffer.
///
/// Each frame starts with a blank buffer, so implementations should paint all
/// visible content. The renderer handles comparison with the previous frame.
///
/// ```
/// use rsille::render::{area::Size, chunk::Chunk, style::Style, Builder, Draw, DrawErr};
///
/// struct Greeting;
///
/// impl Draw for Greeting {
///     fn draw(&mut self, mut chunk: Chunk<'_>) -> Result<Size, DrawErr> {
///         chunk.set_string(0, 0, "Hello", Style::default())?;
///         Ok((5, 1).into())
///     }
/// }
///
/// let mut output = Vec::new();
/// Builder::new()
///     .size((5, 1))
///     .clear(false)
///     .build_render(Greeting, &mut output)
///     .render()?;
/// assert!(output.ends_with(b"Hello"));
/// # Ok::<(), std::io::Error>(())
/// ```
pub trait Draw {
    /// Paints into `chunk` using coordinates relative to its top-left corner.
    ///
    /// Returns the content size in terminal cells. [`crate::render::Render`]
    /// currently ignores this size; use [`Update::required_size`] to request
    /// event-loop resizing. Drawing errors propagate to the caller.
    fn draw(&mut self, chunk: Chunk) -> Result<Size, DrawErr>;
}

/// Processes input and advances an application's state in the event loop.
///
/// Each iteration dispatches input, calls [`update`](Self::update), checks
/// [`should_quit`](Self::should_quit), and applies a size request before
/// drawing if needed. Update and draw failures are logged by the event loop.
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

    /// Processes an ordered batch of terminal events before the next update.
    ///
    /// The batch may be empty. Fullscreen mouse positions are terminal
    /// coordinates; the default inline dispatcher translates them to viewport
    /// coordinates. Errors are logged by the event loop, which keeps running.
    fn on_events(&mut self, events: &[crossterm::event::Event]) -> Result<(), DrawErr>;

    /// Dispatch raw terminal events into an Inline viewport. The default drops
    /// outside pointer input; capture-aware applications may retain signed
    /// outside positions without splitting the input batch.
    ///
    /// `origin` is the viewport's absolute `(column, row)` and `viewport` is
    /// its size in cells. Returns whether dispatch should trigger a redraw;
    /// the default returns `true` if any events remain after filtering.
    /// Errors from [`on_events`](Self::on_events) are propagated.
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

    /// Opt into update checkpoints between input events, allowing an event to
    /// request a commit before the rest of the batch is dispatched. Ordinary
    /// draws are still coalesced. The default preserves batched event delivery.
    fn event_frame_boundaries(&self) -> bool {
        false
    }

    /// Whether updates are paused for a hard inline presentation boundary.
    /// The event loop renders and commits before acknowledging this request.
    fn frame_commit_pending(&self) -> bool {
        false
    }

    /// Acknowledges successful presentation and cursor advancement. Never called
    /// on a failed render or commit; implementations may now queue a continuation.
    fn frame_committed(&mut self) {}

    /// Check if the application should quit
    ///
    /// This allows the application to signal a clean shutdown without
    /// calling std::process::exit(), which would bypass terminal cleanup.
    ///
    /// The event loop checks this after updating and exits before drawing
    /// another frame, allowing terminal cleanup via `Drop`.
    fn should_quit(&self) -> bool {
        false
    }

    /// Get required size for next render (optional)
    ///
    /// Returns `None` to keep the current size, or `Some(size)` to request a
    /// change after updating and before drawing. Fullscreen mode resizes the
    /// buffer; inline mode uses only the requested height, clamped to the
    /// existing buffer capacity.
    fn required_size(&self, _current_size: Size) -> Option<Size> {
        None // Default: no size change requested
    }
}

/// Combines drawing and updating for an event-loop application or trait object.
///
/// Implemented automatically for every type implementing [`Draw`] and [`Update`].
pub trait DrawUpdate: Draw + Update {}

impl<T: Draw + Update> DrawUpdate for T {}
