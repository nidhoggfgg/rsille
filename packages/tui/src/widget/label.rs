//! Label widget - text display

use super::*;
use crate::style::Style;

/// Label widget for displaying text
#[derive(Debug, Clone)]
pub struct Label {
    content: String,
    style: Style,
    wrap: bool,
}

impl Label {
    /// Create a new label with the specified text
    ///
    /// # Examples
    /// ```
    /// use tui::widget::Label;
    ///
    /// let label = Label::new("Hello, World!");
    /// ```
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            style: Style::default(),
            wrap: false,
        }
    }

    /// Set the label style
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Enable text wrapping
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Get the text content
    pub fn content(&self) -> &str {
        &self.content
    }
}

impl Widget for Label {
    fn render(&self, buf: &mut Buffer, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        // Render text at the widget's position
        buf.set_string(area.x, area.y, &self.content, self.style);
    }

    fn handle_event(&mut self, _event: &Event) -> EventResult {
        // Labels don't handle events
        EventResult::Ignored
    }

    fn constraints(&self) -> Constraints {
        // Measure text width using unicode-width
        use unicode_width::UnicodeWidthStr;
        let width = self.content.width() as u16;

        // Height is 1 line for now (wrapping not implemented yet)
        let height = 1;

        Constraints {
            min_width: width,
            max_width: Some(width),
            min_height: height,
            max_height: Some(height),
            flex: None,
        }
    }

    fn focusable(&self) -> bool {
        false
    }
}
