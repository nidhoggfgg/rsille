//! Container widget for layout composition

use render::area::Area;

use super::border_renderer::{render_background, render_border};
use super::taffy_bridge::TaffyBridge;
use crate::event::{Event, EventResult};
use crate::layout::Constraints;
use crate::style::{Padding, Style};
use crate::widget::{AnyWidget, Widget};
use std::sync::RwLock;

/// Layout direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Vertical,
    Horizontal,
}

/// Container widget that arranges children using flexbox layout
pub struct Container<M = ()> {
    children: Vec<AnyWidget<M>>,
    direction: Direction,
    gap: u16,
    padding: Padding,
    style: Style,
    /// Cached layout areas from last render (for mouse event handling)
    cached_child_areas: RwLock<Vec<Area>>,
}

impl<M> std::fmt::Debug for Container<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Container")
            .field("children", &self.children.len())
            .field("direction", &self.direction)
            .field("gap", &self.gap)
            .field("padding", &self.padding)
            .field("style", &self.style)
            .finish()
    }
}

impl<M> Container<M> {
    /// Create a new container with the specified direction
    fn with_direction(children: Vec<AnyWidget<M>>, direction: Direction) -> Self {
        Self {
            children,
            direction,
            gap: 0,
            padding: Padding::ZERO,
            style: Style::default(),
            cached_child_areas: RwLock::new(Vec::new()),
        }
    }

    /// Create a new container with vertical layout
    ///
    /// # Examples
    /// ```
    /// use tui::prelude::*;
    ///
    /// let container: Container<()> = Container::vertical(vec![
    ///     Label::new("Line 1").into(),
    ///     Label::new("Line 2").into(),
    /// ]);
    /// ```
    pub fn vertical(children: Vec<AnyWidget<M>>) -> Self {
        Self::with_direction(children, Direction::Vertical)
    }

    /// Create a new container with horizontal layout
    ///
    /// # Examples
    /// ```
    /// use tui::prelude::*;
    ///
    /// let container: Container<()> = Container::horizontal(vec![
    ///     Button::new("OK").into(),
    ///     Button::new("Cancel").into(),
    /// ]);
    /// ```
    pub fn horizontal(children: Vec<AnyWidget<M>>) -> Self {
        Self::with_direction(children, Direction::Horizontal)
    }

    /// Create a new empty container
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

    /// Set the container style
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Add a child widget
    pub fn add_child(&mut self, child: impl Into<AnyWidget<M>>) {
        self.children.push(child.into());
    }

    /// Remove a child at the specified index
    pub fn remove_child(&mut self, index: usize) -> AnyWidget<M> {
        self.children.remove(index)
    }

    /// Get the number of children
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Check if container is empty
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Get reference to children
    pub fn children(&self) -> &[AnyWidget<M>] {
        &self.children
    }

    /// Handle event and collect any generated messages
    pub fn handle_event_with_messages(&mut self, event: &Event) -> (EventResult<M>, Vec<M>)
    where
        M: Clone,
    {
        let mut all_messages = Vec::new();

        // For mouse events, check if the click is within any child's area
        if let Event::Mouse(mouse_event) = event {
            // Only handle clicks within our cached layout
            let cached_areas = self.cached_child_areas.read().unwrap();
            if !cached_areas.is_empty() {
                for (idx, child_area) in cached_areas.iter().enumerate() {
                    // Check if mouse is within this child's area
                    if mouse_event.column >= child_area.x()
                        && mouse_event.column < child_area.x() + child_area.width()
                        && mouse_event.row >= child_area.y()
                        && mouse_event.row < child_area.y() + child_area.height()
                    {
                        // Route event to this specific child
                        if let Some(child) = self.children.get_mut(idx) {
                            let (result, messages) = child.handle_event_with_messages(event);
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

        // For non-mouse events, try each child until one consumes the event
        for child in &mut self.children {
            let (result, messages) = child.handle_event_with_messages(event);
            all_messages.extend(messages);

            if result.is_consumed() {
                return (EventResult::consumed(), all_messages);
            }
        }

        (EventResult::Ignored, all_messages)
    }
}

impl<M: Clone> Widget for Container<M> {
    type Message = M;

    fn render(&self, chunk: &mut render::chunk::Chunk) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        // Convert TUI style to render style
        let render_style = self.style.to_render_style();

        // Calculate area inside border (if any)
        let border_area = if self.style.border.is_some() {
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

        // Apply container background if specified (only inside border)
        if self.style.bg_color.is_some() {
            render_background(chunk, border_area, render_style);
        }

        // Draw border after background (so it's on top)
        if let Some(border) = self.style.border {
            render_border(chunk, area, border, render_style);
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
        let child_areas = bridge.compute_layout(&self.children, inner, self.direction, self.gap);

        // Cache layout info for mouse event handling
        *self.cached_child_areas.write().unwrap() = child_areas.clone();

        // Render each child in its allocated area using sequential sub-chunk creation
        for (child, child_area) in self.children.iter().zip(child_areas) {
            // Create a sub-chunk for this child
            if let Ok(mut child_chunk) = chunk.from_area(child_area) {
                child.render(&mut child_chunk);
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
        let border_size = if self.style.border.is_some() { 2 } else { 0 };

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
                    max_height: None,
                    flex: Some(1.0),
                }
            }
        }
    }
}

impl<M> Default for Container<M> {
    fn default() -> Self {
        Self::new()
    }
}
