//! Interactive widget wrapper for mouse event handling
//!
//! This module provides the `Interactive<M, W>` wrapper type that adds mouse event
//! handling to any widget while remaining completely transparent for rendering.
//!
//! # Examples
//!
//! ```
//! use tui::prelude::*;
//!
//! #[derive(Clone)]
//! enum Message {
//!     Clicked,
//!     Hovered,
//! }
//!
//! let clickable_label = interactive(label("Click me"))
//!     .on_click(|| Message::Clicked)
//!     .on_hover(|| Message::Hovered);
//! ```

use crate::{
    event::{Event, EventResult, MouseButton, MouseEvent, MouseEventKind},
    widget::Widget,
};
use render::area::Area;
use std::sync::{Arc, RwLock};

/// Internal state for tracking mouse interactions
#[derive(Debug, Clone, Copy, Default)]
struct InteractiveState {
    /// Whether mouse is currently hovering over this widget
    hovering: bool,

    /// Whether mouse button is currently pressed down on this widget
    pressed: bool,

    /// Position where mouse was pressed (for click detection)
    press_pos: Option<(u16, u16)>,
}

/// Interactive wrapper that adds mouse event handling to any widget
///
/// Makes any widget respond to mouse events (click, hover, press, release, drag)
/// while remaining completely transparent for rendering.
///
/// The wrapper caches the widget's rendered area during `render()` and uses it
/// for precise hit testing during `handle_event()`.
///
/// # Event Types
///
/// - **Click**: Mouse button pressed and released at the same location
/// - **Hover**: Mouse enters the widget's area
/// - **Press**: Mouse button pressed down
/// - **Release**: Mouse button released
/// - **Drag**: Mouse moved while button is pressed
///
/// # Examples
///
/// ```
/// use tui::prelude::*;
///
/// #[derive(Clone)]
/// enum Message {
///     LabelClicked,
///     BoxClicked,
/// }
///
/// // Make a label clickable
/// let clickable_label = interactive(label("Click me"))
///     .on_click(|| Message::LabelClicked);
///
/// // Make an entire container clickable
/// let clickable_box = interactive(
///     col()
///         .border(BorderStyle::Single)
///         .child(label("Content"))
/// )
/// .on_click(|| Message::BoxClicked);
/// ```
///
/// # Performance
///
/// The wrapper adds minimal overhead:
/// - Memory: ~250 bytes per widget (handlers + state + cached area)
/// - Render: Single write to cache area, then delegate to inner widget
/// - Events: One cache read + simple hit test arithmetic
pub struct Interactive<M, W> {
    /// The wrapped widget
    inner: W,

    /// Cached area from last render (for hit testing)
    /// Uses RwLock for interior mutability in render()
    cached_area: RwLock<Option<Area>>,

    /// Click handler (mouse button down + up in same location)
    on_click: Option<Arc<dyn Fn() -> M + Send + Sync>>,

    /// Hover handler (mouse moved over widget)
    on_hover: Option<Arc<dyn Fn() -> M + Send + Sync>>,

    /// Press handler (mouse button down)
    on_press: Option<Arc<dyn Fn() -> M + Send + Sync>>,

    /// Release handler (mouse button up)
    on_release: Option<Arc<dyn Fn() -> M + Send + Sync>>,

    /// Drag handler (mouse moved while button down)
    on_drag: Option<Arc<dyn Fn() -> M + Send + Sync>>,

    /// Internal state for tracking interactions
    /// Tracks if mouse is currently hovering and if button is pressed
    state: RwLock<InteractiveState>,
}

