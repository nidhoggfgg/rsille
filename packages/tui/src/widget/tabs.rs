//! Tabs widget - multi-panel content switching component
//!
//! A modern tabs component with support for multiple visual variants,
//! keyboard navigation, and flexible layouts.

use super::*;
use crate::event::{Event, EventResult, KeyCode};
use crate::layout::Constraints;
use crate::style::{Style, ThemeManager};
use crate::widget::common::{StatefulWidgetBuilder, WidgetState};
use render::chunk::Chunk;
use std::sync::Arc;
use unicode_width::UnicodeWidthStr;

/// Tab orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TabOrientation {
    /// Horizontal tab bar (tabs arranged left to right)
    #[default]
    Horizontal,
    /// Vertical tab bar (tabs arranged top to bottom)
    Vertical,
}

/// Tab visual variants
///
/// Different visual styles inspired by modern UI frameworks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TabVariant {
    /// Line style with bottom/right indicator (default)
    /// Active tab has an underline/side-line
    #[default]
    Line,
    /// Enclosed style with borders
    /// Tabs have borders and connect with content area
    Enclosed,
    /// Solid style with filled background
    /// Active tab has solid background color
    Solid,
    /// Pills style with rounded appearance
    /// Active tab has rounded pill-like background
    Pills,
    /// Minimal style with subtle changes
    /// Only text color/weight changes on activation
    Minimal,
}

/// Badge indicator for a tab
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabBadge {
    /// Numeric badge (e.g., notification count)
    Count(u32),
    /// Dot indicator (simple presence indicator)
    Dot,
    /// Custom text badge
    Text(String),
}

impl TabBadge {
    /// Get the display width of the badge
    fn width(&self) -> u16 {
        match self {
            TabBadge::Count(n) => {
                // " (99)" format
                let num_str = n.to_string();
                3 + num_str.len() as u16 // " (" + digits + ")"
            }
            TabBadge::Dot => 2, // " •"
            TabBadge::Text(s) => {
                // " [text]" format
                3 + s.width() as u16
            }
        }
    }

    /// Render the badge to a string
    fn to_string(&self) -> String {
        match self {
            TabBadge::Count(n) => format!(" ({})", n),
            TabBadge::Dot => " •".to_string(),
            TabBadge::Text(s) => format!(" [{}]", s),
        }
    }
}

/// A single tab item with label and content
pub struct TabItem<M = ()> {
    /// Tab label text
    pub label: String,
    /// Tab content widget
    pub content: Box<dyn Widget<M>>,
    /// Whether this tab is disabled
    pub disabled: bool,
    /// Optional badge indicator
    pub badge: Option<TabBadge>,
}

impl<M> std::fmt::Debug for TabItem<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TabItem")
            .field("label", &self.label)
            .field("disabled", &self.disabled)
            .field("badge", &self.badge)
            .finish()
    }
}

impl<M> TabItem<M> {
    /// Create a new tab item
    pub fn new(label: impl Into<String>, content: impl Widget<M> + 'static) -> Self {
        Self {
            label: label.into(),
            content: Box::new(content),
            disabled: false,
            badge: None,
        }
    }

    /// Set the disabled state
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set a badge indicator
    pub fn badge(mut self, badge: TabBadge) -> Self {
        self.badge = Some(badge);
        self
    }

    /// Get the display width of this tab (label + badge)
    fn display_width(&self) -> u16 {
        let label_width = self.label.width() as u16;
        let badge_width = self.badge.as_ref().map(|b| b.width()).unwrap_or(0);
        label_width + badge_width
    }
}

/// Event emitted when tab changes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabChangeEvent {
    /// Index of the newly selected tab
    pub index: usize,
}

/// Tabs widget for multi-panel interfaces
///
/// Supports multiple visual variants, keyboard navigation, and flexible layouts.
///
/// # Examples
/// ```ignore
/// use tui::widget::{Tabs, TabItem, TabVariant};
///
/// let tabs = Tabs::new()
///     .variant(TabVariant::Line)
///     .tab(TabItem::new("Settings", settings_content))
///     .tab(TabItem::new("Profile", profile_content))
///     .tab(TabItem::new("Notifications", notifications_content)
///         .badge(TabBadge::Count(3)))
///     .on_change(|event| Message::TabChanged(event.index));
/// ```
pub struct Tabs<M = ()> {
    /// Tab items (label + content pairs)
    tabs: Vec<TabItem<M>>,
    /// Currently active tab index
    active_index: usize,
    /// Visual variant
    variant: TabVariant,
    /// Tab bar orientation
    orientation: TabOrientation,
    /// Widget state
    state: WidgetState,
    /// Tab change handler
    on_change: Option<Arc<dyn Fn(TabChangeEvent) -> M + Send + Sync>>,
}

