use crate::event::KeyCode;
use crate::style::{Theme, ThemeManager};
use crate::{layout::Layout, WidgetResult, WidgetError};
use std::collections::HashMap;
use std::sync::Arc;

pub type EventHandler<M> = Arc<dyn Fn() -> M + Send + Sync>;

/// Application runtime for managing TUI lifecycle
pub struct App<State, M = ()> {
    pub(super) state: State,
    pub(super) global_key_handlers: HashMap<KeyCode, EventHandler<M>>,
}

// Manual Debug implementation because EventHandler contains Fn
impl<State: std::fmt::Debug, M> std::fmt::Debug for App<State, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App")
            .field("state", &self.state)
            .field("global_key_handlers", &self.global_key_handlers.keys())
            .finish()
    }
}

impl<State, M: Clone + std::fmt::Debug + Send + Sync + 'static> App<State, M> {
    /// Create a new application with the given state
    ///
    /// The message type M is automatically inferred from subsequent method calls
    /// like .on_key() or .run().
    ///
    /// # Example
    /// ```no_run
    /// use tui::prelude::*;
    /// use tui::event::KeyCode;
    ///
    /// App::new(state)
    ///     .on_key(KeyCode::Char('q'), || Message::Quit)
    ///     .on_key(KeyCode::Esc, || Message::Cancel)
    ///     .run(update, view);
    /// ```
    pub fn new(state: State) -> Self {
        Self {
            state,
            global_key_handlers: HashMap::new(),
        }
    }

    /// Register global keyboard shortcuts
    ///
    /// This method can be called multiple times to register different key handlers.
    /// The message type M is automatically inferred from the handler's return type.
    ///
    /// # Example
    /// ```no_run
    /// use tui::prelude::*;
    /// use tui::event::KeyCode;
    ///
    /// App::new(state)
    ///     .on_key(KeyCode::Char('q'), || Message::Quit)
    ///     .on_key(KeyCode::Char('h'), || Message::ShowHelp)
    ///     .on_key(KeyCode::Esc, || Message::Cancel)
    ///     .run(update, view);
    /// ```
    pub fn on_key<F>(mut self, key: KeyCode, handler: F) -> Self
    where
        F: Fn() -> M + Send + Sync + 'static,
    {
        self.global_key_handlers.insert(key, Arc::new(handler));
        self
    }

    /// Set the initial theme for the application
    ///
    /// # Example
    /// ```no_run
    /// use tui::prelude::*;
    ///
    /// let app = App::new(state)
    ///     .with_theme(Theme::dark());
    /// ```
    pub fn with_theme(self, theme: Theme) -> Self {
        ThemeManager::global().set_theme(theme);
        self
    }

    pub fn run<F, V, L>(self, update: F, view: V) -> WidgetResult<()>
    where
        F: Fn(&mut State, M) + Send + Sync + 'static,
        V: Fn(&State) -> L + Send + Sync + 'static,
        L: Layout<M> + 'static,
        M: Clone + std::fmt::Debug + Send + Sync + 'static,
        State: Send + Sync + 'static,
    {
        // Wrap the view function to box the layout trait object
        let view_wrapper = move |state: &State| -> Box<dyn Layout<M>> { Box::new(view(state)) };
        self.run_with_options(update, view_wrapper, false)
    }

    pub fn run_inline<F, V, L>(self, update: F, view: V) -> WidgetResult<()>
    where
        F: Fn(&mut State, M) + Send + Sync + 'static,
        V: Fn(&State) -> L + Send + Sync + 'static,
        L: Layout<M> + 'static,
        M: Clone + std::fmt::Debug + Send + Sync + 'static,
        State: Send + Sync + 'static,
    {
        // Wrap the view function to box the layout trait object
        let view_wrapper = move |state: &State| -> Box<dyn Layout<M>> { Box::new(view(state)) };
        self.run_with_options(update, view_wrapper, true)
    }

    /// Internal method to run the application with options
    fn run_with_options<F, V>(self, update: F, view: V, inline_mode: bool) -> WidgetResult<()>
    where
        F: Fn(&mut State, M) + Send + Sync + 'static,
        V: Fn(&State) -> Box<dyn Layout<M>> + Send + Sync + 'static,
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
            let layout = view(&self.state);
            let required_height = layout.constraints().min_height;
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
                .frame_limit(60)
                // Note: Mouse events are NOT enabled in inline mode because:
                // 1. User can scroll the terminal, invalidating our coordinate tracking
                // 2. The rendered content position changes with scrolling
                // 3. inline mode is for simple CLI output, not interactive UIs
                // For mouse interaction, use fullscreen mode with run() instead of run_inline()
                .size((width, buffer_height)); // Allocate buffer at max capacity
        } else {
            // Full-screen mode: use alternate screen
            builder
                .enable_all() // Enable raw mode, alt screen, mouse capture, hide cursor
                .frame_limit(60)
                .size((width, height));
        }

        let mut event_loop = builder.build_event_loop(app_wrapper);

        // In inline mode, set initial used height to avoid rendering empty space
        if inline_mode {
            event_loop.set_initial_used_height(initial_used_height);
        }

        event_loop
            .run()
            .map_err(|e| WidgetError::render_error(e.to_string()))?;

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
