//! Progress and loading indicator widgets.

use crate::render::text::TextWidth;

use crate::tui::layout::Constraints;
use crate::tui::style::Style;
use crate::tui::widget::{RenderCtx, Widget};
use crate::tui::widgets::VariantWidget;

/// Built-in visual treatments for [`ProgressBar`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgressBarVariant {
    /// A slim modern rail with a leading edge marker.
    #[default]
    Line,
    /// A dense bar using block and shaded cells.
    Block,
    /// Discrete filled/empty segments.
    Segmented,
    /// Legacy ASCII-style bar with brackets.
    Classic,
}

/// Horizontal progress bar.
#[derive(Debug, Clone)]
pub struct ProgressBar<M = ()> {
    value: f64,
    label: Option<String>,
    width: u16,
    variant: ProgressBarVariant,
    custom_style: Option<Style>,
    fill_style: Option<Style>,
    widget_key: Option<String>,
    marker: std::marker::PhantomData<fn() -> M>,
}

impl<M> ProgressBar<M> {
    pub fn new(value: f64) -> Self {
        Self {
            value,
            label: None,
            width: 24,
            variant: ProgressBarVariant::default(),
            custom_style: None,
            fill_style: None,
            widget_key: None,
            marker: std::marker::PhantomData,
        }
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn value(mut self, value: f64) -> Self {
        self.value = value;
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn width(mut self, width: u16) -> Self {
        self.width = width.max(3);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }

    pub fn fill_style(mut self, style: Style) -> Self {
        self.fill_style = Some(style);
        self
    }
}

impl<M> VariantWidget for ProgressBar<M> {
    type Variant = ProgressBarVariant;

    fn variant(mut self, variant: Self::Variant) -> Self {
        self.variant = variant;
        self
    }
}

impl<M> Widget<M> for ProgressBar<M> {
    fn render(&self, chunk: &mut crate::render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        let theme = ctx.theme();
        let track_style = self
            .custom_style
            .map(|style| style.merge(theme.styles.scrollbar_track))
            .unwrap_or(theme.styles.scrollbar_track)
            .to_render_style();
        let fill_style = self
            .fill_style
            .map(|style| style.merge(theme.styles.scrollbar_thumb))
            .unwrap_or(theme.styles.scrollbar_thumb)
            .to_render_style();
        let value = self.value.clamp(0.0, 1.0);
        let bar_width = area.width();

        match self.variant {
            ProgressBarVariant::Line => {
                render_line_bar(chunk, bar_width, value, track_style, fill_style)
            }
            ProgressBarVariant::Block => {
                render_block_bar(chunk, bar_width, value, track_style, fill_style)
            }
            ProgressBarVariant::Segmented => {
                render_segmented_bar(chunk, bar_width, value, track_style, fill_style)
            }
            ProgressBarVariant::Classic => {
                render_classic_bar(chunk, bar_width, value, track_style, fill_style)
            }
        }

        if let Some(label) = self.label.as_ref() {
            let text = truncate_to_width(label, area.width() as usize, chunk.text_metrics());
            let x = area
                .width()
                .saturating_sub(chunk.text_metrics().width(&text) as u16)
                / 2;
            let _ = chunk.set_string(x, 0, &text, fill_style);
        }
    }

    fn constraints(&self) -> Constraints {
        Constraints {
            min_width: self.width,
            max_width: None,
            min_height: 1,
            max_height: Some(1),
            flex: Some(1.0),
        }
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

fn filled_slots(width: u16, value: f64) -> u16 {
    if value <= 0.0 {
        0
    } else {
        ((width as f64) * value).ceil() as u16
    }
    .min(width)
}

fn render_line_bar(
    chunk: &mut crate::render::chunk::Chunk,
    width: u16,
    value: f64,
    track_style: crate::render::style::Style,
    fill_style: crate::render::style::Style,
) {
    let filled = filled_slots(width, value);
    let head = filled.saturating_sub(1);

    for x in 0..width {
        let (ch, style) = if value >= 1.0 || x < head {
            ('━', fill_style)
        } else if filled > 0 && x == head {
            ('╸', fill_style)
        } else {
            ('─', track_style)
        };
        let _ = chunk.set_char(x, 0, ch, style);
    }
}

fn render_block_bar(
    chunk: &mut crate::render::chunk::Chunk,
    width: u16,
    value: f64,
    track_style: crate::render::style::Style,
    fill_style: crate::render::style::Style,
) {
    let progress = value * width as f64;
    let full = progress.floor() as u16;
    let partial = progress - full as f64;
    let partials = ['▏', '▎', '▍', '▌', '▋', '▊', '▉'];

    for x in 0..width {
        let (ch, style) = if x < full {
            ('█', fill_style)
        } else if x == full && partial > 0.0 && value < 1.0 {
            let index = ((partial * partials.len() as f64).ceil() as usize)
                .saturating_sub(1)
                .min(partials.len() - 1);
            (partials[index], fill_style)
        } else {
            ('░', track_style)
        };
        let _ = chunk.set_char(x, 0, ch, style);
    }
}

fn render_segmented_bar(
    chunk: &mut crate::render::chunk::Chunk,
    width: u16,
    value: f64,
    track_style: crate::render::style::Style,
    fill_style: crate::render::style::Style,
) {
    let filled = filled_slots(width, value);

    for x in 0..width {
        let (ch, style) = if x < filled {
            ('■', fill_style)
        } else {
            ('□', track_style)
        };
        let _ = chunk.set_char(x, 0, ch, style);
    }
}

fn render_classic_bar(
    chunk: &mut crate::render::chunk::Chunk,
    width: u16,
    value: f64,
    track_style: crate::render::style::Style,
    fill_style: crate::render::style::Style,
) {
    if width < 3 {
        render_segmented_bar(chunk, width, value, track_style, fill_style);
        return;
    }

    let inner_width = width.saturating_sub(2);
    let filled = ((inner_width as f64) * value).round() as u16;

    let _ = chunk.set_char(0, 0, '[', track_style);
    for x in 0..inner_width {
        let (ch, style) = if x < filled {
            ('#', fill_style)
        } else {
            ('-', track_style)
        };
        let _ = chunk.set_char(x + 1, 0, ch, style);
    }
    let _ = chunk.set_char(width - 1, 0, ']', track_style);
}

/// Small frame-driven loading indicator.
#[derive(Debug, Clone)]
pub struct LoadingIndicator<M = ()> {
    label: Option<String>,
    frame: usize,
    custom_style: Option<Style>,
    widget_key: Option<String>,
    marker: std::marker::PhantomData<fn() -> M>,
}

impl<M> LoadingIndicator<M> {
    pub fn new() -> Self {
        Self {
            label: None,
            frame: 0,
            custom_style: None,
            widget_key: None,
            marker: std::marker::PhantomData,
        }
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn frame(mut self, frame: usize) -> Self {
        self.frame = frame;
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }
}

impl<M> Default for LoadingIndicator<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> Widget<M> for LoadingIndicator<M> {
    fn render(&self, chunk: &mut crate::render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        let theme = ctx.theme();
        let spinner_style = self
            .custom_style
            .map(|style| style.merge(theme.styles.validation_info))
            .unwrap_or(theme.styles.validation_info)
            .to_render_style();
        let label_style = self
            .custom_style
            .map(|style| style.merge(theme.styles.text))
            .unwrap_or(theme.styles.text)
            .to_render_style();
        let frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let frame = self.frame;
        let spinner = frames[frame % frames.len()];
        let _ = chunk.set_char(0, 0, spinner, spinner_style);

        if area.width() <= 2 {
            return;
        }

        if let Some(label) = self.label.as_ref() {
            let _ = chunk.set_char(1, 0, ' ', label_style);
            let display = truncate_to_width(
                label,
                area.width().saturating_sub(2) as usize,
                chunk.text_metrics(),
            );
            let _ = chunk.set_string(2, 0, &display, label_style);
        }
    }

    fn constraints(&self) -> Constraints {
        let width = self
            .label
            .as_ref()
            .map(|label| label.width() as u16 + 2)
            .unwrap_or(1);
        Constraints {
            min_width: width,
            max_width: None,
            min_height: 1,
            max_height: Some(1),
            flex: None,
        }
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

fn truncate_to_width(
    text: &str,
    max_width: usize,
    metrics: crate::render::text::TextMetrics,
) -> String {
    metrics.clip(text, 0, max_width)
}

pub fn progress_bar<M>(value: f64) -> ProgressBar<M> {
    ProgressBar::new(value)
}

pub fn loading_indicator<M>() -> LoadingIndicator<M> {
    LoadingIndicator::new()
}

#[cfg(test)]
mod tests {
    use crate::tui::animation::AnimationStore;
    use crate::tui::style::Theme;
    use crate::tui::widget::{RenderCtx, Widget, WidgetStore};

    use super::*;

    #[test]
    fn progress_renders_current_value_directly() {
        let widget = ProgressBar::<()>::new(0.5);
        assert_eq!(render_progress(widget, 10), "━━━━╸─────");
    }

    #[test]
    fn progress_supports_built_in_variants() {
        assert_eq!(
            render_progress(
                ProgressBar::<()>::new(0.5).variant(ProgressBarVariant::Line),
                10
            ),
            "━━━━╸─────"
        );
        assert_eq!(
            render_progress(
                ProgressBar::<()>::new(0.45).variant(ProgressBarVariant::Block),
                10
            ),
            "████▌░░░░░"
        );
        assert_eq!(
            render_progress(
                ProgressBar::<()>::new(0.5).variant(ProgressBarVariant::Segmented),
                10
            ),
            "■■■■■□□□□□"
        );
        assert_eq!(
            render_progress(
                ProgressBar::<()>::new(0.5).variant(ProgressBarVariant::Classic),
                10
            ),
            "[####----]"
        );
    }

    #[test]
    fn loading_indicator_uses_modern_spinner_frames() {
        let widget = LoadingIndicator::<()>::new().frame(3).label("Loading");
        let mut buffer = crate::render::buffer::Buffer::new((10, 1).into());
        let area = crate::render::area::Area::new((0, 0).into(), (10, 1).into());
        let mut chunk = crate::render::chunk::Chunk::new(&mut buffer, area).unwrap();
        let store = WidgetStore::default();
        let animation_store = AnimationStore::new();
        let theme = Theme::dark();
        let geometry = std::cell::RefCell::new(std::collections::HashMap::new());
        let ctx = RenderCtx::new(&store, &animation_store, &theme, None, &geometry);

        widget.render(&mut chunk, &ctx);

        let rendered: String = (0..9)
            .map(|x| {
                let index = x as usize;
                buffer.content()[index].content.as_char().unwrap()
            })
            .collect();
        assert_eq!(rendered, "⠸ Loading");
    }

    fn render_progress(widget: ProgressBar<()>, width: u16) -> String {
        let mut buffer = crate::render::buffer::Buffer::new((width, 1).into());
        let area = crate::render::area::Area::new((0, 0).into(), (width, 1).into());
        let mut chunk = crate::render::chunk::Chunk::new(&mut buffer, area).unwrap();
        let store = WidgetStore::default();
        let animation_store = AnimationStore::new();
        let theme = Theme::dark();
        let geometry = std::cell::RefCell::new(std::collections::HashMap::new());
        let ctx = RenderCtx::new(&store, &animation_store, &theme, None, &geometry);

        widget.render(&mut chunk, &ctx);

        (0..width)
            .map(|x| {
                let index = x as usize;
                buffer.content()[index].content.as_char().unwrap()
            })
            .collect()
    }
}
