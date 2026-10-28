//! Button widget — focusable interactive button

use crate::tui::animation::AnimationCtx;
use crate::tui::event::{Event, KeyCode, MouseButton, MouseEventKind};
use crate::tui::focus::FocusConfig;
use crate::tui::layout::Constraints;
use crate::tui::style::{Style, Theme};
use crate::tui::widget::{EventCtx, EventPhase, RenderCtx, Widget};
use crate::tui::widgets::VariantWidget;

/// Button style variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Ghost,
    Link,
    Destructive,
}

/// Interactive button widget.
///
/// Focusable by default. Activated by Enter or Space when focused. Use
/// `.focusable(false)` for mouse-only buttons with separate keyboard shortcuts.
pub struct Button<M = ()> {
    label: String,
    variant: ButtonVariant,
    disabled: bool,
    focusable: bool,
    on_click: Option<Box<dyn Fn() -> M>>,
    widget_key: Option<String>,
}

impl<M> std::fmt::Debug for Button<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Button")
            .field("label", &self.label)
            .field("variant", &self.variant)
            .field("disabled", &self.disabled)
            .field("focusable", &self.focusable)
            .field("on_click", &self.on_click.is_some())
            .finish()
    }
}

impl<M> Button<M> {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: ButtonVariant::default(),
            disabled: false,
            focusable: true,
            on_click: None,
            widget_key: None,
        }
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set keyboard focus participation. A non-focusable button still accepts
    /// mouse clicks, but is skipped by Tab and cannot receive keyboard focus.
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }

    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn() -> M + 'static,
    {
        self.on_click = Some(Box::new(handler));
        self
    }

    fn compute_style(&self, theme: &Theme, is_focused: bool) -> Style {
        if self.disabled {
            return theme.styles.disabled;
        }
        match self.variant {
            ButtonVariant::Primary => {
                if is_focused {
                    theme.styles.primary_action_focused
                } else {
                    theme.styles.primary_action
                }
            }
            ButtonVariant::Secondary => {
                if is_focused {
                    theme.styles.secondary_action_focused
                } else {
                    theme.styles.secondary_action
                }
            }
            ButtonVariant::Ghost => {
                if is_focused {
                    theme.styles.interactive_focused
                } else {
                    theme.styles.text
                }
            }
            ButtonVariant::Link => {
                let base = if is_focused {
                    theme.styles.interactive_focused
                } else {
                    theme.styles.text
                };
                base.underlined()
            }
            ButtonVariant::Destructive => {
                if is_focused {
                    theme.styles.destructive_action_focused
                } else {
                    theme.styles.destructive_action
                }
            }
        }
    }
}

impl<M> VariantWidget for Button<M> {
    type Variant = ButtonVariant;

    fn variant(mut self, variant: Self::Variant) -> Self {
        self.variant = variant;
        self
    }
}

impl<M> Widget<M> for Button<M> {
    fn animate(&self, ctx: &mut AnimationCtx) -> bool {
        ctx.track_value(
            "button-hover",
            if !self.disabled && ctx.is_hovered() {
                1.0
            } else {
                0.0
            },
            ctx.animation_theme().fast,
        )
    }