impl<M, W> Interactive<M, W> {
    /// Create a new Interactive wrapper around a widget
    ///
    /// # Examples
    ///
    /// ```
    /// use tui::widget::{Interactive, Label};
    ///
    /// let label = Label::new("Text");
    /// let interactive_label = Interactive::<(), _>::new(label);
    /// ```
    ///
    /// Prefer using the `interactive()` helper function for more ergonomic usage.
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            cached_area: RwLock::new(None),
            on_click: None,
            on_hover: None,
            on_press: None,
            on_release: None,
            on_drag: None,
            state: RwLock::new(InteractiveState::default()),
        }
    }

    /// Set click handler (fires on mouse down + up in same location)
    ///
    /// The handler is called when the user presses and releases the left mouse
    /// button within the widget's area. A small tolerance (1 cell) is allowed
    /// for movement between press and release.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui::prelude::*;
    ///
    /// #[derive(Clone)]
    /// enum Message { Clicked }
    ///
    /// let button = interactive(label("Click me"))
    ///     .on_click(|| Message::Clicked);
    /// ```
    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn() -> M + Send + Sync + 'static,
    {
        self.on_click = Some(Arc::new(handler));
        self
    }

    /// Set hover handler (fires when mouse enters widget area)
    ///
    /// The handler is called once when the mouse enters the widget's area.
    /// It won't be called again until the mouse leaves and re-enters.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui::prelude::*;
    ///
    /// #[derive(Clone)]
    /// enum Message { Hovered }
    ///
    /// let label = interactive(label("Hover me"))
    ///     .on_hover(|| Message::Hovered);
    /// ```
    pub fn on_hover<F>(mut self, handler: F) -> Self
    where
        F: Fn() -> M + Send + Sync + 'static,
    {
        self.on_hover = Some(Arc::new(handler));
        self
    }

    /// Set press handler (fires when mouse button pressed down)
    ///
    /// The handler is called when the left mouse button is pressed down
    /// within the widget's area.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui::prelude::*;
    ///
    /// #[derive(Clone)]
    /// enum Message { Pressed }
    ///
    /// let widget = interactive(label("Press me"))
    ///     .on_press(|| Message::Pressed);
    /// ```
    pub fn on_press<F>(mut self, handler: F) -> Self
    where
        F: Fn() -> M + Send + Sync + 'static,
    {
        self.on_press = Some(Arc::new(handler));
        self
    }

    /// Set release handler (fires when mouse button released)
    ///
    /// The handler is called when the left mouse button is released
    /// within the widget's area, regardless of where it was pressed.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui::prelude::*;
    ///
    /// #[derive(Clone)]
    /// enum Message { Released }
    ///
    /// let widget = interactive(label("Release here"))
    ///     .on_release(|| Message::Released);
    /// ```
    pub fn on_release<F>(mut self, handler: F) -> Self
    where
        F: Fn() -> M + Send + Sync + 'static,
    {
        self.on_release = Some(Arc::new(handler));
        self
    }

    /// Set drag handler (fires when mouse moves while button pressed)
    ///
    /// The handler is called when the mouse moves within the widget's area
    /// while the left mouse button is pressed.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui::prelude::*;
    ///
    /// #[derive(Clone)]
    /// enum Message { Dragging }
    ///
    /// let widget = interactive(label("Drag me"))
    ///     .on_drag(|| Message::Dragging);
    /// ```
    pub fn on_drag<F>(mut self, handler: F) -> Self
    where
        F: Fn() -> M + Send + Sync + 'static,
    {
        self.on_drag = Some(Arc::new(handler));
        self
    }

    /// Get reference to inner widget
    ///
    /// # Examples
    ///
    /// ```
    /// use tui::prelude::*;
    ///
    /// let label = label("Text");
    /// let interactive_label = interactive(label);
    /// let inner_ref = interactive_label.inner();
    /// ```
    pub fn inner(&self) -> &W {
        &self.inner
    }

    /// Get mutable reference to inner widget
    ///
    /// # Examples
    ///
    /// ```
    /// use tui::prelude::*;
    ///
    /// let mut interactive_label = interactive(label("Text"));
    /// let inner_mut = interactive_label.inner_mut();
    /// ```
    pub fn inner_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Consume wrapper and return inner widget
    ///
    /// # Examples
    ///
    /// ```
    /// use tui::prelude::*;
    ///
    /// let interactive_label = interactive(label("Text"));
    /// let label = interactive_label.into_inner();
    /// ```
    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<M, W> Interactive<M, W>
