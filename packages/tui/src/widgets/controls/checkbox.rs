//! Checkbox widget.

use render::text::TextWidth;

use crate::event::{Event, KeyCode, MouseButton, MouseEventKind};
use crate::focus::FocusConfig;
use crate::layout::Constraints;
use crate::style::Style;
use crate::widget::{EventCtx, EventPhase, RenderCtx, Widget};

/// Focusable boolean checkbox.
pub struct Checkbox<M = ()> {
    label: String,
    checked: bool,
    disabled: bool,
    custom_style: Option<Style>,
    custom_focus_style: Option<Style>,
    on_change: Option<Box<dyn Fn(bool) -> M>>,
    widget_key: Option<String>,
}

impl<M> std::fmt::Debug for Checkbox<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Checkbox")
            .field("label", &self.label)
            .field("checked", &self.checked)
            .field("disabled", &self.disabled)
            .field("on_change", &self.on_change.is_some())
            .finish()
    }
}

impl<M> Checkbox<M> {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            checked: false,
            disabled: false,
            custom_style: None,
            custom_focus_style: None,
            on_change: None,
            widget_key: None,
        }
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }

    pub fn focus_style(mut self, style: Style) -> Self {
        self.custom_focus_style = Some(style);
        self
    }

    pub fn on_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(bool) -> M + 'static,
    {
        self.on_change = Some(Box::new(handler));
        self
    }
}

impl<M> Widget<M> for Checkbox<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        let theme = ctx.theme();
        let base_style = if self.disabled {
            theme.styles.interactive_disabled
        } else if ctx.is_focused() {
            theme.styles.interactive_focused
        } else {
            theme.styles.interactive
        };
        let style = if ctx.is_focused() {
            self.custom_focus_style
                .map(|s| s.merge(base_style))
                .or_else(|| self.custom_style.map(|s| s.merge(base_style)))
                .unwrap_or(base_style)
        } else {
            self.custom_style
                .map(|s| s.merge(base_style))
                .unwrap_or(base_style)
        }
        .to_render_style();

        let mark = if self.checked { "x" } else { " " };
        let text = format!("[{mark}] {}", self.label);
        let display = truncate_to_width(&text, area.width() as usize, chunk.text_metrics());
        let _ = chunk.set_string(0, 0, &display, style);
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if ctx.phase() != EventPhase::Target || self.disabled {
            return;
        }

        let activated = match event {
            Event::Key(key_event) => matches!(key_event.code, KeyCode::Enter | KeyCode::Char(' ')),
            Event::Mouse(mouse_event) => {
                matches!(mouse_event.kind, MouseEventKind::Down(MouseButton::Left))
            }
            _ => false,
        };

        if activated {
            ctx.set_handled();
            if let Some(handler) = self.on_change.as_ref() {
                ctx.emit(handler(!self.checked));
            }
        }
    }

    fn constraints(&self) -> Constraints {
        Constraints {
            min_width: self.label.width() as u16 + 4,
            max_width: None,
            min_height: 1,
            max_height: Some(1),
            flex: None,
        }
    }

    fn focus_config(&self) -> FocusConfig {
        if self.disabled {
            FocusConfig::None
        } else {
            FocusConfig::Leaf
        }
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

fn truncate_to_width(text: &str, max_width: usize, metrics: render::text::TextMetrics) -> String {
    metrics.clip(text, 0, max_width)
}

/// Create a checkbox widget.
pub fn checkbox<M>(label: impl Into<String>) -> Checkbox<M> {
    Checkbox::new(label)
}