impl<M> std::fmt::Debug for Tabs<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tabs")
            .field("tabs", &self.tabs)
            .field("active_index", &self.active_index)
            .field("variant", &self.variant)
            .field("orientation", &self.orientation)
            .field("on_change", &self.on_change.is_some())
            .finish()
    }
}

impl<M> Tabs<M> {
    /// Create a new empty tabs widget
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_index: 0,
            variant: TabVariant::default(),
            orientation: TabOrientation::default(),
            state: WidgetState::new(),
            on_change: None,
        }
    }

    /// Add a tab item
    pub fn tab(mut self, item: TabItem<M>) -> Self {
        self.tabs.push(item);
        self
    }

    /// Set the visual variant
    pub fn variant(mut self, variant: TabVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the tab bar orientation
    pub fn orientation(mut self, orientation: TabOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Set the initially active tab
    pub fn active(mut self, index: usize) -> Self {
        // Always set the active_index, validation will happen during render
        // This allows calling .active() before adding tabs
        self.active_index = index;
        self
    }

    /// Set the tab change handler
    pub fn on_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(TabChangeEvent) -> M + Send + Sync + 'static,
    {
        self.on_change = Some(Arc::new(handler));
        self
    }

    /// Move to the next non-disabled tab
    fn next_tab(&self) -> Vec<M> {
        if self.tabs.is_empty() {
            return vec![];
        }

        let start = self.active_index;
        let mut next = (start + 1) % self.tabs.len();

        // Find next non-disabled tab
        while next != start {
            if !self.tabs[next].disabled {
                // Emit event for the next tab
                if let Some(ref handler) = self.on_change {
                    let event = TabChangeEvent { index: next };
                    return vec![handler(event)];
                }
                return vec![];
            }
            next = (next + 1) % self.tabs.len();
        }

        vec![]
    }

    /// Move to the previous non-disabled tab
    fn previous_tab(&self) -> Vec<M> {
        if self.tabs.is_empty() {
            return vec![];
        }

        let start = self.active_index;
        let mut prev = if start == 0 {
            self.tabs.len() - 1
        } else {
            start - 1
        };

        // Find previous non-disabled tab
        while prev != start {
            if !self.tabs[prev].disabled {
                // Emit event for the previous tab
                if let Some(ref handler) = self.on_change {
                    let event = TabChangeEvent { index: prev };
                    return vec![handler(event)];
                }
                return vec![];
            }
            prev = if prev == 0 {
                self.tabs.len() - 1
            } else {
                prev - 1
            };
        }

        vec![]
    }

    /// Render the tab bar (horizontal)
    fn render_horizontal_tabs(&self, chunk: &mut Chunk) {
        let area = chunk.area();
        if area.width() == 0 || area.height() < 1 {
            return;
        }

        // Get theme colors
        ThemeManager::global().with_theme(|theme| {
            let mut x = 0u16;
            let y = 0u16;

            for (idx, tab) in self.tabs.iter().enumerate() {
                let is_active = idx == self.active_index;
                let is_focused = self.state.is_focused();

                // Calculate tab label with badge
                let label_text = if let Some(badge) = &tab.badge {
                    format!("{}{}", tab.label, badge.to_string())
                } else {
                    tab.label.clone()
                };

                let tab_width = tab.display_width() + 4; // Add padding

                if x + tab_width > area.width() {
                    break; // No more space
                }

                // Determine style based on state and variant
                let (fg, bg, bold) = if tab.disabled {
                    (theme.colors.text_muted, theme.colors.background, false)
                } else if is_active {
                    match self.variant {
                        TabVariant::Line | TabVariant::Minimal => {
                            if is_focused {
                                (theme.colors.primary, theme.colors.background, true)
                            } else {
                                (theme.colors.primary, theme.colors.background, false)
                            }
                        }
                        TabVariant::Solid | TabVariant::Pills | TabVariant::Enclosed => {
                            (theme.colors.text, theme.colors.primary, true)
                        }
                    }
                } else {
                    (theme.colors.text_muted, theme.colors.background, false)
                };

                let mut style = Style::default().fg(fg).bg(bg);
                if bold {
                    style = style.bold();
                }

                // Render tab content
                let render_style = style.to_render_style();
                let padded_label = format!(" {} ", label_text);

                // Render based on variant
                match self.variant {
                    TabVariant::Line => {
                        // Render label
                        let _ = chunk.set_string(x, y, &padded_label, render_style);

                        // Render bottom line for active tab
                        if is_active && area.height() > 1 {
                            let line_style =
                                Style::default().fg(theme.colors.primary).to_render_style();
                            let label_width = padded_label.width() as u16;
                            for i in 0..label_width {
                                let _ = chunk.set_string(x + i, y + 1, "▔", line_style);
                            }
                        }
                    }
                    TabVariant::Enclosed => {
                        // Render with borders for active tab, plain text for inactive
                        if is_active && area.height() > 1 {
                            // Top border
                            let _ = chunk.set_string(x, y, "┌", render_style);
                            for i in 1..tab_width - 1 {
                                let _ = chunk.set_string(x + i, y, "─", render_style);
                            }
                            let _ = chunk.set_string(x + tab_width - 1, y, "┐", render_style);

                            // Label with side borders
                            let _ = chunk.set_string(x, y + 1, "│", render_style);
                            let _ = chunk.set_string(x + 2, y + 1, &label_text, render_style);
                            let _ = chunk.set_string(x + tab_width - 1, y + 1, "│", render_style);
                        } else {
                            // Inactive tabs: just render the padded label on the first line
                            let _ = chunk.set_string(x, y, &padded_label, render_style);
                        }
                    }
                    TabVariant::Solid | TabVariant::Pills => {
                        // Render with background fill
                        // First fill the background
                        for i in 0..tab_width {
                            let _ = chunk.set_string(x + i, y, " ", render_style);
                        }
                        // Then render the padded label on top (centered with 2-char padding on each side)
                        let _ = chunk.set_string(x + 2, y, &label_text, render_style);
                    }
                    TabVariant::Minimal => {
                        // Just render the text
                        let _ = chunk.set_string(x, y, &padded_label, render_style);
                    }
                }

                x += tab_width + 1; // Add gap between tabs
            }
        });
    }

    /// Render the tab bar (vertical)
    fn render_vertical_tabs(&self, chunk: &mut Chunk, max_height: u16) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        // Calculate max label width
        let max_label_width = self
            .tabs
            .iter()
            .map(|t| t.display_width())
            .max()
            .unwrap_or(10)
            + 4;

        ThemeManager::global().with_theme(|theme| {
            let mut y = 0u16;

            for (idx, tab) in self.tabs.iter().enumerate() {
                if y >= max_height {
                    break;
                }

                let is_active = idx == self.active_index;
                let is_focused = self.state.is_focused();

                let label_text = if let Some(badge) = &tab.badge {
                    format!("{}{}", tab.label, badge.to_string())
                } else {
                    tab.label.clone()
                };

                let (fg, bg, bold) = if tab.disabled {
                    (theme.colors.text_muted, theme.colors.background, false)
                } else if is_active {
                    match self.variant {
                        TabVariant::Line | TabVariant::Minimal => {
                            if is_focused {
                                (theme.colors.primary, theme.colors.background, true)
                            } else {
                                (theme.colors.primary, theme.colors.background, false)
                            }
                        }
                        TabVariant::Solid | TabVariant::Pills | TabVariant::Enclosed => {
                            (theme.colors.text, theme.colors.primary, true)
                        }
                    }
                } else {
                    (theme.colors.text_muted, theme.colors.background, false)
                };

                let mut style = Style::default().fg(fg).bg(bg);
                if bold {
                    style = style.bold();
                }
                let render_style = style.to_render_style();

                let padded_label = format!(" {} ", label_text);

                match self.variant {
                    TabVariant::Line => {
                        let _ = chunk.set_string(0, y, &padded_label, render_style);
                        if is_active && max_label_width < area.width() {
                            let line_style =
                                Style::default().fg(theme.colors.primary).to_render_style();
                            let _ = chunk.set_string(max_label_width - 1, y, "▏", line_style);
                        }
                    }
                    TabVariant::Solid | TabVariant::Pills | TabVariant::Enclosed => {
                        // Fill background
                        for i in 0..max_label_width {
                            let _ = chunk.set_string(i, y, " ", render_style);
                        }
                        // Render label on top
                        let _ = chunk.set_string(2, y, &label_text, render_style);
                    }
                    TabVariant::Minimal => {
                        let _ = chunk.set_string(0, y, &padded_label, render_style);
                    }
                }

                y += 1;
            }
        });
    }

    /// Get the tab bar height (for horizontal orientation)
    fn tab_bar_height(&self) -> u16 {
        // Always reserve space based on variant, even if not currently needed
        // This prevents layout jumps when switching tabs or variants
        match self.variant {
            TabVariant::Enclosed => 2, // Top border + text line
            TabVariant::Line => 2,     // Text line + underline
            _ => 1,                    // Just text line
        }
    }

    /// Get the tab bar width (for vertical orientation)
    fn tab_bar_width(&self) -> u16 {
        self.tabs
            .iter()
            .map(|t| t.display_width())
            .max()
            .unwrap_or(10)
            + 4
    }
}

