//! Flex widget for layout composition

use render::area::Area;

use super::border_renderer::{render_background, render_border};
use super::taffy_bridge::TaffyBridge;
use super::Overflow;
use crate::event::{Event, EventResult};
use crate::layout::Constraints;
use crate::style::{BorderStyle, Padding, Style, ThemeManager};
use crate::widget::{IntoWidget, Widget};
use std::sync::Mutex;
use taffy::style::{AlignItems, JustifyContent};

use super::layout::Layout;

/// Layout direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Vertical,
    Horizontal,
}

/// Flex widget that arranges children using flexbox layout
pub struct Flex<M = ()> {
    children: Vec<Box<dyn Widget<M>>>,
    direction: Direction,
    gap: u16,
    padding: Padding,
    border: Option<BorderStyle>,
    style: Style,
    align_items: Option<AlignItems>,
    justify_content: Option<JustifyContent>,
    overflow: Overflow,
    /// Cached layout areas from last render (for mouse event handling)
    /// Using RefCell since rendering is single-threaded
    cached_child_areas: Mutex<Vec<Area>>,
}

impl<M> std::fmt::Debug for Flex<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Flex")
            .field("children", &self.children.len())
            .field("direction", &self.direction)
            .field("gap", &self.gap)
            .field("padding", &self.padding)
            .field("border", &self.border)
            .field("style", &self.style)
            .field("align_items", &self.align_items)
            .field("justify_content", &self.justify_content)
            .field("overflow", &self.overflow)
            .finish()
    }
}

impl<M> Flex<M> {
    /// Create a new flex layout with the specified direction
    fn with_direction(children: Vec<Box<dyn Widget<M>>>, direction: Direction) -> Self {
        Self {
            children,
            direction,
            gap: 0,
            padding: Padding::ZERO,
            border: None,
            style: Style::default(),
            align_items: None,
            justify_content: None,
            overflow: Overflow::default(),
            cached_child_areas: Mutex::new(Vec::new()),
        }
    }

    /// Create a new flex layout with vertical layout
    ///
    /// # Examples
    /// ```
    /// use tui::prelude::*;
    ///
    /// let flex_layout: Flex<()> = Flex::vertical(vec![
    ///     Box::new(Label::new("Line 1")),
    ///     Box::new(Label::new("Line 2")),
    /// ]);
    /// ```
    pub fn vertical(children: Vec<Box<dyn Widget<M>>>) -> Self {
        Self::with_direction(children, Direction::Vertical)
    }

    /// Create a new flex layout with horizontal layout
    ///
    /// # Examples
    /// ```
    /// use tui::prelude::*;
    ///
    /// let flex_layout: Flex<()> = Flex::horizontal(vec![
    ///     Box::new(Button::new("OK")),
    ///     Box::new(Button::new("Cancel")),
    /// ]);
    /// ```
    pub fn horizontal(children: Vec<Box<dyn Widget<M>>>) -> Self {
        Self::with_direction(children, Direction::Horizontal)
    }

    /// Create a new empty flex layout
    pub fn new() -> Self {
        Self::vertical(Vec::new())
    }

    /// Set the gap between children (in terminal cells)
    pub fn gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    /// Set the inner padding
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    /// Set the flex layout style
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set the flex layout border
    pub fn border(mut self, border: BorderStyle) -> Self {
        self.border = Some(border);
        self
    }

    /// Set the alignment of items along the cross axis
    pub fn align_items(mut self, align: AlignItems) -> Self {
        self.align_items = Some(align);
        self
    }

    /// Set the justification of content along the main axis
    pub fn justify_content(mut self, justify: JustifyContent) -> Self {
        self.justify_content = Some(justify);
        self
    }

    /// Set the overflow behavior for children that exceed the flex layout bounds
    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.overflow = overflow;
        self
    }

    /// Conditionally modify the flex layout
    ///
    /// If the condition is true, applies the given function to the flex layout.
    /// Otherwise, returns the flex layout unchanged.
    ///
    /// # Examples
    /// ```
    /// use tui::prelude::*;
    ///
    /// let show_footer = true;
    /// let flex_layout = col()
    ///     .child(label("Header"))
    ///     .when(show_footer, |c| c.child(label("Footer")));
    /// ```
    pub fn when<F>(self, condition: bool, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        if condition {
            f(self)
        } else {
            self
        }
    }

    /// Add a child widget
    pub fn add_child(&mut self, child: Box<dyn Widget<M>>) {
        self.children.push(child);
    }
}

