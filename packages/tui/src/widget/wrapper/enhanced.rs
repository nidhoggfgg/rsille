//! Enhanced widget wrapper - adds common functionality to any widget
//!
//! This module provides a unified wrapper that adds multiple enhancement capabilities
//! to any widget without deep nesting. All features are opt-in and can be combined freely.
//!
//! # Features
//!
//! - **Focus**: Make any widget focusable with Tab navigation
//! - **Hover**: Visual hover effects and hover events
//! - **Click**: Mouse click handling
//! - **Border**: Add borders with automatic focus ring
//! - **Padding**: Add padding around content
//! - **Styling**: State-aware styling (normal/hover/focus/disabled)
//!
//! # Examples
//!
//! ```
//! use tui::prelude::*;
//!
//! // Simple click handler
//! enhanced(label("Click me"))
//!     .on_click(|| Message::Clicked)
//!
//! // Full-featured component
//! enhanced(label("Complete"))
//!     .focusable()
//!     .hoverable()
//!     .bordered(BorderStyle::Single)
//!     .padding(Padding::all(1))
//!     .fg(Color::White)
//!     .hover_fg(Color::Cyan)
//!     .on_click(|| Message::Clicked)
//! ```

use crate::{
    event::{Event, EventResult, KeyCode, MouseButton, MouseEvent, MouseEventKind},
    layout::Constraints,
    style::{BorderStyle, Color, Padding, Style, ThemeManager},
    widget::Widget,
};
use render::area::Area;
use std::sync::{Arc, RwLock};

/// Internal state for tracking widget interactions
#[derive(Debug, Clone, Copy, Default)]
struct EnhancedState {
    /// Whether widget currently has keyboard focus
    focused: bool,
    /// Whether mouse is currently hovering over widget
    hovering: bool,
    /// Whether mouse button is pressed down on widget
    pressed: bool,
    /// Mouse press position for click detection
    press_pos: Option<(u16, u16)>,
    /// Whether widget is disabled
    disabled: bool,
}

/// Enhanced wrapper that adds common functionality to any widget
///
/// All features are optional and can be combined in any order through the builder API.
/// The wrapper maintains a single level of nesting regardless of how many features are used.
///
/// # Type Parameters
///
/// - `M`: Message type for event handlers
/// - `W`: Inner widget type
///
/// # Examples
///
/// ```
/// use tui::prelude::*;
///
/// #[derive(Clone)]
/// enum Message {
///     Clicked,
///     Hovered,
///     Focused,
/// }
///
/// let widget = enhanced(label("Interactive"))
///     .focusable()
///     .hoverable()
///     .on_click(|| Message::Clicked)
///     .on_mouse_enter(|| Message::MouseEntered)
///     .on_focus(|| Message::Focused);
/// ```
pub struct Enhanced<M, W> {
    /// The wrapped widget
    inner: W,

    // === Feature flags ===
    /// Whether widget can receive keyboard focus
    is_focusable: bool,
    /// Whether hover effects are enabled
    is_hoverable: bool,
    /// Border style (None = no border)
    border_style: Option<BorderStyle>,
    /// Padding around content (None = no padding)
    padding: Option<Padding>,

    // === Styling ===
    /// Base style for normal state
    base_style: Option<Style>,
    /// Style when hovering
    hover_style: Option<Style>,
    /// Style when focused
    focus_style: Option<Style>,
    /// Style when disabled
    disabled_style: Option<Style>,
    /// Border color (None = use theme default)
    border_color: Option<Color>,
    /// Border color when focused (None = use theme focus_ring)
    focus_border_color: Option<Color>,

