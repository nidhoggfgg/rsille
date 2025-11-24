//! Grid container widget for 2D grid layout

use render::area::Area;

use super::border_renderer::{render_background, render_border};
use super::grid_track::GridTrack;
use super::taffy_bridge::TaffyBridge;
use crate::event::{Event, EventResult};
use crate::focus::FocusPath;
use crate::layout::Constraints;
use crate::style::{BorderStyle, Padding, Style};
use crate::widget::{IntoWidget, Widget};
use std::sync::RwLock;
use taffy::style::{AlignItems, JustifyItems};

/// Grid container widget that arranges children in a 2D grid
///
/// # Examples
/// ```
/// use tui::prelude::*;
///
/// // Simple 3-column grid
/// let grid = grid()
///     .columns("1fr 1fr 1fr")
///     .rows("auto")
///     .gap(2)
///     .child(label("Cell 1"))
///     .child(label("Cell 2"))
///     .child(label("Cell 3"));
/// ```
pub struct Grid<M = ()> {
    children: Vec<Box<dyn Widget<M>>>,
    template_columns: Vec<GridTrack>,
    template_rows: Vec<GridTrack>,
    gap: u16,
    gap_row: Option<u16>,
    gap_column: Option<u16>,
    padding: Padding,
    border: Option<BorderStyle>,
    style: Style,
    align_items: Option<AlignItems>,
    justify_items: Option<JustifyItems>,
    /// Cached layout areas from last render (for mouse event handling)
    cached_child_areas: RwLock<Vec<Area>>,
}

impl<M> std::fmt::Debug for Grid<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Grid")
            .field("children", &self.children.len())
            .field("template_columns", &self.template_columns)
            .field("template_rows", &self.template_rows)
            .field("gap", &self.gap)
            .field("padding", &self.padding)
            .field("border", &self.border)
            .field("style", &self.style)
            .finish()
    }
}

impl<M> Grid<M> {
    /// Create a new empty grid
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            template_columns: vec![GridTrack::Fr(1.0)], // Default: 1 column
            template_rows: vec![GridTrack::Auto],       // Default: auto rows
            gap: 0,
            gap_row: None,
            gap_column: None,
            padding: Padding::ZERO,
            border: None,
            style: Style::default(),
            align_items: None,
            justify_items: None,
            cached_child_areas: RwLock::new(Vec::new()),
        }
    }

    /// Set grid template columns from a string template
    ///
    /// # Examples
    /// ```
    /// use tui::prelude::*;
    ///
    /// let grid = grid()
    ///     .columns("1fr 2fr 1fr"); // 3 columns with 1:2:1 ratio
    /// ```
    pub fn columns(mut self, template: &str) -> Self {
        self.template_columns = GridTrack::parse_template(template);
        self
    }

    /// Set grid template rows from a string template
    ///
    /// # Examples
    /// ```
    /// use tui::prelude::*;
    ///
    /// let grid = grid()
    ///     .rows("auto 1fr auto"); // Header, content, footer
    /// ```
    pub fn rows(mut self, template: &str) -> Self {
        self.template_rows = GridTrack::parse_template(template);
        self
    }

    /// Set grid template columns from a vector of GridTrack
    ///
    /// # Examples
    /// ```
    /// use tui::prelude::*;
    /// use tui::layout::GridTrack;
    ///
    /// let grid = grid()
    ///     .template_columns(vec![
    ///         GridTrack::Fixed(20),
    ///         GridTrack::Fr(1.0),
    ///         GridTrack::Auto,
    ///     ]);
    /// ```
    pub fn template_columns(mut self, tracks: Vec<GridTrack>) -> Self {
        self.template_columns = tracks;
        self
    }

    /// Set grid template rows from a vector of GridTrack
    ///
    /// # Examples
    /// ```
    /// use tui::prelude::*;
    /// use tui::layout::GridTrack;
    ///
    /// let grid = grid()
    ///     .template_rows(vec![
    ///         GridTrack::Auto,
    ///         GridTrack::Fr(1.0),
    ///     ]);
    /// ```
    pub fn template_rows(mut self, tracks: Vec<GridTrack>) -> Self {
        self.template_rows = tracks;
        self
    }

    /// Set the gap between grid cells (both row and column)
    ///
    /// # Examples
    /// ```
    /// use tui::prelude::*;
    ///
    /// let grid = grid().gap(2); // 2 cells gap
    /// ```
    pub fn gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    /// Set the row gap separately
    pub fn gap_row(mut self, gap: u16) -> Self {
        self.gap_row = Some(gap);
        self
    }

    /// Set the column gap separately
    pub fn gap_column(mut self, gap: u16) -> Self {
        self.gap_column = Some(gap);
        self
    }

    /// Set the inner padding
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    /// Set the grid style
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set the grid border
    pub fn border(mut self, border: BorderStyle) -> Self {
        self.border = Some(border);
        self
    }

    /// Set the alignment of items along the block axis
    pub fn align_items(mut self, align: AlignItems) -> Self {
        self.align_items = Some(align);
        self
    }

    /// Set the alignment of items along the inline axis
    pub fn justify_items(mut self, justify: JustifyItems) -> Self {
        self.justify_items = Some(justify);
        self
    }

    /// Conditionally modify the grid
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

    /// Get reference to children
    pub fn child_widgets(&self) -> &[Box<dyn Widget<M>>] {
        &self.children
    }

    /// Get the number of children
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Check if grid is empty
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
}