// Methods that require Send + Sync bounds
impl<M: Send + Sync> Flex<M> {
    /// Add a single child widget using fluent API
    ///
    /// # Examples
    /// ```
    /// use tui::prelude::*;
    ///
    /// let flex_layout = col()
    ///     .child(label("Hello"))
    ///     .child(label("World"));
    /// ```
    pub fn child(mut self, widget: impl IntoWidget<M>) -> Self {
        self.children.push(widget.into_widget());
        self
    }

    /// Add multiple child widgets using fluent API
    ///
    /// Accepts any iterator that yields items implementing `IntoWidget<M>`.
    ///
    /// # Examples
    /// ```
    /// use tui::prelude::*;
    ///
    /// let items = vec!["Item 1", "Item 2", "Item 3"];
    /// let flex_layout = col()
    ///     .children(items.iter().map(|text| label(text)));
    /// ```
    pub fn children<I>(mut self, widgets: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoWidget<M>,
    {
        self.children
            .extend(widgets.into_iter().map(|w| w.into_widget()));
        self
    }
}

impl<M> Flex<M> {
    /// Remove a child at the specified index
    pub fn remove_child(&mut self, index: usize) -> Box<dyn Widget<M>> {
        self.children.remove(index)
    }

    /// Get the number of children
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Check if flex layout is empty
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Get reference to children
    pub fn child_widgets(&self) -> &[Box<dyn Widget<M>>] {
        &self.children
    }
}

// Methods that require Clone bound
impl<M: Clone> Flex<M> {
    /// Handle event and collect any generated messages
    pub fn handle_event_with_messages(&mut self, event: &Event) -> (EventResult<M>, Vec<M>)
    where
        M: Clone,
    {
        self.handle_event_with_focus(event, &[], None)
    }

    /// Internal helper for event handling with focus routing
    ///
    /// This method contains the core event routing logic used by both
    /// the public handle_event_with_focus and Layout trait implementation.
    fn handle_event_internal(
        &mut self,
        event: &Event,
        current_path: &[usize],
        focus_path: Option<&[usize]>,
        include_fallback: bool,
    ) -> (EventResult<M>, Vec<M>)
    where
        M: Clone,
    {
        let mut all_messages = Vec::new();

        // For mouse events, use spatial routing
        if let Event::Mouse(mouse_event) = event {
            use crate::event::MouseEventKind;

            // For MouseMoved events, broadcast to ALL children
            // This is necessary for hover enter/leave events to work correctly
            // (a widget needs to receive MouseMoved events even when mouse is outside)
            if matches!(mouse_event.kind, MouseEventKind::Moved) {
                for child in self.children.iter_mut() {
                    let result = child.handle_event(event);
                    let messages = result.messages_ref().to_vec();
                    all_messages.extend(messages);

                    if result.is_consumed() {
                        return (EventResult::consumed(), all_messages);
                    }
                }
                return (EventResult::Ignored, all_messages);
            }

            // For other mouse events (click, drag, etc.), use spatial routing
            let cached_areas = self.cached_child_areas.lock().unwrap();
            if !cached_areas.is_empty() {
                for (idx, child_area) in cached_areas.iter().enumerate() {
                    // Check if mouse is within this child's area
                    let is_hit = mouse_event.column >= child_area.x()
                        && mouse_event.column < child_area.x() + child_area.width()
                        && mouse_event.row >= child_area.y()
                        && mouse_event.row < child_area.y() + child_area.height();

                    if is_hit {
                        // Route event to this specific child
                        if let Some(child) = self.children.get_mut(idx) {
                            let result = child.handle_event(event);
                            let messages = result.messages_ref().to_vec();
                            all_messages.extend(messages);

                            if result.is_consumed() {
                                return (EventResult::consumed(), all_messages);
                            }
                        }
                    }
                }
            }
            return (EventResult::Ignored, all_messages);
        }

        // For keyboard events, use focus-based routing
        if let Event::Key(_) = event {
            // Check if focus is within our children
            if let Some(focus) = focus_path {
                if focus.starts_with(current_path) && focus.len() > current_path.len() {
                    // Focus is in one of our children
                    let child_idx = focus[current_path.len()];

                    if let Some(child) = self.children.get_mut(child_idx) {
                        let result = child.handle_event(event);
                        let messages = result.messages_ref().to_vec();
                        all_messages.extend(messages);

                        if result.is_consumed() {
                            return (EventResult::consumed(), all_messages);
                        }
                    }
                }
            }
        }

        // Fallback: try each child sequentially (for events not routed by focus)
        if include_fallback {
            for child in &mut self.children {
                let result = child.handle_event(event);
                let messages = result.messages_ref().to_vec();
                all_messages.extend(messages);

                if result.is_consumed() {
                    return (EventResult::consumed(), all_messages);
                }
            }
        }

        (EventResult::Ignored, all_messages)
    }

