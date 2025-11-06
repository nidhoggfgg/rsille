use crate::{
    event::{Event, FocusManager, KeyCode, KeyModifiers},
    layout::Container,
    Result, WidgetError,
};

/// Application runtime for managing TUI lifecycle
#[derive(Debug)]
pub struct App<State> {
    pub(super) state: State,
    pub(super) focus_manager: Option<FocusManager>,
}

impl<State> App<State> {
    /// Create a new application with initial state
    ///
    /// # Examples
    /// ```
    /// use tui::prelude::*;
    ///
    /// struct Counter { value: u32 }
    /// let app = App::new(Counter { value: 0 });
    /// ```
    pub fn new(state: State) -> Self {
        Self {
            state,
            focus_manager: None,
        }
    }

    /// Run the application with update and view functions
    ///
    /// Basic event loop demonstrating the Elm Architecture pattern.
    /// Uses the render package's event_loop for better performance and architecture.
    ///
    /// # Examples
    /// ```no_run
    /// use tui::prelude::*;
    ///
    /// struct Counter { value: i32 }
    ///
    /// #[derive(Clone, Debug)]
    /// enum Message { Increment }
    ///
    /// fn update(state: &mut Counter, msg: Message) {
    ///     match msg {
    ///         Message::Increment => state.value += 1,
    ///     }
    /// }
    ///
    /// fn view(state: &Counter) -> Container<Message> {
    ///     Container::vertical(vec![
    ///         Label::new(format!("Count: {}", state.value)).into(),
    ///     ])
    /// }
    ///
    /// let app = App::new(Counter { value: 0 });
    /// app.run(update, view)?;
    /// # Ok::<(), tui::error::WidgetError>(())
    /// ```
    pub fn run<M, F, V>(self, update: F, view: V) -> Result<()>
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

        // Build and run event loop
        let event_loop = render::Builder::new()
            .enable_all() // Enable raw mode, alt screen, mouse capture, hide cursor
            .clear(false) // Don't clear on every frame (event_loop clears once on start)
            .append_newline(false) // Don't append a newline
            .size((width, height))
            .build_event_loop(app_wrapper);

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

    /// Apply focus state from FocusManager to container widgets
    pub(crate) fn apply_focus<M: Clone>(&mut self, container: &mut Container<M>) {
        let focus_manager = self
            .focus_manager
            .get_or_insert_with(|| FocusManager::new(container.children()));

        // Clear focus on all widgets first
        for (idx, child) in container.children_mut().iter_mut().enumerate() {
            let is_focused = focus_manager.is_focused(idx);
            child.set_focused(is_focused);
        }
    }

    /// Handle Tab key for focus navigation
    pub(crate) fn handle_tab<M: Clone>(&mut self, container: &mut Container<M>, shift: bool) {
        if let Some(ref mut focus_manager) = self.focus_manager {
            if shift {
                focus_manager.prev();
            } else {
                focus_manager.next();
            }
            self.apply_focus(container);
        }
    }

    /// Route event to the container, handling Tab navigation
    #[allow(dead_code)] // Will be used in async event system
    pub(crate) fn route_event<M>(&mut self, container: &mut Container<M>, event: &Event)
    where
        M: Clone,
    {
        // Handle Tab navigation
        if let Event::Key(key_event) = event {
            if key_event.code == KeyCode::Tab {
                let shift = key_event.modifiers.contains(KeyModifiers::SHIFT);
                self.handle_tab(container, shift);
                return;
            }
        }

        // Route event to container
        let _ = container.handle_event_with_messages(event);
    }

    /// Get the message sender for this app (for use in view functions)
    pub fn sender(&self) -> std::sync::mpsc::Sender<()> {
        // This is a placeholder - proper implementation will come later
        std::sync::mpsc::channel().0
    }
}