// Methods that require Send + Sync bounds
impl<M: Send + Sync> Grid<M> {
    /// Add a single child widget using fluent API
    ///
    /// # Examples
    /// ```
    /// use tui::prelude::*;
    ///
    /// let grid = grid()
    ///     .child(label("Cell 1"))
    ///     .child(label("Cell 2"));
    /// ```
    pub fn child(mut self, widget: impl IntoWidget<M>) -> Self {
        self.children.push(widget.into_widget());
        self
    }

    /// Add multiple child widgets using fluent API
    ///
    /// # Examples
    /// ```
    /// use tui::prelude::*;
    ///
    /// let items = vec!["Item 1", "Item 2", "Item 3"];
    /// let grid = grid()
    ///     .columns("1fr 1fr 1fr")
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

impl<M: Clone> Grid<M> {
    /// Build focus chain by recursively collecting focusable widgets
    pub fn build_focus_chain(&self, current_path: &mut Vec<usize>, chain: &mut Vec<FocusPath>) {
        // Grid itself is not focusable
        // Recursively traverse children
        for (idx, child) in self.children.iter().enumerate() {
            current_path.push(idx);

            // If child is focusable, add to chain
            if child.focusable() {
                chain.push(current_path.clone());
            }

            // Recursively build focus chain for nested containers
            child.build_focus_chain_recursive(current_path, chain);

            current_path.pop();
        }
    }

    /// Update focus state for all children based on focus path
    pub fn update_focus_states(&mut self, current_path: &[usize], focus_path: Option<&FocusPath>) {
        for (idx, child) in self.children.iter_mut().enumerate() {
            let mut child_path = current_path.to_vec();
            child_path.push(idx);

            let is_focused = focus_path.map_or(false, |fp| fp == &child_path);
            child.set_focused(is_focused);

            // Recursively update focus states for nested containers
            child.update_focus_states_recursive(&child_path, focus_path);
        }
    }

    /// Handle event and collect any generated messages
    pub fn handle_event_with_messages(&mut self, event: &Event) -> (EventResult<M>, Vec<M>)
    where
        M: Clone,
    {
        self.handle_event_with_focus(event, &[], None)
    }