    /// Handle event with focus information
    ///
    /// Routes events based on focus state for keyboard events.
    ///
    /// # Arguments
    /// * `event` - The event to handle
    /// * `current_path` - Current path in widget tree
    /// * `focus_path` - Path to the focused widget (if any)
    pub fn handle_event_with_focus(
        &mut self,
        event: &Event,
        current_path: &[usize],
        focus_path: Option<&[usize]>,
    ) -> (EventResult<M>, Vec<M>)
    where
        M: Clone,
    {
        self.handle_event_internal(event, current_path, focus_path, true)
    }
}

impl<M: Clone> Widget<M> for Flex<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        // Apply theme: merge explicit style with theme default
        let theme_style = ThemeManager::global().with_theme(|theme| theme.styles.surface);
        let final_style = self.style.merge(theme_style);

        // Convert TUI style to render style
        let render_style = final_style.to_render_style();

        // Calculate area inside border (if any)
        let border_area = if self.border.is_some() {
            // Reserve 1 cell on each side for border
            if area.width() < 2 || area.height() < 2 {
                return; // Not enough space for border
            }
            Area::new(
                (area.x() + 1, area.y() + 1).into(),
                (area.width() - 2, area.height() - 2).into(),
            )
        } else {
            area
        };

        // Apply flex layout background if specified (only inside border)
        if final_style.bg_color.is_some() {
            render_background(chunk, render_style);
        }

        // Draw border after background (so it's on top)
        if let Some(border) = self.border {
            render_border(chunk, border, render_style);
        }

        // Calculate inner area after padding
        let inner = border_area.shrink_saturating(
            self.padding.top,
            self.padding.bottom,
            self.padding.left,
            self.padding.right,
        );

        if inner.width() == 0 || inner.height() == 0 {
            return;
        }

        // Compute layout using Taffy
        let mut bridge = TaffyBridge::new();
        let child_areas = match bridge.compute_layout(
            &self.children,
            inner,
            self.direction,
            self.gap,
            self.align_items,
            self.justify_content,
        ) {
            Ok(areas) => areas,
            Err(_) => {
                // Layout computation failed, cannot render children
                return;
            }
        };

        // Cache layout info for mouse event handling
        *self.cached_child_areas.lock().unwrap() = child_areas.clone();

        // Render each child in its allocated area using sequential sub-chunk creation
        for (index, (child, child_area)) in self.children.iter().zip(child_areas).enumerate() {
            // Skip rendering if the child has zero dimensions
            if child_area.width() == 0 || child_area.height() == 0 {
                continue;
            }

            // Apply overflow handling
            match self.overflow {
                Overflow::Hidden => {
                    // Skip child if it's completely outside the inner bounds
                    if !child_area.intersects(&inner) {
                        continue;
                    }
                }
                Overflow::Visible => {
                    // Allow rendering outside bounds
                }
            }

            // Create a sub-chunk for this child
            if let Ok(mut child_chunk) = chunk.from_area(child_area) {
                // Maintain render path for hover tracking
                crate::hover::RenderContext::push_index(index);
                child.render(&mut child_chunk);
                crate::hover::RenderContext::pop_index();
            }
        }
    }

    fn handle_event(&mut self, event: &Event) -> EventResult<M> {
        let (result, messages) = self.handle_event_with_messages(event);
        if result.is_consumed() && !messages.is_empty() {
            EventResult::Consumed(messages)
        } else if result.is_consumed() {
            EventResult::consumed()
        } else {
            EventResult::Ignored
        }
    }

    fn constraints(&self) -> Constraints {
        // Calculate border size (2 cells if border exists, 0 otherwise)
        let border_size = if self.border.is_some() { 2 } else { 0 };

        if self.children.is_empty() {
            return Constraints::fixed(
                self.padding.horizontal_total() + border_size,
                self.padding.vertical_total() + border_size,
            );
        }

        // Aggregate child constraints based on direction
        match self.direction {
            Direction::Vertical => {
                let total_height = self
                    .children
                    .iter()
                    .map(|c| c.constraints().min_height)
                    .sum::<u16>()
                    + (self.children.len() as u16 - 1) * self.gap
                    + self.padding.vertical_total()
                    + border_size;

                let max_width = self
                    .children
                    .iter()
                    .map(|c| c.constraints().min_width)
                    .max()
                    .unwrap_or(0)
                    + self.padding.horizontal_total()
                    + border_size;

                Constraints {
                    min_width: max_width,
                    max_width: None,
                    min_height: total_height,
                    max_height: None,
                    flex: Some(1.0),
                }
            }
            Direction::Horizontal => {
                let total_width = self
                    .children
                    .iter()
                    .map(|c| c.constraints().min_width)
                    .sum::<u16>()
                    + (self.children.len() as u16 - 1) * self.gap
                    + self.padding.horizontal_total()
                    + border_size;

                let max_height = self
                    .children
                    .iter()
                    .map(|c| c.constraints().min_height)
                    .max()
                    .unwrap_or(0)
                    + self.padding.vertical_total()
                    + border_size;

                Constraints {
                    min_width: total_width,
                    max_width: None,
                    min_height: max_height,
                    max_height: Some(max_height), // Fixed: set max_height to ensure fixed height
                    flex: None,                   // Fixed: row should not flex vertically
                }
            }
        }
    }

    fn build_focus_chain_recursive(
        &self,
        current_path: &mut Vec<usize>,
        chain: &mut Vec<crate::widget_id::WidgetId>,
        registry: &mut crate::focus::WidgetRegistry,
    ) {
        use smallvec::SmallVec;

        // For nested layouts, traverse children without adding self again
        // (self was already added by parent's traversal)
        for (idx, child) in self.children.iter().enumerate() {
            current_path.push(idx);

            // If child is focusable, add to chain with stable ID
            if child.focusable() {
                let widget_key = child.widget_key();
                let widget_id =
                    crate::widget_id::WidgetId::from_path_and_key(current_path, widget_key);
                chain.push(widget_id);
                registry.register(widget_id, SmallVec::from_slice(current_path));
            }

            // Recursively build focus chain for nested layouts
            child.build_focus_chain_recursive(current_path, chain, registry);

            current_path.pop();
        }
    }

    fn update_focus_states_recursive(
        &mut self,
        current_path: &[usize],
        focus_id: Option<crate::widget_id::WidgetId>,
    ) {
        // For nested layouts, traverse children and update their focus states
        for (idx, child) in self.children.iter_mut().enumerate() {
            let mut child_path = current_path.to_vec();
            child_path.push(idx);

            let widget_key = child.widget_key();
            let child_id = crate::widget_id::WidgetId::from_path_and_key(&child_path, widget_key);
            let is_focused = focus_id.as_ref() == Some(&child_id);
            child.set_focused(is_focused);

            // Recursively update focus states for nested layouts
            child.update_focus_states_recursive(&child_path, focus_id.clone());
        }
    }
}