where
    M: Send + Sync,
    W: Widget<M>,
{
    /// Handle mouse events with cached area for hit testing
    fn handle_mouse_event(&mut self, mouse_event: &MouseEvent) -> EventResult<M> {
        // Get cached area (return Ignored if not yet rendered)
        let area = match *self.cached_area.read().unwrap() {
            Some(area) => area,
            None => return EventResult::Ignored,
        };

        // Hit test: check if mouse is within widget area
        let mouse_x = mouse_event.column;
        let mouse_y = mouse_event.row;
        let is_inside = mouse_x >= area.x()
            && mouse_x < area.x() + area.width()
            && mouse_y >= area.y()
            && mouse_y < area.y() + area.height();

        let mut state = *self.state.read().unwrap();
        let mut messages = Vec::new();

        match mouse_event.kind {
            MouseEventKind::Moved => {
                if is_inside {
                    // Mouse entered or is hovering
                    if !state.hovering {
                        state.hovering = true;
                        if let Some(ref handler) = self.on_hover {
                            messages.push(handler());
                        }
                    }
                } else {
                    // Mouse left
                    if state.hovering {
                        state.hovering = false;
                        // Could add on_hover_leave handler here if needed
                    }
                }
            }

            MouseEventKind::Down(MouseButton::Left) => {
                if is_inside {
                    state.pressed = true;
                    state.press_pos = Some((mouse_x, mouse_y));

                    if let Some(ref handler) = self.on_press {
                        messages.push(handler());
                    }
                }
            }

            MouseEventKind::Up(MouseButton::Left) => {
                if is_inside {
                    if let Some(ref handler) = self.on_release {
                        messages.push(handler());
                    }

                    // Check if this is a click (press and release in similar location)
                    if state.pressed {
                        if let Some(press_pos) = state.press_pos {
                            // Allow small movement (within 1 cell) for click
                            let dx = (mouse_x as i16 - press_pos.0 as i16).abs();
                            let dy = (mouse_y as i16 - press_pos.1 as i16).abs();

                            if dx <= 1 && dy <= 1 {
                                if let Some(ref handler) = self.on_click {
                                    messages.push(handler());
                                }
                            }
                        }
                    }
                }

                // Always reset press state on mouse up
                state.pressed = false;
                state.press_pos = None;
            }

            MouseEventKind::Drag(MouseButton::Left) => {
                if state.pressed && is_inside {
                    if let Some(ref handler) = self.on_drag {
                        messages.push(handler());
                    }
                }
            }

            _ => {}
        }

        // Update state
        *self.state.write().unwrap() = state;

        // Return result with messages
        if !messages.is_empty() {
            EventResult::Consumed(messages)
        } else if is_inside {
            // Consume event if mouse is inside, even without handlers
            // This prevents event from propagating to widgets behind
            EventResult::consumed()
        } else {
            EventResult::Ignored
        }
    }
}

impl<M, W> Widget<M> for Interactive<M, W>
where
    M: Send + Sync,
    W: Widget<M>,
{
    fn render(&self, chunk: &mut render::chunk::Chunk) {
        // Cache the area for hit testing
        *self.cached_area.write().unwrap() = Some(chunk.area());

        // Delegate rendering to inner widget (complete transparency)
        self.inner.render(chunk);
    }

    fn handle_event(&mut self, event: &Event) -> EventResult<M> {
        // First, try inner widget's event handling
        let inner_result = self.inner.handle_event(event);

        // If inner widget consumed the event, respect that
        if inner_result.is_consumed() {
            return inner_result;
        }

        // Otherwise, handle mouse events
        if let Event::Mouse(mouse_event) = event {
            return self.handle_mouse_event(mouse_event);
        }

        // Not a mouse event and inner didn't consume it
        EventResult::Ignored
    }

    fn constraints(&self) -> crate::layout::Constraints {
        // Completely transparent - use inner widget's constraints
        self.inner.constraints()
    }

    // Delegate focus-related methods to inner widget
    fn focusable(&self) -> bool {
        self.inner.focusable()
    }

    fn is_focused(&self) -> bool {
        self.inner.is_focused()
    }

    fn set_focused(&mut self, focused: bool) {
        self.inner.set_focused(focused);
    }

    fn build_focus_chain_recursive(
        &self,
        current_path: &mut Vec<usize>,
        chain: &mut Vec<crate::widget_id::WidgetId>,
    ) {
        self.inner.build_focus_chain_recursive(current_path, chain);
    }

    fn update_focus_states_recursive(
        &mut self,
        current_path: &[usize],
        focus_id: Option<crate::widget_id::WidgetId>,
    ) {
        self.inner
            .update_focus_states_recursive(current_path, focus_id);
    }
}

impl<M, W> std::fmt::Debug for Interactive<M, W>
where
    W: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Interactive")
            .field("inner", &self.inner)
            .field("has_click_handler", &self.on_click.is_some())
            .field("has_hover_handler", &self.on_hover.is_some())
            .field("has_press_handler", &self.on_press.is_some())
            .field("has_release_handler", &self.on_release.is_some())
            .field("has_drag_handler", &self.on_drag.is_some())
            .field("state", &*self.state.read().unwrap())
            .finish()
    }
}

impl<M, W> Clone for Interactive<M, W>
where
    W: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            cached_area: RwLock::new(*self.cached_area.read().unwrap()),
            on_click: self.on_click.clone(),
            on_hover: self.on_hover.clone(),
            on_press: self.on_press.clone(),
            on_release: self.on_release.clone(),
            on_drag: self.on_drag.clone(),
            state: RwLock::new(*self.state.read().unwrap()),
        }
    }
}

/// Create an Interactive wrapper around a widget
///
/// This is a convenience function for creating `Interactive<M, W>` wrappers
/// with a more ergonomic syntax.
///
/// # Examples
///
/// ```
/// use tui::prelude::*;
///
/// #[derive(Clone)]
/// enum Message { Clicked }
///
/// let clickable = interactive(label("Click me"))
///     .on_click(|| Message::Clicked);
/// ```
pub fn interactive<M, W>(widget: W) -> Interactive<M, W> {
    Interactive::new(widget)
}