    // === Event handlers ===
    /// Click handler (mouse down + up in same location)
    on_click: Option<Arc<dyn Fn() -> M + Send + Sync>>,
    /// Mouse enter handler (mouse enters widget area)
    on_mouse_enter: Option<Arc<dyn Fn() -> M + Send + Sync>>,
    /// Mouse leave handler (mouse leaves widget area)
    on_mouse_leave: Option<Arc<dyn Fn() -> M + Send + Sync>>,
    /// Focus handler (widget gains focus)
    on_focus: Option<Arc<dyn Fn() -> M + Send + Sync>>,
    /// Blur handler (widget loses focus)
    on_blur: Option<Arc<dyn Fn() -> M + Send + Sync>>,

    // === Internal state ===
    /// Cached area from last render (for hit testing)
    cached_area: RwLock<Option<Area>>,
    /// Cached widget path from last render (for hover tracking)
    cached_path: RwLock<Option<crate::hover::WidgetPath>>,
    /// Interaction state (focus, hover, pressed, etc.)
    state: RwLock<EnhancedState>,
}

impl<M, W> Enhanced<M, W> {
    /// Create a new Enhanced wrapper around a widget
    ///
    /// All features are disabled by default. Use builder methods to enable them.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui::widget::{Enhanced, Label};
    ///
    /// let label = Label::new("Text");
    /// let enhanced_label = Enhanced::<(), _>::new(label);
    /// ```
    ///
    /// Prefer using the `enhanced()` helper function for more ergonomic usage.
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            is_focusable: false,
            is_hoverable: false,
            border_style: None,
            padding: None,
            base_style: None,
            hover_style: None,
            focus_style: None,
            disabled_style: None,
            border_color: None,
            focus_border_color: None,
            on_click: None,
            on_mouse_enter: None,
            on_mouse_leave: None,
            on_focus: None,
            on_blur: None,
            cached_area: RwLock::new(None),
            cached_path: RwLock::new(None),
            state: RwLock::new(EnhancedState::default()),
        }
    }

    // === Feature toggles ===

    /// Make the widget focusable (can receive keyboard focus via Tab)
    ///
    /// # Examples
    ///
    /// ```
    /// enhanced(label("Focusable"))
    ///     .focusable()
    ///     .on_focus(|| Message::Focused)
    /// ```
    pub fn focusable(mut self) -> Self {
        self.is_focusable = true;
        self
    }

    /// Enable hover effects (visual feedback when mouse hovers)
    ///
    /// # Examples
    ///
    /// ```
    /// enhanced(label("Hover me"))
    ///     .hoverable()
    ///     .hover_fg(Color::Cyan)
    /// ```
    pub fn hoverable(mut self) -> Self {
        self.is_hoverable = true;
        self
    }

    /// Add a border around the widget
    ///
    /// The border color automatically changes to the focus ring color when focused.
    ///
    /// # Examples
    ///
    /// ```
    /// enhanced(label("Bordered"))
    ///     .bordered(BorderStyle::Single)
    ///     .border_color(Color::Blue)
    /// ```
    pub fn bordered(mut self, style: BorderStyle) -> Self {
        self.border_style = Some(style);
        self
    }

    /// Add padding around the widget content
    ///
    /// # Examples
    ///
    /// ```
    /// // Uniform padding
    /// enhanced(label("Padded"))
    ///     .padding(Padding::all(2))
    ///
    /// // Custom padding
    /// enhanced(label("Custom"))
    ///     .padding(Padding { top: 1, bottom: 1, left: 2, right: 2 })
    /// ```
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = Some(padding);
        self
    }

    /// Set disabled state
    ///
    /// Disabled widgets don't handle events and use muted styling.
    ///
    /// # Examples
    ///
    /// ```
    /// enhanced(label("Disabled"))
    ///     .disabled(true)
    /// ```
    pub fn disabled(self, disabled: bool) -> Self {
        self.state.write().unwrap().disabled = disabled;
        self
    }

    // === Styling methods ===

    /// Set foreground color for normal state
    pub fn fg(mut self, color: Color) -> Self {
        let style = self.base_style.unwrap_or(Style::default()).fg(color);
        self.base_style = Some(style);
        self
    }

    /// Set background color for normal state
    pub fn bg(mut self, color: Color) -> Self {
        let style = self.base_style.unwrap_or(Style::default()).bg(color);
        self.base_style = Some(style);
        self
    }

    /// Set foreground color for hover state
    pub fn hover_fg(mut self, color: Color) -> Self {
        let style = self.hover_style.unwrap_or(Style::default()).fg(color);
        self.hover_style = Some(style);
        self
    }

    /// Set background color for hover state
    pub fn hover_bg(mut self, color: Color) -> Self {
        let style = self.hover_style.unwrap_or(Style::default()).bg(color);
        self.hover_style = Some(style);
        self
    }

    /// Set foreground color for focus state
    pub fn focus_fg(mut self, color: Color) -> Self {
        let style = self.focus_style.unwrap_or(Style::default()).fg(color);
        self.focus_style = Some(style);
        self
    }

    /// Set background color for focus state
    pub fn focus_bg(mut self, color: Color) -> Self {
        let style = self.focus_style.unwrap_or(Style::default()).bg(color);
        self.focus_style = Some(style);
        self
    }

    /// Set custom border color
    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    /// Set custom border color for focus state
    pub fn focus_border_color(mut self, color: Color) -> Self {
        self.focus_border_color = Some(color);
        self
    }

    // === Event handler setters ===

    /// Set click handler (fires on mouse down + up in same location)
    ///
    /// # Examples
    ///
    /// ```
    /// enhanced(label("Click me"))
    ///     .on_click(|| Message::Clicked)
    /// ```
    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn() -> M + Send + Sync + 'static,
    {
        self.on_click = Some(Arc::new(handler));
        self
    }

    /// Set mouse enter handler (fires when mouse enters widget area)
    ///
    /// This event fires only once when the mouse crosses the boundary into the widget.
    /// It will not fire again until the mouse leaves and re-enters.
    ///
    /// # Examples
    ///
    /// ```
    /// enhanced(label("Hover me"))
    ///     .hoverable()
    ///     .on_mouse_enter(|| Message::MouseEntered)
    /// ```
    pub fn on_mouse_enter<F>(mut self, handler: F) -> Self
    where
        F: Fn() -> M + Send + Sync + 'static,
    {
        self.on_mouse_enter = Some(Arc::new(handler));
        self
    }

    /// Set mouse leave handler (fires when mouse leaves widget area)
    ///
    /// This event fires only once when the mouse crosses the boundary out of the widget.
    ///
    /// # Examples
    ///
    /// ```
    /// enhanced(label("Hover me"))
    ///     .hoverable()
    ///     .on_mouse_leave(|| Message::MouseLeft)
    /// ```
    pub fn on_mouse_leave<F>(mut self, handler: F) -> Self
    where
        F: Fn() -> M + Send + Sync + 'static,
    {
        self.on_mouse_leave = Some(Arc::new(handler));
        self
    }

    /// Set focus handler (fires when widget gains focus)
    ///
    /// # Examples
    ///
    /// ```
    /// enhanced(label("Focus me"))
    ///     .focusable()
    ///     .on_focus(|| Message::Focused)
    /// ```
    pub fn on_focus<F>(mut self, handler: F) -> Self
    where
        F: Fn() -> M + Send + Sync + 'static,
    {
        self.on_focus = Some(Arc::new(handler));
        self
    }

    /// Set blur handler (fires when widget loses focus)
    ///
    /// # Examples
    ///
    /// ```
    /// enhanced(label("Focus me"))
    ///     .focusable()
    ///     .on_blur(|| Message::Blurred)
    /// ```
    pub fn on_blur<F>(mut self, handler: F) -> Self
    where
        F: Fn() -> M + Send + Sync + 'static,
    {
        self.on_blur = Some(Arc::new(handler));
        self
    }

    /// Get reference to inner widget
    pub fn inner(&self) -> &W {
        &self.inner
    }

    /// Get mutable reference to inner widget
    pub fn inner_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Consume wrapper and return inner widget
    pub fn into_inner(self) -> W {
        self.inner
    }
}

