//! Application runtime

pub mod message;

pub use message::{MessageChannel, MessageSender};

use crate::error::Result;
use crate::event::{Event, FocusManager, KeyCode, Modifiers};
use crate::layout::Container;
use crate::widget::Widget;
use crate::WidgetError;

// Re-export for AppWrapper
pub use render::{area::Size, chunk::Chunk, Draw, DrawErr, DrawUpdate, Update};

/// Application runtime for managing TUI lifecycle
#[derive(Debug)]
pub struct App<State> {
    state: State,
    focus_manager: Option<FocusManager>,
}

/// Wrapper to adapt App to DrawUpdate trait for use with event_loop
pub struct AppWrapper<State, F, V, M> {
    app: App<State>,
    update_fn: F,
    view_fn: V,
    messages: Vec<M>,
    needs_redraw: bool,
    // Performance optimization: cache widget tree to avoid rebuilding every frame
    cached_container: Option<Box<Container<M>>>,
    state_changed: bool,
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
        let app_wrapper = AppWrapper::new(self, update, view);

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
    fn apply_focus<M: Clone>(&mut self, container: &mut Container<M>) {
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
    fn handle_tab<M: Clone>(&mut self, container: &mut Container<M>, shift: bool) {
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
                let shift = key_event.modifiers.contains_shift();
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

impl<State, F, V, M> AppWrapper<State, F, V, M>
where
    F: Fn(&mut State, M),
    V: Fn(&State) -> Container<M>,
    M: Clone + std::fmt::Debug,
{
    pub fn new(app: App<State>, update_fn: F, view_fn: V) -> Self {
        Self {
            app,
            update_fn,
            view_fn,
            messages: Vec::new(),
            needs_redraw: true,
            cached_container: None,
            state_changed: true, // Start with state_changed=true to build initial tree
        }
    }
}

impl<State, F, V, M> Draw for AppWrapper<State, F, V, M>
where
    F: Fn(&mut State, M),
    V: Fn(&State) -> Container<M>,
    M: Clone + std::fmt::Debug,
{
    fn draw(&mut self, mut chunk: Chunk) -> std::result::Result<Size, DrawErr> {
        // Rebuild widget tree only if state changed or no cache exists
        if self.state_changed || self.cached_container.is_none() {
            let mut container = (self.view_fn)(&self.app.state);

            // Initialize focus management
            self.app.apply_focus(&mut container);

            // Cache the container
            self.cached_container = Some(Box::new(container));

            // Reset state_changed flag after rebuilding
            self.state_changed = false;
        }

        // Get reference to cached container for rendering
        let container = self.cached_container.as_mut()
            .expect("Container should be cached after rebuild");

        // Get the chunk area
        let area = chunk.area();
        let size = area.size();

        // Create a Rect from the area for rendering (starting at 0,0 since chunk handles positioning)
        use crate::widget::common::Rect;
        let rect = Rect::new(0, 0, size.width, size.height);

        // Render the widget tree directly to chunk
        container.render(&mut chunk, rect);

        Ok(size)
    }
}

impl<State, F, V, M> Update for AppWrapper<State, F, V, M>
where
    F: Fn(&mut State, M),
    V: Fn(&State) -> Container<M>,
    M: Clone + std::fmt::Debug,
{
    fn on_events(&mut self, events: &[crossterm::event::Event]) -> std::result::Result<(), DrawErr> {
        // Ensure we have a container (should be cached from draw, but check anyway)
        if self.cached_container.is_none() {
            let mut container = (self.view_fn)(&self.app.state);
            self.app.apply_focus(&mut container);
            self.cached_container = Some(Box::new(container));
        }

        for event in events {
            // Convert render event to our Event type
            let our_event = render_event_to_our(event);

            // Handle Tab navigation
            if let Event::Key(ref key_ev) = our_event {
                if key_ev.code == KeyCode::Tab {
                    // Tab navigation modifies focus, need to rebuild on next draw
                    let container = self.cached_container.as_mut()
                        .expect("Container should be cached");
                    let shift = key_ev.modifiers.contains_shift();
                    self.app.handle_tab(container, shift);
                    self.needs_redraw = true;
                    // Note: We don't set state_changed because focus is not part of state
                    // But we'll need to reapply focus on next rebuild
                    continue;
                }
            }

            // Route event to widgets and collect messages using cached container
            let container = self.cached_container.as_mut()
                .expect("Container should be cached");
            let (_result, messages) = container.handle_event_with_messages(&our_event);

            // Store messages for processing in update()
            self.messages.extend(messages);
            self.needs_redraw = true;
        }

        Ok(())
    }

    fn update(&mut self) -> std::result::Result<bool, DrawErr> {
        // Process all collected messages
        let had_messages = !self.messages.is_empty();
        for msg in self.messages.drain(..) {
            (self.update_fn)(&mut self.app.state, msg);
        }

        // If we processed any messages, state has changed
        if had_messages {
            self.state_changed = true;
        }

        // Return whether we need to redraw
        let needs_redraw = self.needs_redraw;
        self.needs_redraw = false;
        Ok(needs_redraw)
    }
}

/// Convert crossterm event to our Event type
fn render_event_to_our(event: &crossterm::event::Event) -> Event {
    match event {
        crossterm::event::Event::Key(key_event) => Event::Key(crate::event::KeyEvent {
            code: render_keycode_to_our(key_event.code),
            modifiers: render_modifiers_to_our(key_event.modifiers),
        }),
        crossterm::event::Event::Mouse(mouse_event) => {
            use crossterm::event::{MouseButton as RMB, MouseEventKind as RMK};

            let kind = match mouse_event.kind {
                RMK::Down(RMB::Left) => {
                    crate::event::MouseEventKind::Down(crate::event::MouseButton::Left)
                }
                RMK::Down(RMB::Right) => {
                    crate::event::MouseEventKind::Down(crate::event::MouseButton::Right)
                }
                RMK::Down(RMB::Middle) => {
                    crate::event::MouseEventKind::Down(crate::event::MouseButton::Middle)
                }
                RMK::Up(RMB::Left) => {
                    crate::event::MouseEventKind::Up(crate::event::MouseButton::Left)
                }
                RMK::Up(RMB::Right) => {
                    crate::event::MouseEventKind::Up(crate::event::MouseButton::Right)
                }
                RMK::Up(RMB::Middle) => {
                    crate::event::MouseEventKind::Up(crate::event::MouseButton::Middle)
                }
                RMK::Drag(RMB::Left) => {
                    crate::event::MouseEventKind::Drag(crate::event::MouseButton::Left)
                }
                RMK::Drag(RMB::Right) => {
                    crate::event::MouseEventKind::Drag(crate::event::MouseButton::Right)
                }
                RMK::Drag(RMB::Middle) => {
                    crate::event::MouseEventKind::Drag(crate::event::MouseButton::Middle)
                }
                RMK::Moved => crate::event::MouseEventKind::Moved,
                RMK::ScrollDown => crate::event::MouseEventKind::ScrollDown,
                RMK::ScrollUp => crate::event::MouseEventKind::ScrollUp,
                _ => return Event::Key(crate::event::KeyEvent::new(KeyCode::Null)), // Skip unknown events
            };

            Event::Mouse(crate::event::MouseEvent::with_modifiers(
                kind,
                mouse_event.column,
                mouse_event.row,
                render_modifiers_to_our(mouse_event.modifiers),
            ))
        }
        crossterm::event::Event::Resize(_, _) => {
            // Terminal was resized, re-render on next loop
            Event::Key(crate::event::KeyEvent::new(KeyCode::Null)) // Placeholder
        }
        _ => Event::Key(crate::event::KeyEvent::new(KeyCode::Null)), // Skip other events
    }
}

/// Convert crossterm KeyCode to our KeyCode
fn render_keycode_to_our(code: crossterm::event::KeyCode) -> KeyCode {
    use crossterm::event::KeyCode as RK;
    match code {
        RK::Backspace => KeyCode::Backspace,
        RK::Enter => KeyCode::Enter,
        RK::Left => KeyCode::Left,
        RK::Right => KeyCode::Right,
        RK::Up => KeyCode::Up,
        RK::Down => KeyCode::Down,
        RK::Home => KeyCode::Home,
        RK::End => KeyCode::End,
        RK::PageUp => KeyCode::PageUp,
        RK::PageDown => KeyCode::PageDown,
        RK::Tab => KeyCode::Tab,
        RK::BackTab => KeyCode::BackTab,
        RK::Delete => KeyCode::Delete,
        RK::Insert => KeyCode::Insert,
        RK::F(n) => KeyCode::F(n),
        RK::Char(c) => KeyCode::Char(c),
        RK::Esc => KeyCode::Esc,
        _ => KeyCode::Null,
    }
}

/// Convert crossterm KeyModifiers to our Modifiers
fn render_modifiers_to_our(modifiers: crossterm::event::KeyModifiers) -> Modifiers {
    use crossterm::event::KeyModifiers as RM;
    let mut mods = Modifiers::empty();

    if modifiers.contains(RM::SHIFT) {
        mods = mods.with_shift();
    }
    if modifiers.contains(RM::CONTROL) {
        mods = mods.with_control();
    }
    if modifiers.contains(RM::ALT) {
        mods = mods.with_alt();
    }

    mods
}
