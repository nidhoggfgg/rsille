//! Application runtime

pub mod message;

pub use message::{MessageChannel, MessageSender};

use crate::buffer::Buffer;
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

    /// Render a single widget (helper for testing)
    ///
    /// # Examples
    /// ```
    /// use tui::prelude::*;
    ///
    /// struct State;
    /// let app = App::new(State);
    /// let label = Label::new("Test");
    /// // app.render_widget(&label)?; // Would render in real scenario
    /// ```
    pub fn render_widget(&self, widget: &dyn Widget) -> Result<()> {
        let (width, height) = crossterm::terminal::size()?;
        let mut buffer = Buffer::new(width, height);
        let area = buffer.area();

        widget.render(&mut buffer, area);

        // Buffer flush to terminal will be implemented when integrating with render package
        buffer.flush()?;

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
    fn apply_focus<M>(&mut self, container: &mut Container<M>) {
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
    fn handle_tab<M>(&mut self, container: &mut Container<M>, shift: bool) {
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
        // Create view from current state
        let mut container = (self.view_fn)(&self.app.state);

        // Initialize focus management
        self.app.apply_focus(&mut container);

        // Create a buffer from the chunk area
        let area = chunk.area();
        let size = area.size();
        let mut buffer = Buffer::new(size.width, size.height);

        // Create a Rect from the area for rendering (starting at 0,0 since chunk handles positioning)
        use crate::widget::common::Rect;
        let rect = Rect::new(0, 0, size.width, size.height);

        // Render the widget tree
        container.render(&mut buffer, rect);

        // Copy buffer content to chunk
        for y in 0..size.height {
            for x in 0..size.width {
                if let Some(cell) = buffer.get(x, y) {
                    // Convert our Cell to render::style::Stylized
                    let stylized = convert_cell_to_stylized(cell);
                    let _ = chunk.set_forced(x, y, stylized);
                }
            }
        }

        Ok(size)
    }
}

impl<State, F, V, M> Update for AppWrapper<State, F, V, M>
where
    F: Fn(&mut State, M),
    V: Fn(&State) -> Container<M>,
    M: Clone + std::fmt::Debug,
{
    fn on_events(&mut self, events: &[term::event::Event]) -> std::result::Result<(), DrawErr> {
        for event in events {
            // Convert render event to our Event type
            let our_event = render_event_to_our(event);

            // Handle Tab navigation
            if let Event::Key(ref key_ev) = our_event {
                if key_ev.code == KeyCode::Tab {
                    let mut container = (self.view_fn)(&self.app.state);
                    let shift = key_ev.modifiers.contains_shift();
                    self.app.handle_tab(&mut container, shift);
                    self.needs_redraw = true;
                    continue;
                }
            }

            // Route event to widgets and collect messages
            let mut container = (self.view_fn)(&self.app.state);
            self.app.apply_focus(&mut container);
            let (_result, messages) = container.handle_event_with_messages(&our_event);

            // Store messages for processing in update()
            self.messages.extend(messages);
            self.needs_redraw = true;
        }

        Ok(())
    }

    fn update(&mut self) -> std::result::Result<bool, DrawErr> {
        // Process all collected messages
        for msg in self.messages.drain(..) {
            (self.update_fn)(&mut self.app.state, msg);
        }

        // Return whether we need to redraw
        let needs_redraw = self.needs_redraw;
        self.needs_redraw = false;
        Ok(needs_redraw)
    }
}

/// Convert render event to our Event type
fn render_event_to_our(event: &term::event::Event) -> Event {
    match event {
        term::event::Event::Key(key_event) => Event::Key(crate::event::KeyEvent {
            code: render_keycode_to_our(key_event.code),
            modifiers: render_modifiers_to_our(key_event.modifiers),
        }),
        term::event::Event::Mouse(mouse_event) => {
            use term::event::{MouseButton as RMB, MouseEventKind as RMK};

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
        term::event::Event::Terminal(term::event::TerminalEvent::Resize(_, _)) => {
            // Terminal was resized, re-render on next loop
            Event::Key(crate::event::KeyEvent::new(KeyCode::Null)) // Placeholder
        }
        _ => Event::Key(crate::event::KeyEvent::new(KeyCode::Null)), // Skip other events
    }
}

/// Convert render KeyCode to our KeyCode
fn render_keycode_to_our(code: term::event::KeyCode) -> KeyCode {
    use term::event::KeyCode as RK;
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

/// Convert render KeyModifiers to our Modifiers
fn render_modifiers_to_our(modifiers: term::event::KeyModifiers) -> Modifiers {
    use term::event::KeyModifiers as RM;
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

/// Convert our Cell to render::style::Stylized
fn convert_cell_to_stylized(cell: &crate::buffer::Cell) -> render::style::Stylized {
    use render::style::Style;
    use term::crossterm::style::Attributes;

    // Only create Colors if we have actual colors to set
    // This prevents SetColors from being called on empty cells,
    // which would apply the terminal's default background color
    let colors = if cell.fg.is_some() || cell.bg.is_some() {
        let fg = cell.fg.map(convert_color_to_crossterm);
        let bg = cell.bg.map(convert_color_to_crossterm);
        Some(term::crossterm::style::Colors {
            foreground: fg,
            background: bg,
        })
    } else {
        None
    };

    // Only create Attributes if we have modifiers
    let attr = if !cell.modifiers.is_empty() {
        let mut a = Attributes::default();
        if cell.modifiers.contains_bold() {
            a = a | term::crossterm::style::Attribute::Bold;
        }
        if cell.modifiers.contains_italic() {
            a = a | term::crossterm::style::Attribute::Italic;
        }
        if cell.modifiers.contains_underlined() {
            a = a | term::crossterm::style::Attribute::Underlined;
        }
        Some(a)
    } else {
        None
    };

    // Use the appropriate Style constructor based on what we have
    let style = match (colors, attr) {
        (Some(c), Some(a)) => Style::with_both(c, a),
        (Some(c), None) => Style::with_colors(c),
        (None, Some(a)) => Style::with_attr(a),
        (None, None) => Style::default(),
    };

    render::style::Stylized::new(cell.symbol, style)
}

/// Convert our Color to crossterm Color
fn convert_color_to_crossterm(color: crate::style::Color) -> crossterm::style::Color {
    use crossterm::style::Color as CC;
    match color {
        crate::style::Color::Black => CC::Black,
        crate::style::Color::Red => CC::DarkRed,
        crate::style::Color::Green => CC::DarkGreen,
        crate::style::Color::Yellow => CC::DarkYellow,
        crate::style::Color::Blue => CC::DarkBlue,
        crate::style::Color::Magenta => CC::DarkMagenta,
        crate::style::Color::Cyan => CC::DarkCyan,
        crate::style::Color::White => CC::White,
        crate::style::Color::Rgb(r, g, b) => CC::Rgb { r, g, b },
        crate::style::Color::Indexed(i) => CC::AnsiValue(i),
    }
}
