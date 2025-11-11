//! Label widget - text display

use super::*;
use crate::style::{Color, Style};

/// Label widget for displaying text
#[derive(Debug, Clone)]
pub struct Label<M = ()> {
    content: String,
    style: Style,
    wrap: bool,
    _phantom: std::marker::PhantomData<M>,
}

impl<M> Label<M> {
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
            _phantom: std::marker::PhantomData,
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

    /// Set the foreground color (fluent API)
    pub fn fg(mut self, color: Color) -> Self {
        self.style = self.style.fg(color);
        self
    }

    /// Set the background color (fluent API)
    pub fn bg(mut self, color: Color) -> Self {
        self.style = self.style.bg(color);
        self
    }

    /// Make the text bold (fluent API)
    pub fn bold(mut self) -> Self {
        self.style = self.style.bold();
        self
    }

    /// Make the text italic (fluent API)
    pub fn italic(mut self) -> Self {
        self.style = self.style.italic();
        self
    }

    /// Make the text underlined (fluent API)
    pub fn underline(mut self) -> Self {
        self.style = self.style.underlined();
        self
    }
}

impl<M: Send + Sync> Widget<M> for Label<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        // Convert TUI style to render style
        let render_style = self.style.to_render_style();

        // Render text at relative coordinates (0, 0)
        let _ = chunk.set_string(0, 0, &self.content, render_style);
    }

    fn handle_event(&mut self, _event: &Event) -> EventResult<M> {
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
}

/// Create a new label widget (convenience function)
///
/// # Examples
/// ```
/// use tui::prelude::*;
///
/// let label = label("Hello, World!")
///     .fg(Color::Green)
///     .bold();
/// ```
pub fn label<M>(content: impl Into<String>) -> Label<M> {
    Label::new(content)
}