/// Create an Enhanced wrapper around a widget
///
/// This is a convenience function for creating `Enhanced<M, W>` wrappers
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
/// let widget = enhanced(label("Click me"))
///     .on_click(|| Message::Clicked);
/// ```
pub fn enhanced<M, W>(widget: W) -> Enhanced<M, W> {
    Enhanced::new(widget)
}

// === Widget trait implementation ===

impl<M, W> Widget<M> for Enhanced<M, W>
where
    M: Send + Sync,
    W: Widget<M>,
{
    fn render(&self, chunk: &mut render::chunk::Chunk) {
        let area = chunk.area();

        // Cache area for hit testing
        *self.cached_area.write().unwrap() = Some(area);

        // Register with HoverManager if we have mouse enter/leave handlers
        if self.on_mouse_enter.is_some() || self.on_mouse_leave.is_some() {
            let path = crate::hover::RenderContext::current_path();
            *self.cached_path.write().unwrap() = Some(path.clone());
            crate::hover::HoverManager::global().register_widget(path, area);
        }

        let state = self.state.read().unwrap();

        // Step 1: Compute effective style and fill background
        let effective_style = self.compute_effective_style(&state);
        if effective_style.bg_color.is_some() {
            let render_style = effective_style.to_render_style();
            let _ = chunk.fill(0, 0, area.width(), area.height(), ' ', render_style);
        }

        // Step 2: Apply padding to get inner area
        let mut content_area = area;
        if let Some(ref padding) = self.padding {
            let new_x = content_area.x().saturating_add(padding.left);
            let new_y = content_area.y().saturating_add(padding.top);
            let new_width = content_area
                .width()
                .saturating_sub(padding.left + padding.right);
            let new_height = content_area
                .height()
                .saturating_sub(padding.top + padding.bottom);

            if new_width > 0 && new_height > 0 {
                content_area = Area::new(
                    (new_x, new_y).into(),
                    (new_width, new_height).into(),
                );
            } else {
                // Padding consumed all space
                return;
            }
        }

        // Step 3: Render border and shrink content area
        if let Some(ref border_style) = self.border_style {
            let border_color = self.compute_border_color(&state);
            let border_render_style = Style::default().fg(border_color).to_render_style();

            // Render border using border_renderer
            use crate::layout::border_renderer;
            border_renderer::render_border(chunk, *border_style, border_render_style);

            // Shrink content area for border (1 cell on each side)
            let new_x = content_area.x().saturating_add(1);
            let new_y = content_area.y().saturating_add(1);
            let new_width = content_area.width().saturating_sub(2);
            let new_height = content_area.height().saturating_sub(2);

            if new_width > 0 && new_height > 0 {
                content_area = Area::new(
                    (new_x, new_y).into(),
                    (new_width, new_height).into(),
                );
            } else {
                // Border consumed all space
                return;
            }
        }

        // Step 4: Create subchunk for inner widget and render it
        if let Ok(mut inner_chunk) = chunk.from_area(content_area) {
            self.inner.render(&mut inner_chunk);
        }
    }

    fn handle_event(&mut self, event: &Event) -> EventResult<M> {
        // First, let inner widget handle the event
        let inner_result = self.inner.handle_event(event);
        if inner_result.is_consumed() {
            return inner_result;
        }

        // Enhanced processes the event
        let mut messages = Vec::new();

        match event {
            Event::Mouse(mouse_event) => {
                // Check for hover enter/leave events ONLY on mouse movement
                use crate::event::MouseEventKind;
                if matches!(mouse_event.kind, MouseEventKind::Moved) {
                    if let Some(ref path) = *self.cached_path.read().unwrap() {
                        if let Some(ref handler) = self.on_mouse_enter {
                            if crate::hover::HoverManager::global().should_fire_enter(path) {
                                messages.push(handler());
                            }
                        }

                        if let Some(ref handler) = self.on_mouse_leave {
                            if crate::hover::HoverManager::global().should_fire_leave(path) {
                                messages.push(handler());
                            }
                        }
                    }
                }

                // Handle other mouse events (click, etc.)
                messages.extend(self.handle_mouse_event(mouse_event));
            }
            Event::Key(key_event) => {
                // Only handle key events if focused
                if self.state.read().unwrap().focused {
                    // Currently no key handling, but could add Enter/Space for activation
                    match key_event.code {
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            if let Some(ref handler) = self.on_click {
                                messages.push(handler());
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        if !messages.is_empty() {
            EventResult::Consumed(messages)
        } else {
            EventResult::Ignored
        }
    }

    fn constraints(&self) -> Constraints {
        let mut constraints = self.inner.constraints();

        // Add padding to constraints
        if let Some(ref padding) = self.padding {
            constraints.min_width += padding.left + padding.right;
            constraints.min_height += padding.top + padding.bottom;
            if let Some(max_w) = constraints.max_width {
                constraints.max_width = Some(max_w + padding.left + padding.right);
            }
            if let Some(max_h) = constraints.max_height {
                constraints.max_height = Some(max_h + padding.top + padding.bottom);
            }
        }

        // Add border to constraints (2 cells: 1 on each side)
        if self.border_style.is_some() {
            constraints.min_width += 2;
            constraints.min_height += 2;
            if let Some(max_w) = constraints.max_width {
                constraints.max_width = Some(max_w + 2);
            }
            if let Some(max_h) = constraints.max_height {
                constraints.max_height = Some(max_h + 2);
            }
        }

        constraints
    }

    fn focusable(&self) -> bool {
        self.is_focusable || self.inner.focusable()
    }

    fn is_focused(&self) -> bool {
        self.state.read().unwrap().focused
    }

    fn set_focused(&mut self, focused: bool) {
        let mut state = self.state.write().unwrap();
        state.focused = focused;
        drop(state);

        // Propagate to inner widget
        self.inner.set_focused(focused);

        // TODO: Trigger focus/blur events
        // This requires collecting messages, but set_focused doesn't return EventResult
        // For now, events will fire on next handle_event call
    }

    fn build_focus_chain_recursive(
        &self,
        current_path: &mut Vec<usize>,
        chain: &mut Vec<crate::widget_id::WidgetId>,
    ) {
        // Layout has already checked focusable() and added us to chain if needed
        // Just delegate to inner in case it's a container with children
        self.inner.build_focus_chain_recursive(current_path, chain);
    }

    fn update_focus_states_recursive(
        &mut self,
        current_path: &[usize],
        focus_id: Option<crate::widget_id::WidgetId>,
    ) {
        // Layout has already called set_focused() on us
        // Just delegate to inner in case it's a container with children
        self.inner
            .update_focus_states_recursive(current_path, focus_id);
    }
}

// === Helper methods ===

impl<M, W> Enhanced<M, W> {
    /// Compute effective style based on current state
    fn compute_effective_style(&self, state: &EnhancedState) -> Style {
        if state.disabled {
            self.disabled_style.unwrap_or_else(|| {
                ThemeManager::global().with_theme(|t| t.styles.disabled)
            })
        } else if state.focused {
            self.focus_style
                .or(self.base_style)
                .unwrap_or_else(|| {
                    ThemeManager::global().with_theme(|t| t.styles.interactive_focused)
                })
        } else if state.hovering {
            self.hover_style
                .or(self.base_style)
                .unwrap_or_else(|| ThemeManager::global().with_theme(|t| t.styles.hover))
        } else {
            self.base_style.unwrap_or_else(|| {
                ThemeManager::global().with_theme(|t| t.styles.interactive)
            })
        }
    }

    /// Compute border color based on current state
    fn compute_border_color(&self, state: &EnhancedState) -> Color {
        if state.focused {
            self.focus_border_color.unwrap_or_else(|| {
                ThemeManager::global().with_theme(|t| t.colors.focus_ring)
            })
        } else {
            self.border_color.unwrap_or_else(|| {
                ThemeManager::global().with_theme(|t| t.colors.border)
            })
        }
    }

    /// Handle mouse events with hit testing
    fn handle_mouse_event(&mut self, mouse_event: &MouseEvent) -> Vec<M> {
        // Get cached area
        let area = match *self.cached_area.read().unwrap() {
            Some(area) => area,
            None => return Vec::new(),
        };

        // Hit test: check if mouse is within widget area
        let mouse_x = mouse_event.column;
        let mouse_y = mouse_event.row;
        let is_inside = mouse_x >= area.x()
            && mouse_x < area.x() + area.width()
            && mouse_y >= area.y()
            && mouse_y < area.y() + area.height();

        let mut state = self.state.write().unwrap();
        let mut messages = Vec::new();

        match mouse_event.kind {
            // Note: Mouse enter/leave is now handled by HoverManager
            // to work correctly across component tree rebuilds

            MouseEventKind::Down(MouseButton::Left) => {
                if is_inside {
                    state.pressed = true;
                    state.press_pos = Some((mouse_x, mouse_y));
                }
            }

            MouseEventKind::Up(MouseButton::Left) => {
                if is_inside && state.pressed {
                    // Check if this is a click (press and release in similar location)
                    if let Some(press_pos) = state.press_pos {
                        let dx = (mouse_x as i16 - press_pos.0 as i16).abs();
                        let dy = (mouse_y as i16 - press_pos.1 as i16).abs();

                        // Allow small movement (within 1 cell) for click
                        if dx <= 1 && dy <= 1 {
                            if let Some(ref handler) = self.on_click {
                                messages.push(handler());
                            }
                        }
                    }
                }

                // Always reset press state
                state.pressed = false;
                state.press_pos = None;
            }

            _ => {}
        }

        messages
    }
}

// === Debug implementation ===

impl<M, W> std::fmt::Debug for Enhanced<M, W>
where
    W: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Enhanced")
            .field("inner", &self.inner)
            .field("is_focusable", &self.is_focusable)
            .field("is_hoverable", &self.is_hoverable)
            .field("border_style", &self.border_style)
            .field("padding", &self.padding)
            .field("has_on_click", &self.on_click.is_some())
            .field("has_on_mouse_enter", &self.on_mouse_enter.is_some())
            .field("has_on_mouse_leave", &self.on_mouse_leave.is_some())
            .field("has_on_focus", &self.on_focus.is_some())
            .field("has_on_blur", &self.on_blur.is_some())
            .field("state", &*self.state.read().unwrap())
            .finish()
    }
}

// === Clone implementation ===

impl<M, W> Clone for Enhanced<M, W>
where
    W: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            is_focusable: self.is_focusable,
            is_hoverable: self.is_hoverable,
            border_style: self.border_style,
            padding: self.padding,
            base_style: self.base_style,
            hover_style: self.hover_style,
            focus_style: self.focus_style,
            disabled_style: self.disabled_style,
            border_color: self.border_color,
            focus_border_color: self.focus_border_color,
            on_click: self.on_click.clone(),
            on_mouse_enter: self.on_mouse_enter.clone(),
            on_mouse_leave: self.on_mouse_leave.clone(),
            on_focus: self.on_focus.clone(),
            on_blur: self.on_blur.clone(),
            cached_area: RwLock::new(*self.cached_area.read().unwrap()),
            cached_path: RwLock::new(self.cached_path.read().unwrap().clone()),
            state: RwLock::new(*self.state.read().unwrap()),
        }
    }
}