impl<M: Send + Sync> Widget<M> for Tabs<M> {
    fn render(&self, chunk: &mut Chunk) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 || self.tabs.is_empty() {
            return;
        }

        match self.orientation {
            TabOrientation::Horizontal => {
                let tab_bar_height = self.tab_bar_height();

                // Render tab bar at the top
                if let Ok(mut tab_chunk) =
                    chunk.shrink(0, area.height().saturating_sub(tab_bar_height), 0, 0)
                {
                    self.render_horizontal_tabs(&mut tab_chunk);
                }

                // Render active tab content below
                if area.height() > tab_bar_height {
                    if let Some(active_tab) = self.tabs.get(self.active_index) {
                        if let Ok(mut content_chunk) = chunk.shrink(tab_bar_height, 0, 0, 0) {
                            active_tab.content.render(&mut content_chunk);
                        }
                    }
                }
            }
            TabOrientation::Vertical => {
                let tab_bar_width = self.tab_bar_width();

                // Render tab bar on the left
                if let Ok(mut tab_chunk) =
                    chunk.shrink(0, 0, 0, area.width().saturating_sub(tab_bar_width))
                {
                    self.render_vertical_tabs(&mut tab_chunk, area.height());
                }

                // Render active tab content on the right
                if area.width() > tab_bar_width {
                    if let Some(active_tab) = self.tabs.get(self.active_index) {
                        if let Ok(mut content_chunk) = chunk.shrink(0, 0, tab_bar_width, 0) {
                            active_tab.content.render(&mut content_chunk);
                        }
                    }
                }
            }
        }
    }

    fn handle_event(&mut self, event: &Event) -> EventResult<M> {
        // Disabled tabs don't handle events
        if self.state.is_disabled() {
            return EventResult::Ignored;
        }

        match event {
            Event::Key(key_event) => {
                let messages = match self.orientation {
                    TabOrientation::Horizontal => match key_event.code {
                        KeyCode::Left => self.previous_tab(),
                        KeyCode::Right => self.next_tab(),
                        _ => {
                            // Forward other events to active tab content
                            if let Some(active_tab) = self.tabs.get_mut(self.active_index) {
                                return active_tab.content.handle_event(event);
                            }
                            return EventResult::Ignored;
                        }
                    },
                    TabOrientation::Vertical => match key_event.code {
                        KeyCode::Up => self.previous_tab(),
                        KeyCode::Down => self.next_tab(),
                        _ => {
                            // Forward other events to active tab content
                            if let Some(active_tab) = self.tabs.get_mut(self.active_index) {
                                return active_tab.content.handle_event(event);
                            }
                            return EventResult::Ignored;
                        }
                    },
                };

                // Always consume left/right/up/down keys, even if no messages
                // This prevents the keys from bubbling up to parent widgets
                if !messages.is_empty() {
                    EventResult::Consumed(messages)
                } else {
                    // Still consumed the event, just no messages to send
                    EventResult::Consumed(vec![])
                }
            }
            _ => {
                // Forward all other events to active tab content
                if let Some(active_tab) = self.tabs.get_mut(self.active_index) {
                    active_tab.content.handle_event(event)
                } else {
                    EventResult::Ignored
                }
            }
        }
    }

    fn constraints(&self) -> Constraints {
        // Calculate minimum dimensions needed for the tabs component
        match self.orientation {
            TabOrientation::Horizontal => {
                // Tab bar height + content's minimum height
                let tab_bar_height = self.tab_bar_height();

                // Calculate minimum width: max of tab bar width and content width
                let tab_bar_width = self
                    .tabs
                    .iter()
                    .map(|t| t.display_width() + 4) // label + padding
                    .sum::<u16>()
                    + self.tabs.len().saturating_sub(1) as u16; // gaps between tabs

                let (content_min_width, content_min_height) =
                    if let Some(active_tab) = self.tabs.get(self.active_index) {
                        let c = active_tab.content.constraints();
                        (c.min_width, c.min_height)
                    } else {
                        (10, 1)
                    };

                Constraints {
                    min_width: tab_bar_width.max(content_min_width),
                    max_width: None,
                    min_height: tab_bar_height + content_min_height,
                    max_height: None,
                    flex: Some(1.0),
                }
            }
            TabOrientation::Vertical => {
                // Tab bar width + content's minimum width
                let tab_bar_width = self.tab_bar_width();

                // Calculate minimum height: max of tab bar height and content height
                let tab_bar_height = self.tabs.len().max(3) as u16;

                let (content_min_width, content_min_height) =
                    if let Some(active_tab) = self.tabs.get(self.active_index) {
                        let c = active_tab.content.constraints();
                        (c.min_width, c.min_height)
                    } else {
                        (10, 1)
                    };

                Constraints {
                    min_width: tab_bar_width + content_min_width,
                    max_width: None,
                    min_height: tab_bar_height.max(content_min_height),
                    max_height: None,
                    flex: Some(1.0),
                }
            }
        }
    }

    fn focusable(&self) -> bool {
        self.state.is_focusable()
    }

    fn is_focused(&self) -> bool {
        self.state.is_focused()
    }

    fn set_focused(&mut self, focused: bool) {
        self.state.set_focused(focused);
    }

    fn build_focus_chain_recursive(
        &self,
        current_path: &mut Vec<usize>,
        chain: &mut Vec<crate::widget_id::WidgetId>,
        registry: &mut crate::focus::WidgetRegistry,
    ) {
        use smallvec::SmallVec;

        // Add self to focus chain if focusable
        if self.focusable() {
            let widget_key = self.widget_key();
            let widget_id = crate::widget_id::WidgetId::from_path_and_key(current_path, widget_key);
            chain.push(widget_id);
            registry.register(widget_id, SmallVec::from_slice(current_path));
        }

        // Build focus chain for active tab content
        if let Some(active_tab) = self.tabs.get(self.active_index) {
            current_path.push(self.active_index);
            active_tab
                .content
                .build_focus_chain_recursive(current_path, chain, registry);
            current_path.pop();
        }
    }

    fn update_focus_states_recursive(
        &mut self,
        current_path: &[usize],
        focus_id: Option<crate::widget_id::WidgetId>,
    ) {
        // Update focus states for active tab content
        if let Some(active_tab) = self.tabs.get_mut(self.active_index) {
            active_tab
                .content
                .update_focus_states_recursive(current_path, focus_id);
        }
    }
}

impl<M> Default for Tabs<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> StatefulWidgetBuilder for Tabs<M> {
    fn widget_state_mut(&mut self) -> &mut WidgetState {
        &mut self.state
    }
}

/// Create a new tabs widget (convenience function)
///
/// # Examples
/// ```ignore
/// use tui::widget::{tabs, TabItem, label};
///
/// let my_tabs = tabs()
///     .tab(TabItem::new("Home", label("Home content")))
///     .tab(TabItem::new("Settings", label("Settings content")));
/// ```
pub fn tabs<M>() -> Tabs<M> {
    Tabs::new()
}
