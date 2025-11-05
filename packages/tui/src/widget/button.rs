//! Button widget - clickable action

use super::*;
use crate::event::handler::EventHandler;
use crate::event::KeyCode;
use crate::style::Style;

/// Button widget for triggering actions
#[derive(Clone)]
pub struct Button<M = ()> {
    label: String,
    style: Style,
    focused: bool,
    on_click: Option<EventHandler<M>>,
}

impl<M> std::fmt::Debug for Button<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Button")
            .field("label", &self.label)
            .field("style", &self.style)
            .field("focused", &self.focused)
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
            focused: false,
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
    /// - Keyboard: Press Enter or Space when the button is focused
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

    /// Set focus state (managed by FocusManager)
    pub(crate) fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Get the label text
    pub fn label(&self) -> &str {
        &self.label
    }
}

impl<M> Widget for Button<M> {
    type Message = M;
    fn render(&self, chunk: &mut render::chunk::Chunk, area: Rect) {
        if area.width < 4 || area.height == 0 {
            return;
        }

        // Render button with brackets: [ Label ]
        let prefix = if self.focused { "[>" } else { "[ " };
        let suffix = if self.focused { "<]" } else { " ]" };

        let button_text = format!("{}{}{}", prefix, self.label, suffix);

        // Convert TUI style to render style
        let render_style = crate::style::to_render_style(&self.style);

        let _ = chunk.set_string(area.x, area.y, &button_text, render_style);
    }

    fn handle_event(&mut self, event: &Event) -> EventResult<M> {
        match event {
            Event::Key(key_event) if self.focused => {
                // Handle Enter or Space key as activation when focused
                match key_event.code {
                    KeyCode::Enter | KeyCode::Space => {
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

    fn focusable(&self) -> bool {
        true
    }
}