    fn render(&self, chunk: &mut crate::render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        let is_focused = self.focusable && !self.disabled && ctx.is_focused();
        let theme = ctx.theme();
        let hover = if self.disabled {
            0.0
        } else {
            ctx.animation_value("button-hover")
                .unwrap_or(if ctx.is_hovered() { 1.0 } else { 0.0 })
        };
        let mut style = self.compute_style(theme, is_focused);
        if !self.disabled && !is_focused {
            let target = match self.variant {
                ButtonVariant::Primary => theme.styles.primary_action_hover,
                ButtonVariant::Secondary => theme.styles.secondary_action_hover,
                ButtonVariant::Destructive => theme.styles.destructive_action_hover,
                ButtonVariant::Ghost => theme.styles.interactive_focused,
                ButtonVariant::Link => theme.styles.interactive_focused.underlined(),
            };
            style = style.interpolate(target, hover);
        }
        let render_style = style.to_render_style();

        let width = area.width();
        let height = area.height();

        // Background fill for solid variants
        if matches!(
            self.variant,
            ButtonVariant::Primary | ButtonVariant::Secondary | ButtonVariant::Destructive
        ) {
            let _ = chunk.fill(0, 0, width, height, ' ', render_style);
        }

        // Border for Ghost variant
        if matches!(self.variant, ButtonVariant::Ghost) {
            let border_style = if is_focused {
                theme.styles.border_focused.to_render_style()
            } else {
                theme
                    .styles
                    .border
                    .interpolate(theme.styles.border_focused, hover)
                    .to_render_style()
            };
            use crate::tui::layout::border_renderer;
            use crate::tui::style::BorderStyle;
            border_renderer::render_border(chunk, BorderStyle::Single, border_style);
        }

        // Center label text
        let text_width = chunk.text_metrics().width(&self.label) as u16;
        let padding_offset = if matches!(self.variant, ButtonVariant::Ghost) {
            1
        } else {
            0
        };
        let available_width = width.saturating_sub(padding_offset * 2);
        let text_x = if available_width > text_width {
            padding_offset + (available_width - text_width) / 2
        } else {
            padding_offset
        };
        let text_y = height / 2;

        let _ = chunk.set_string(text_x, text_y, &self.label, render_style);
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if matches!(event, Event::Key(key) if key.kind != crate::tui::event::KeyEventKind::Press) {
            return;
        }
        if ctx.phase() != EventPhase::Target {
            return;
        }
        if self.disabled {
            return;
        }
        match event {
            Event::Key(key_event) if self.focusable => {
                if matches!(key_event.code, KeyCode::Enter | KeyCode::Char(' ')) {
                    if let Some(ref handler) = self.on_click {
                        ctx.emit(handler());
                    }
                }
            }
            Event::Mouse(mouse_event) => {
                if matches!(mouse_event.kind, MouseEventKind::Down(MouseButton::Left)) {
                    if let Some(ref handler) = self.on_click {
                        ctx.emit(handler());
                    } else {
                        ctx.set_handled();
                    }
                }
            }
            _ => {}
        }
    }

    fn constraints(&self) -> Constraints {
        use crate::render::text::TextWidth;
        let label_width = self.label.width() as u16;
        let (total_width, height) = match self.variant {
            ButtonVariant::Ghost => (label_width + 6, 3),
            _ => (label_width + 4, 1),
        };
        Constraints {
            min_width: total_width,
            max_width: Some(total_width),
            min_height: height,
            max_height: Some(height),
            flex: None,
        }
    }