impl<M> Default for Flex<M> {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a new empty vertical flex layout
///
/// Shorthand for `Flex::new()` (which defaults to vertical).
///
/// # Examples
/// ```
/// use tui::prelude::*;
///
/// let flex_layout = col()
///     .gap(1)
///     .child(label("Line 1"))
///     .child(label("Line 2"));
/// ```
pub fn col<M>() -> Flex<M> {
    Flex::new()
}

/// Create a new empty horizontal flex layout
///
/// Shorthand for `Flex::horizontal(Vec::new())`.
///
/// # Examples
/// ```
/// use tui::prelude::*;
///
/// let flex_layout = row()
///     .gap(2)
///     .child(button("OK"))
///     .child(button("Cancel"));
/// ```
pub fn row<M>() -> Flex<M> {
    Flex::horizontal(Vec::new())
}

// Implement Layout trait for Flex
impl<M: Clone> Layout<M> for Flex<M> {
    fn update_focus_states(
        &mut self,
        focus_id: Option<crate::widget_id::WidgetId>,
        _registry: &crate::focus::WidgetRegistry,
    ) {
        // We need to rebuild child paths to check focus
        // But we can get the parent path from the first child in registry if available
        let base_path = Vec::new();

        for (idx, child) in self.children.iter_mut().enumerate() {
            let mut child_path = base_path.clone();
            child_path.push(idx);

            // Build child ID to check if it's focused
            let widget_key = child.widget_key();
            let child_id = crate::widget_id::WidgetId::from_path_and_key(&child_path, widget_key);

            // Check if this child is focused by comparing IDs directly (O(1))
            let is_focused = focus_id.as_ref() == Some(&child_id);
            child.set_focused(is_focused);

            // Recursively update nested widgets
            child.update_focus_states_recursive(&child_path, focus_id);
        }
    }

    fn handle_event_with_focus(
        &mut self,
        event: &Event,
        focus_id: Option<crate::widget_id::WidgetId>,
        registry: &crate::focus::WidgetRegistry,
    ) -> (EventResult<M>, Vec<M>) {
        // Get focus path from registry for event routing
        let focus_path = focus_id.and_then(|id| registry.get_path(&id));

        // Delegate to internal helper with fallback enabled
        // This ensures unfocused widgets like keyboard_controller can still receive events
        self.handle_event_internal(event, &[], focus_path, true)
    }
}
