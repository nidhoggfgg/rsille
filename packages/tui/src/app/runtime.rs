use crate::{layout::Container, Result, WidgetError};

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

        // Create AppWrapper
        let app_wrapper = super::wrapper::AppWrapper::new(self, update, view);

        // Build event loop with appropriate settings
        let mut builder = render::Builder::new();
        builder
            .enable_raw_mode()
            .clear(false)
            .append_newline(false)
            .enable_hide_cursor();

        if inline_mode {
            let inline_height = 20;
            builder
                .inline_mode(true)
                .size((width, inline_height.min(height)));
        } else {
            // Full-screen mode: use alternate screen
            builder
                .enable_all() // Enable raw mode, alt screen, mouse capture, hide cursor
                .size((width, height));
        }

        let event_loop = builder.build_event_loop(app_wrapper);

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
