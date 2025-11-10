use crate::{layout::Container, widget::Widget, Result, WidgetError};

/// Application runtime for managing TUI lifecycle
#[derive(Debug)]
pub struct App<State> {
    pub(super) state: State,
}

impl<State> App<State> {
    pub fn new(state: State) -> Self {
        Self { state }
    }

    pub fn run<M, F, V>(self, update: F, view: V) -> Result<()>
    where
        F: Fn(&mut State, M) + Send + Sync + 'static,
        V: Fn(&State) -> Container<M> + Send + Sync + 'static,
        M: Clone + std::fmt::Debug + Send + Sync + 'static,
        State: Send + Sync + 'static,
    {
        self.run_with_options(update, view, false)
    }

    pub fn run_inline<M, F, V>(self, update: F, view: V) -> Result<()>
    where
        F: Fn(&mut State, M) + Send + Sync + 'static,
        V: Fn(&State) -> Container<M> + Send + Sync + 'static,
        M: Clone + std::fmt::Debug + Send + Sync + 'static,
        State: Send + Sync + 'static,
    {
        self.run_with_options(update, view, true)
    }

    /// Internal method to run the application with options
    fn run_with_options<M, F, V>(self, update: F, view: V, inline_mode: bool) -> Result<()>
    where
        F: Fn(&mut State, M) + Send + Sync + 'static,
        V: Fn(&State) -> Container<M> + Send + Sync + 'static,
        M: Clone + std::fmt::Debug + Send + Sync + 'static,
        State: Send + Sync + 'static,
    {
        // Get terminal size
        let (width, height) = crossterm::terminal::size()?;

        // Default max height for inline mode
        let inline_max_height = 50;

        // For inline mode, calculate initial height from actual content
        // This prevents the first frame from occupying excessive space
        let (buffer_height, initial_used_height) = if inline_mode {
            // Call view to get initial widget tree before moving self
            let container = view(&self.state);
            let required_height = container.constraints().min_height;
            // Apply the same formula as dynamic resizing
            let used_height = required_height.min(inline_max_height).min(height);
            // Buffer is allocated at max capacity to avoid reallocation
            // but we only use initial_used_height for rendering
            (inline_max_height.min(height), used_height)
        } else {
            (height, height)
        };

        // Create AppWrapper with inline configuration (self is moved here)
        let app_wrapper = super::wrapper::AppWrapper::new(self, update, view).with_inline_config(
            inline_mode,
            inline_max_height,
            width,
        );

        // Build event loop with appropriate settings
        let mut builder = render::Builder::new();
        builder
            .enable_raw_mode()
            .clear(false)
            .append_newline(false)
            .enable_hide_cursor();

        if inline_mode {
            builder
                .inline_mode(true)
                .inline_max_height(buffer_height)
                .size((width, buffer_height)); // Allocate buffer at max capacity
        } else {
            // Full-screen mode: use alternate screen
            builder
                .enable_all() // Enable raw mode, alt screen, mouse capture, hide cursor
                .size((width, height));
        }

        let mut event_loop = builder.build_event_loop(app_wrapper);

        // In inline mode, set initial used height to avoid rendering empty space
        if inline_mode {
            event_loop.set_initial_used_height(initial_used_height);
        }

        event_loop
            .run()
            .map_err(|e| WidgetError::RenderError(e.to_string()))?;

        Ok(())
    }

    /// Get a reference to the application state
    pub fn state(&self) -> &State {
        &self.state
    }

    /// Get a mutable reference to the application state
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }
}