    fn focus_config(&self) -> FocusConfig {
        if self.disabled || !self.focusable {
            FocusConfig::None
        } else {
            FocusConfig::Leaf
        }
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

/// Create a new button widget.
pub fn button<M>(label: impl Into<String>) -> Button<M> {
    Button::new(label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::area::Area;
    use crate::render::buffer::Buffer;
    use crate::render::chunk::Chunk;
    use crate::tui::animation::AnimationStore;
    use crate::tui::widget::{WidgetPath, WidgetStore};
    use std::cell::RefCell;
    use std::collections::HashMap;

    #[test]
    fn mouse_only_button_accepts_clicks_without_stealing_keyboard_focus() {
        use crate::tui::prelude::*;
        let mut app = App::new(Vec::new())
            .update(|clicks, message, _| clicks.push(message))
            .view(|_| {
                row()
                    .child(
                        button("Mouse")
                            .key("mouse")
                            .focusable(false)
                            .on_click(|| "mouse"),
                    )
                    .child(button("Keyboard").key("keyboard").on_click(|| "keyboard"))
            })
            .build()
            .size(40, 1);
        app.step(&[]).unwrap();
        assert_eq!(app.focused_key(), Some("keyboard"));
        app.step(&[Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 1,
            row: 0,
            modifiers: KeyModifiers::empty(),
        })])
        .unwrap();
        assert_eq!(app.state(), &["mouse"]);
        assert_eq!(app.focused_key(), Some("keyboard"));
        app.step(&[Event::Key(KeyEvent::new(
            KeyCode::Tab,
            KeyModifiers::empty(),
        ))])
        .unwrap();
        assert_eq!(app.focused_key(), Some("keyboard"));
        app.step(&[Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::empty(),
        ))])
        .unwrap();
        assert_eq!(app.state(), &["mouse", "keyboard"]);
    }

    #[test]
    fn focused_buttons_use_theme_styles_without_markers() {
        let theme = Theme::dark();
        for (variant, style) in [
            (ButtonVariant::Primary, theme.styles.primary_action_focused),
            (
                ButtonVariant::Secondary,
                theme.styles.secondary_action_focused,
            ),
            (
                ButtonVariant::Destructive,
                theme.styles.destructive_action_focused,
            ),
            (
                ButtonVariant::Link,
                theme.styles.interactive_focused.underlined(),
            ),
        ] {
            let button = button::<()>("OK").variant(variant);
            let buffer = render_button(&button, 6, 1, true);

            for (x, expected) in "  OK  ".chars().enumerate() {
                assert_eq!(cell_char(&buffer, x as u16, 0), Some(expected));
            }
            assert_eq!(buffer.content()[2].content.style, style.to_render_style());

            let unfocused = render_button(&button, 6, 1, false);
            assert_ne!(
                buffer.content()[2].content.style,
                unfocused.content()[2].content.style
            );

            let narrow = render_button(&button, 2, 1, true);
            assert_eq!(cell_char(&narrow, 0, 0), Some('O'));
            assert_eq!(cell_char(&narrow, 1, 0), Some('K'));
        }
    }

    #[test]
    fn unfocused_solid_button_does_not_render_focus_markers() {
        let button = button::<()>("OK");
        let buffer = render_button(&button, 6, 1, false);

        assert_ne!(cell_char(&buffer, 0, 0), Some('>'));
        assert_ne!(cell_char(&buffer, 5, 0), Some('<'));
    }

    #[test]
    fn focused_ghost_button_highlights_border_without_markers() {
        let button = button::<()>("OK").variant(ButtonVariant::Ghost);
        let buffer = render_button(&button, 8, 3, true);

        for (x, expected) in "│  OK  │".chars().enumerate() {
            assert_eq!(cell_char(&buffer, x as u16, 1), Some(expected));
        }
        let theme = Theme::dark();
        assert_eq!(
            buffer.content()[8].content.style,
            theme.styles.border_focused.to_render_style()
        );
        assert_eq!(
            buffer.content()[11].content.style,
            theme.styles.interactive_focused.to_render_style()
        );

        let narrow = render_button(&button, 4, 3, true);
        assert_eq!(cell_char(&narrow, 1, 1), Some('O'));
        assert_eq!(cell_char(&narrow, 2, 1), Some('K'));
    }

    #[test]
    fn disabled_button_does_not_render_focus_markers() {
        let button = button::<()>("OK").disabled(true);
        let buffer = render_button(&button, 6, 1, true);

        assert_ne!(cell_char(&buffer, 0, 0), Some('>'));
        assert_ne!(cell_char(&buffer, 5, 0), Some('<'));
    }

    #[test]
    fn left_click_emits_click_message() {
        let button = button("OK").on_click(|| "clicked");
        let mut store = WidgetStore::new();
        let mut messages = Vec::new();
        let path = WidgetPath::root();
        let id = crate::tui::widget::WidgetId::root();
        let mut geometry = HashMap::new();
        geometry.insert(path.clone(), Area::new((0, 0).into(), (6, 1).into()));
        let event = Event::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 2,
            row: 0,
            modifiers: crate::tui::event::KeyModifiers::empty(),
        });
        let mut ctx = crate::tui::widget::EventCtx::new(
            &mut store,
            &mut messages,
            path,
            id,
            None,
            &geometry,
            crate::tui::widget::EventPhase::Target,
            false,
        );

        button.handle_event(&event, &mut ctx);

        assert_eq!(messages, vec!["clicked"]);
    }

    fn render_button(button: &Button<()>, width: u16, height: u16, focused: bool) -> Buffer {
        let mut buffer = Buffer::new((width, height).into());
        let area = Area::new((0, 0).into(), (width, height).into());
        let mut chunk = Chunk::new(&mut buffer, area).unwrap();
        let store = WidgetStore::new();
        let animation_store = AnimationStore::new();
        let theme = Theme::dark();
        let geometry = RefCell::new(HashMap::<WidgetPath, Area>::new());
        let focused_path = focused.then(WidgetPath::root);
        let ctx = crate::tui::widget::RenderCtx::new(
            &store,
            &animation_store,
            &theme,
            focused_path.as_ref(),
            &geometry,
        );

        button.render(&mut chunk, &ctx);
        drop(chunk);
        buffer
    }

    fn cell_char(buffer: &Buffer, x: u16, y: u16) -> Option<char> {
        let index = (y * buffer.size().width + x) as usize;
        buffer.content()[index].content.as_char()
    }
}
