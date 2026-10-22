//! Divider widget — visual separator line

use crate::tui::event::Event;
use crate::tui::layout::Constraints;
use crate::tui::style::Style;
use crate::tui::widget::{EventCtx, RenderCtx, Widget};
use crate::tui::widgets::VariantWidget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DividerDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DividerTextPosition {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DividerVariant {
    #[default]
    Solid,
    Dashed,
    Dotted,
    Heavy,
    Double,
    Faded,
}

impl DividerVariant {
    fn chars(&self) -> (char, char) {
        match self {
            DividerVariant::Solid => ('─', '│'),
            DividerVariant::Dashed => ('╌', '╎'),
            DividerVariant::Dotted => ('·', '┊'),
            DividerVariant::Heavy => ('━', '┃'),
            DividerVariant::Double => ('═', '║'),
            DividerVariant::Faded => ('┈', '┊'),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Divider<M = ()> {
    direction: DividerDirection,
    variant: DividerVariant,
    style: Style,
    text: Option<String>,
    text_position: DividerTextPosition,
    text_spacing: u16,
    constraints: Constraints,
    widget_key: Option<String>,
    _phantom: std::marker::PhantomData<M>,
}

impl<M> Divider<M> {
    pub fn new() -> Self {
        Self {
            direction: DividerDirection::Horizontal,
            variant: DividerVariant::default(),
            style: Style::default(),
            text: None,
            text_position: DividerTextPosition::default(),
            text_spacing: 1,
            constraints: Constraints::content(),
            widget_key: None,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn horizontal(mut self) -> Self {
        self.direction = DividerDirection::Horizontal;
        self
    }

    pub fn vertical(mut self) -> Self {
        self.direction = DividerDirection::Vertical;
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn text_position(mut self, position: DividerTextPosition) -> Self {
        self.text_position = position;
        self
    }

    pub fn text_spacing(mut self, spacing: u16) -> Self {
        self.text_spacing = spacing;
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn width(mut self, width: u16) -> Self {
        self.constraints.min_width = width;
        self.constraints.max_width = Some(width);
        self
    }

    pub fn height(mut self, height: u16) -> Self {
        self.constraints.min_height = height;
        self.constraints.max_height = Some(height);
        self
    }

    pub fn flex(mut self, flex: f32) -> Self {
        self.constraints.flex = Some(flex);
        self
    }

    pub fn fill(mut self) -> Self {
        self.constraints = Constraints::fill();
        self
    }

    fn render_horizontal_with_text(
        &self,
        chunk: &mut crate::render::chunk::Chunk,
        line_char: char,
        text: &str,
        render_style: crate::render::style::Style,
    ) {
        use crate::render::text::TextWidth;

        let area = chunk.area();
        let width = area.width() as usize;
        let text_width = text.width();
        let spacing = self.text_spacing as usize;
        let total_text_width = text_width + spacing * 2;

        if total_text_width >= width {
            let line = line_char.to_string().repeat(width);
            let _ = chunk.set_string(0, 0, &line, render_style);
            return;
        }

        let (left_line_len, right_line_len) = match self.text_position {
            DividerTextPosition::Left => {
                let right_len = width.saturating_sub(total_text_width + spacing);
                (spacing, right_len)
            }
            DividerTextPosition::Center => {
                let remaining = width.saturating_sub(total_text_width);
                let left_len = remaining / 2;
                let right_len = remaining - left_len;
                (left_len, right_len)
            }
            DividerTextPosition::Right => {
                let left_len = width.saturating_sub(total_text_width + spacing);
                (left_len, spacing)
            }
        };

        let mut result = String::new();
        result.push_str(&line_char.to_string().repeat(left_line_len));
        result.push_str(&" ".repeat(spacing));
        result.push_str(text);
        result.push_str(&" ".repeat(spacing));
        result.push_str(&line_char.to_string().repeat(right_line_len));

        let _ = chunk.set_string(0, 0, &result, render_style);
    }
}

impl<M> Default for Divider<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> VariantWidget for Divider<M> {
    type Variant = DividerVariant;

    fn variant(mut self, variant: Self::Variant) -> Self {
        self.variant = variant;
        self
    }
}

impl<M> Widget<M> for Divider<M> {
    fn render(&self, chunk: &mut crate::render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        let theme_style = ctx.theme().styles.border;
        let final_style = self.style.merge(theme_style);
        let render_style = final_style.to_render_style();
        let (h_char, v_char) = self.variant.chars();

        match self.direction {
            DividerDirection::Horizontal => {
                if let Some(ref text) = self.text {
                    self.render_horizontal_with_text(chunk, h_char, text, render_style);
                } else {
                    let line = h_char.to_string().repeat(area.width() as usize);
                    let _ = chunk.set_string(0, 0, &line, render_style);
                }
            }
            DividerDirection::Vertical => {
                for y in 0..area.height() {
                    let _ = chunk.set_string(0, y, &v_char.to_string(), render_style);
                }
            }
        }
    }

    fn handle_event(&self, _event: &Event, _ctx: &mut EventCtx<M>) {}

    fn constraints(&self) -> Constraints {
        if self.constraints.max_width.is_some()
            || self.constraints.max_height.is_some()
            || self.constraints.flex.is_some()
        {
            return self.constraints;
        }

        match self.direction {
            DividerDirection::Horizontal => Constraints {
                min_width: 1,
                max_width: None,
                min_height: 1,
                max_height: Some(1),
                flex: Some(1.0),
            },
            DividerDirection::Vertical => Constraints {
                min_width: 1,
                max_width: Some(1),
                min_height: 1,
                max_height: None,
                flex: None,
            },
        }
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

/// Create a new divider widget.
pub fn divider<M>() -> Divider<M> {
    Divider::new()
}