    /// Handle event with focus information
    pub fn handle_event_with_focus(
        &mut self,
        event: &Event,
        current_path: &[usize],
        focus_path: Option<&FocusPath>,
    ) -> (EventResult<M>, Vec<M>)
    where
        M: Clone,
    {
        let mut all_messages = Vec::new();

        // For mouse events, use spatial routing
        if let Event::Mouse(mouse_event) = event {
            let cached_areas = self.cached_child_areas.read().unwrap();
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

        (EventResult::Ignored, all_messages)
    }
}

impl<M: Clone> Widget<M> for Grid<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        // Convert TUI style to render style
        let render_style = self.style.to_render_style();

        // Render background and border
        let mut content_area = area;

        // Render background
        if self.style.bg_color.is_some() {
            render_background(chunk, render_style);
        }

        // Render border if specified
        if let Some(ref border) = self.border {
            render_border(chunk, *border, render_style);
            // Adjust content area to account for border
            content_area = Area::new(
                (area.x() + 1, area.y() + 1).into(),
                (
                    area.width().saturating_sub(2),
                    area.height().saturating_sub(2),
                )
                    .into(),
            );
        }

        // Apply padding
        let padded_area = Area::new(
            (
                content_area.x() + self.padding.left,
                content_area.y() + self.padding.top,
            )
                .into(),
            (
                content_area
                    .width()
                    .saturating_sub(self.padding.left + self.padding.right),
                content_area
                    .height()
                    .saturating_sub(self.padding.top + self.padding.bottom),
            )
                .into(),
        );

        if padded_area.width() == 0 || padded_area.height() == 0 {
            return;
        }

        // Compute grid layout
        let mut layout = TaffyBridge::new();
        let gap_row = self.gap_row.unwrap_or(self.gap);
        let gap_column = self.gap_column.unwrap_or(self.gap);

        let child_areas = layout.compute_grid_layout(
            &self.children,
            padded_area,
            &self.template_columns,
            &self.template_rows,
            gap_row,
            gap_column,
            self.align_items,
            self.justify_items,
        );

        // Cache child areas for event handling
        *self.cached_child_areas.write().unwrap() = child_areas.clone();

        // Render children
        for (child, child_area) in self.children.iter().zip(child_areas.iter()) {
            if let Ok(mut child_chunk) = chunk.from_area(*child_area) {
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
        // Grid can be any size
        Constraints {
            min_width: 0,
            max_width: None,
            min_height: 0,
            max_height: None,
            flex: Some(1.0),
        }
    }

    fn build_focus_chain_recursive(
        &self,
        current_path: &mut Vec<usize>,
        chain: &mut Vec<FocusPath>,
    ) {
        // For nested grids, traverse children without adding self again
        for (idx, child) in self.children.iter().enumerate() {
            current_path.push(idx);

            // If child is focusable, add to chain
            if child.focusable() {
                chain.push(current_path.clone());
            }

            // Recursively build focus chain for nested containers
            child.build_focus_chain_recursive(current_path, chain);

            current_path.pop();
        }
    }

    fn update_focus_states_recursive(
        &mut self,
        current_path: &[usize],
        focus_path: Option<&FocusPath>,
    ) {
        // For nested grids, traverse children and update their focus states
        for (idx, child) in self.children.iter_mut().enumerate() {
            let mut child_path = current_path.to_vec();
            child_path.push(idx);

            let is_focused = focus_path.map_or(false, |fp| fp == &child_path);
            child.set_focused(is_focused);

            // Recursively update focus states for nested containers
            child.update_focus_states_recursive(&child_path, focus_path);
        }
    }
}

impl<M> Default for Grid<M> {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a new grid container (convenience function)
///
/// # Examples
/// ```
/// use tui::prelude::*;
///
/// let my_grid = grid()
///     .columns("1fr 1fr")
///     .rows("auto auto")
///     .gap(1)
///     .child(label("Top Left"))
///     .child(label("Top Right"))
///     .child(label("Bottom Left"))
///     .child(label("Bottom Right"));
/// ```
pub fn grid<M>() -> Grid<M> {
    Grid::new()
}
