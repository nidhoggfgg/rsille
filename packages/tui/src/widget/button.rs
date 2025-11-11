//! Button widget - clickable action

use super::*;
use crate::event::handler::EventHandler;
use crate::event::KeyCode;
use crate::style::{Color, Style};

/// Button widget for triggering actions
#[derive(Clone)]
pub struct Button<M = ()> {
    label: String,
    style: Style,
    on_click: Option<EventHandler<M>>,
}

impl<M> std::fmt::Debug for Button<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Button")
            .field("label", &self.label)
            .field("style", &self.style)
            .field("on_click", &self.on_click.is_some())
            .finish()
    }
}

impl<M> Button<M> {
    /// Create a new button with the specified label
    ///
    /// # Examples
    /// ```
    /// use tui::widget::Button;
    ///
    /// let button: Button<()> = Button::new("Click me");
    /// ```
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            style: Style::default(),
            on_click: None,
        }
    }

    /// Set the button style
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Attach a click handler that emits a message when activated
    ///
    /// The button can be activated by:
    /// - Keyboard: Press Enter or Space
    /// - Mouse: Click the button with the left mouse button
    ///
    /// # Examples
    /// ```
    /// use tui::widget::Button;
    ///
    /// #[derive(Clone)]
    /// enum Message { Increment }
    ///
    /// let button = Button::new("+").on_click(|| Message::Increment);
    /// ```
    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn() -> M + Send + Sync + 'static,
    {
        self.on_click = Some(std::sync::Arc::new(handler));
        self
    }

    /// Get the label text
    pub fn label(&self) -> &str {
        &self.label
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
}

impl<M: Send + Sync> Widget<M> for Button<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk) {
        let area = chunk.area();
        if area.width() < 4 || area.height() == 0 {
            return;
        }

        // Render button with brackets: [ Label ]
        let button_text = format!("[ {} ]", self.label);

        // Convert TUI style to render style
        let render_style = self.style.to_render_style();

        let _ = chunk.set_string(0, 0, &button_text, render_style);
    }

    fn handle_event(&mut self, event: &Event) -> EventResult<M> {
        match event {
            Event::Key(key_event) => {
                // Handle Enter or Space key as activation
                match key_event.code {
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        // Trigger click handler and return the message
                        if let Some(ref handler) = self.on_click {
                            let message = handler();
                            return EventResult::consumed_with(message);
                        }
                    }
                    _ => {}
                }
            }
            Event::Mouse(mouse_event) => {
                // Handle mouse clicks (left button)
                if let crate::event::MouseEventKind::Up(crate::event::MouseButton::Left) =
                    mouse_event.kind
                {
                    // Trigger click handler and return the message
                    if let Some(ref handler) = self.on_click {
                        let message = handler();
                        return EventResult::consumed_with(message);
                    }
                }
            }
            _ => {}
        }

        EventResult::Ignored
    }

    fn constraints(&self) -> Constraints {
        use unicode_width::UnicodeWidthStr;

        // Button width = label width + 4 (for "[ " and " ]")
        let width = self.label.width() as u16 + 4;
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

/// Create a new button widget (convenience function)
///
/// # Examples
/// ```
/// use tui::prelude::*;
///
/// let button = button("Click me")
///     .on_click(|| Message::Click)
///     .fg(Color::Blue);
/// ```
pub fn button<M>(label: impl Into<String>) -> Button<M> {
    Button::new(label)
}
