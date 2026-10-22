//! Label widget — pure text display

use std::borrow::Cow;

use crate::event::Event;
use crate::layout::{
    AxisLimit, Constraints, HorizontalAlign, LayoutStyle, MeasuredSize, SizeProposal, VerticalAlign,
};
use crate::style::{Color, Style};
use crate::text::{TextMetrics, UnicodeSegmentation};
use crate::widget::{EventCtx, MeasureCtx, RenderCtx, Widget};

/// Label widget for displaying text.
#[derive(Debug, Clone)]
pub struct Label<M = ()> {
    content: String,
    style: Style,
    constraints: Option<Constraints>,
    horizontal_align: HorizontalAlign,
    vertical_align: VerticalAlign,
    wrap: TextWrap,
    tab_width: Option<u16>,
    widget_key: Option<String>,
    _phantom: std::marker::PhantomData<M>,
}

/// Wrapping strategy used by [`Label`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextWrap {
    /// Keep existing label behavior: render each source line as one terminal row.
    #[default]
    None,
    /// Wrap on word boundaries, falling back to character wrapping for long words.
    Word,
    /// Wrap at character boundaries using terminal display width.
    Char,
}

impl<M> Label<M> {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            style: Style::default(),
            constraints: None,
            horizontal_align: HorizontalAlign::Left,
            vertical_align: VerticalAlign::Top,
            wrap: TextWrap::None,
            tab_width: None,
            widget_key: None,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Override the label's layout constraints.
    pub fn constraints(mut self, constraints: Constraints) -> Self {
        self.constraints = Some(constraints);
        self
    }

    /// Give the label an exact width in terminal cells.
    pub fn width(mut self, width: u16) -> Self {
        let mut constraints = self.effective_constraints();
        constraints.min_width = width;
        constraints.max_width = Some(width);
        self.constraints = Some(constraints);
        self
    }

    /// Give the label an exact height in terminal rows.
    pub fn height(mut self, height: u16) -> Self {
        let mut constraints = self.effective_constraints();
        constraints.min_height = height;
        constraints.max_height = Some(height);
        self.constraints = Some(constraints);
        self
    }

    /// Give the label an exact width and height.
    pub fn fixed(mut self, width: u16, height: u16) -> Self {
        self.constraints = Some(Constraints::fixed(width, height));
        self
    }

    /// Set horizontal placement for each rendered line inside the allocated area.
    pub fn align(mut self, align: HorizontalAlign) -> Self {
        self.horizontal_align = align;
        self
    }

    /// Set vertical placement for the text block inside the allocated area.
    pub fn valign(mut self, align: VerticalAlign) -> Self {
        self.vertical_align = align;
        self
    }

    /// Set the label's wrapping strategy.
    ///
    /// The default is [`TextWrap::None`] to preserve existing no-wrap behavior.
    pub fn wrap(mut self, wrap: TextWrap) -> Self {
        self.wrap = wrap;
        self
    }

    /// Wrap text on word boundaries, falling back to character wrapping for long words.
    pub fn wrap_word(self) -> Self {
        self.wrap(TextWrap::Word)
    }

    /// Wrap text at character boundaries using terminal display width.
    pub fn wrap_char(self) -> Self {
        self.wrap(TextWrap::Char)
    }

    /// Expand tab characters to terminal-cell tab stops while measuring and rendering.
    pub fn tab_width(mut self, width: u16) -> Self {
        self.tab_width = Some(width.max(1));
        self
    }

    pub fn fg(mut self, color: Color) -> Self {
        self.style = self.style.fg(color);
        self
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.style = self.style.bg(color);
        self
    }

    pub fn bold(mut self) -> Self {
        self.style = self.style.bold();
        self
    }

    pub fn italic(mut self) -> Self {
        self.style = self.style.italic();
        self
    }

    pub fn underline(mut self) -> Self {
        self.style = self.style.underlined();
        self
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    fn effective_constraints(&self) -> Constraints {
        self.constraints_with(TextMetrics::default())
    }

    fn constraints_with(&self, metrics: TextMetrics) -> Constraints {
        self.constraints.unwrap_or_else(|| {
            let (width, height) = label_size(&self.content, self.tab_width, metrics);
            if self.wrap != TextWrap::None {
                return Constraints {
                    min_width: min_content_width(&self.content, self.tab_width, self.wrap, metrics),
                    max_width: None,
                    min_height: height,
                    max_height: None,
                    flex: None,
                };
            }

            Constraints {
                min_width: width,
                max_width: Some(width),
                min_height: height,
                max_height: Some(height),
                flex: None,
            }
        })
    }
}

impl<M> Widget<M> for Label<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk, ctx: &RenderCtx) {
        let metrics = ctx.theme().text_metrics;
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }
        let theme_style = ctx.theme().styles.text;
        let final_style = self.style.merge(theme_style);
        let render_style = final_style.to_render_style();
        let lines = render_lines(
            &self.content,
            self.tab_width,
            self.wrap,
            area.width(),
            metrics,
        );
        let content_height = lines.len().min(u16::MAX as usize) as u16;
        let y_offset = self.vertical_align.offset(area.height(), content_height);
        let visible_rows = area.height().saturating_sub(y_offset) as usize;

        for (row, line) in lines.iter().enumerate() {
            if row >= visible_rows {
                break;
            }

            let y = y_offset + row as u16;
            let line_width = display_width(line, metrics);
            let x = self.horizontal_align.offset(area.width(), line_width);
            let _ = chunk.set_string(x, y, line, render_style);
        }
    }

    fn handle_event(&self, _event: &Event, _ctx: &mut EventCtx<M>) {}

    fn constraints(&self) -> Constraints {
        self.effective_constraints()
    }

    fn layout_style(&self) -> LayoutStyle {
        self.layout_with(TextMetrics::default())
    }

    fn layout_style_with(&self, ctx: &MeasureCtx) -> LayoutStyle {
        self.layout_with(ctx.theme().text_metrics)
    }

    fn measure(&self, proposal: SizeProposal, ctx: &MeasureCtx) -> MeasuredSize {
        self.measure_with(proposal, ctx.theme().text_metrics)
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

impl<M> Label<M> {
    fn layout_with(&self, metrics: TextMetrics) -> LayoutStyle {
        let mut style = LayoutStyle::from_constraints(self.constraints_with(metrics));
        if self.constraints.is_none() && self.wrap != TextWrap::None {
            let (width, height) = label_size(&self.content, self.tab_width, metrics);
            style.preferred_width = Some(width);
            style.preferred_height = Some(height);
        }
        style
    }

    fn measure_with(&self, proposal: SizeProposal, metrics: TextMetrics) -> MeasuredSize {
        let measured = match self.wrap {
            TextWrap::None => {
                let (width, height) = label_size(&self.content, self.tab_width, metrics);
                MeasuredSize::new(width, height)
            }
            TextWrap::Word | TextWrap::Char => match proposal.width {
                AxisLimit::Unbounded => {
                    let (width, height) = label_size(&self.content, self.tab_width, metrics);
                    MeasuredSize::new(width, height)
                }
                AxisLimit::AtMost(width) => {
                    let lines =
                        render_lines(&self.content, self.tab_width, self.wrap, width, metrics);
                    MeasuredSize::new(
                        max_line_width(&lines, metrics).min(width),
                        measured_line_count(&lines),
                    )
                }
                AxisLimit::Exact(width) => {
                    let lines =
                        render_lines(&self.content, self.tab_width, self.wrap, width, metrics);
                    MeasuredSize::new(width, measured_line_count(&lines))
                }
            },
        };

        self.layout_with(metrics).clamp_size(measured)
    }
}

/// Create a new label widget.
pub fn label<M>(content: impl Into<String>) -> Label<M> {
    Label::new(content)
}

fn label_size(content: &str, tab_width: Option<u16>, metrics: TextMetrics) -> (u16, u16) {
    if content.is_empty() {
        return (0, 0);
    }

    let mut width: u16 = 0;
    let mut height: u16 = 0;
    for line in content.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let line = expand_tabs(line, tab_width, metrics);
        width = width.max(display_width(line.as_ref(), metrics));
        height = height.saturating_add(1);
    }

    (width, height)
}

fn display_width(content: &str, metrics: TextMetrics) -> u16 {
    metrics.width(content).min(u16::MAX as usize) as u16
}

fn render_lines(
    content: &str,
    tab_width: Option<u16>,
    wrap: TextWrap,
    width: u16,
    metrics: TextMetrics,
) -> Vec<String> {
    if content.is_empty() {
        return Vec::new();
    }

    match wrap {
        TextWrap::None => content
            .split('\n')
            .map(|line| {
                let line = line.strip_suffix('\r').unwrap_or(line);
                expand_tabs(line, tab_width, metrics).into_owned()
            })
            .collect(),
        TextWrap::Word => wrap_lines_word(content, tab_width, width, metrics),
        TextWrap::Char => wrap_lines_char(content, tab_width, width, metrics),
    }
}

fn wrap_lines_word(
    content: &str,
    tab_width: Option<u16>,
    width: u16,
    metrics: TextMetrics,
) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }

    let mut lines = Vec::new();
    for source_line in content.split('\n') {
        let source_line = source_line.strip_suffix('\r').unwrap_or(source_line);
        let source_line = expand_tabs(source_line, tab_width, metrics);
        let mut current = String::new();
        let mut current_width: u16 = 0;

        for word in source_line.split_whitespace() {
            let word_width = display_width(word, metrics);
            if word_width > width {
                if !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                    current_width = 0;
                }
                let mut wrapped_word = wrap_single_line_char(word, width, metrics);
                if let Some(last) = wrapped_word.pop() {
                    lines.extend(wrapped_word);
                    current_width = display_width(&last, metrics);
                    current = last;
                }
                continue;
            }

            let separator = u16::from(!current.is_empty());
            if current_width
                .saturating_add(separator)
                .saturating_add(word_width)
                <= width
            {
                if !current.is_empty() {
                    current.push(' ');
                    current_width = current_width.saturating_add(1);
                }
                current.push_str(word);
                current_width = current_width.saturating_add(word_width);
            } else {
                lines.push(std::mem::take(&mut current));
                current.push_str(word);
                current_width = word_width;
            }
        }

        lines.push(current);
    }

    lines
}

fn wrap_lines_char(
    content: &str,
    tab_width: Option<u16>,
    width: u16,
    metrics: TextMetrics,
) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }

    let mut lines = Vec::new();
    for source_line in content.split('\n') {
        let source_line = source_line.strip_suffix('\r').unwrap_or(source_line);
        let source_line = expand_tabs(source_line, tab_width, metrics);
        lines.extend(wrap_single_line_char(source_line.as_ref(), width, metrics));
    }
    lines
}

fn wrap_single_line_char(line: &str, width: u16, metrics: TextMetrics) -> Vec<String> {
    if line.is_empty() {
        return vec![String::new()];
    }

    metrics
        .wrap(line, width as usize)
        .into_iter()
        .map(|line_range| line[line_range.bytes].to_owned())
        .collect()
}

fn min_content_width(
    content: &str,
    tab_width: Option<u16>,
    wrap: TextWrap,
    metrics: TextMetrics,
) -> u16 {
    match wrap {
        TextWrap::None => label_size(content, tab_width, metrics).0,
        TextWrap::Char => content
            .split('\n')
            .flat_map(|line| {
                let line = line.strip_suffix('\r').unwrap_or(line);
                expand_tabs(line, tab_width, metrics)
                    .graphemes(true)
                    .map(|g| metrics.grapheme_width(g, 0).min(u16::MAX as usize) as u16)
                    .collect::<Vec<_>>()
            })
            .max()
            .unwrap_or(0),
        TextWrap::Word => content
            .split('\n')
            .flat_map(|line| {
                let line = line.strip_suffix('\r').unwrap_or(line);
                expand_tabs(line, tab_width, metrics)
                    .split_whitespace()
                    .map(|text| display_width(text, metrics))
                    .collect::<Vec<_>>()
            })
            .max()
            .unwrap_or(0),
    }
}

fn max_line_width(lines: &[String], metrics: TextMetrics) -> u16 {
    lines
        .iter()
        .map(|line| display_width(line, metrics))
        .max()
        .unwrap_or(0)
}

fn measured_line_count(lines: &[String]) -> u16 {
    lines.len().min(u16::MAX as usize) as u16
}

fn expand_tabs(content: &str, tab_width: Option<u16>, metrics: TextMetrics) -> Cow<'_, str> {
    let metrics = TextMetrics {
        tab_width: tab_width.map(usize::from).unwrap_or(metrics.tab_width),
        ..metrics
    };
    if !content.contains('\t') {
        return Cow::Borrowed(content);
    }
    Cow::Owned(metrics.expand(content))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::AnimationStore;
    use crate::style::Theme;
    use crate::widget::{RenderCtx, Widget, WidgetPath, WidgetStore};
    use render::area::Area;
    use render::buffer::Buffer;
    use render::chunk::Chunk;
    use std::cell::RefCell;
    use std::collections::HashMap;

    #[test]
    fn multiline_label_constraints_use_lines() {
        let label = label::<()>("alpha\nbeta\n");
        let constraints = Widget::constraints(&label);

        assert_eq!(constraints.min_width, 5);
        assert_eq!(constraints.max_width, Some(5));
        assert_eq!(constraints.min_height, 3);
        assert_eq!(constraints.max_height, Some(3));
    }

    #[test]
    fn label_measure_reports_intrinsic_size_for_unbounded_proposal() {
        let label = label::<()>("alpha\nbeta");
        let store = WidgetStore::new();
        let theme = Theme::dark();
        let ctx = MeasureCtx::new(&store, &theme);

        let measured = label.measure(SizeProposal::UNBOUNDED, &ctx);

        assert_eq!(measured, MeasuredSize::new(5, 2));
    }

    #[test]
    fn no_wrap_label_keeps_intrinsic_measurement_for_exact_width() {
        let label = label::<()>("abcdef");
        let store = WidgetStore::new();
        let theme = Theme::dark();
        let ctx = MeasureCtx::new(&store, &theme);

        let measured = label.measure(
            SizeProposal {
                width: AxisLimit::Exact(3),
                height: AxisLimit::Unbounded,
            },
            &ctx,
        );

        assert_eq!(measured, MeasuredSize::new(6, 1));
    }

    #[test]
    fn char_wrapping_label_measures_height_for_exact_width() {
        let label = label::<()>("abcdef").wrap_char();
        let store = WidgetStore::new();
        let theme = Theme::dark();
        let ctx = MeasureCtx::new(&store, &theme);

        let measured = label.measure(
            SizeProposal {
                width: AxisLimit::Exact(3),
                height: AxisLimit::Unbounded,
            },
            &ctx,
        );

        assert_eq!(measured, MeasuredSize::new(3, 2));
    }

    #[test]
    fn word_wrapping_label_uses_proposed_width() {
        let label = label::<()>("alpha beta gamma").wrap_word();
        let store = WidgetStore::new();
        let theme = Theme::dark();
        let ctx = MeasureCtx::new(&store, &theme);

        let measured = label.measure(
            SizeProposal {
                width: AxisLimit::AtMost(10),
                height: AxisLimit::Unbounded,
            },
            &ctx,
        );

        assert_eq!(measured, MeasuredSize::new(10, 2));
    }

    #[test]
    fn label_width_and_height_override_intrinsic_constraints() {
        let label = label::<()>("alpha").width(12).height(4);
        let constraints = Widget::constraints(&label);

        assert_eq!(constraints.min_width, 12);
        assert_eq!(constraints.max_width, Some(12));
        assert_eq!(constraints.min_height, 4);
        assert_eq!(constraints.max_height, Some(4));
    }

    #[test]
    fn label_tab_width_expands_tabs_for_constraints() {
        let label = label::<()>("a\tb").tab_width(4);
        let constraints = Widget::constraints(&label);

        assert_eq!(constraints.min_width, 5);
        assert_eq!(constraints.max_width, Some(5));
    }

    #[test]
    fn multiline_label_renders_each_line() {
        let label = label::<()>("ab\ncd");
        let buffer = render_widget(&label, 4, 2);

        assert_eq!(cell_char(&buffer, 0, 0), Some('a'));
        assert_eq!(cell_char(&buffer, 1, 0), Some('b'));
        assert_eq!(cell_char(&buffer, 0, 1), Some('c'));
        assert_eq!(cell_char(&buffer, 1, 1), Some('d'));
    }

    #[test]
    fn label_can_center_multiline_content_in_allocated_area() {
        let label = label::<()>("ab\ncd")
            .fixed(6, 4)
            .align(HorizontalAlign::Center)
            .valign(VerticalAlign::Middle);
        let buffer = render_widget(&label, 6, 4);

        assert_eq!(cell_char(&buffer, 2, 1), Some('a'));
        assert_eq!(cell_char(&buffer, 3, 1), Some('b'));
        assert_eq!(cell_char(&buffer, 2, 2), Some('c'));
        assert_eq!(cell_char(&buffer, 3, 2), Some('d'));
    }

    #[test]
    fn label_can_align_content_to_bottom_right() {
        let label = label::<()>("xy")
            .fixed(5, 3)
            .align(HorizontalAlign::Right)
            .valign(VerticalAlign::Bottom);
        let buffer = render_widget(&label, 5, 3);

        assert_eq!(cell_char(&buffer, 3, 2), Some('x'));
        assert_eq!(cell_char(&buffer, 4, 2), Some('y'));
    }

    #[test]
    fn label_alignment_uses_unicode_display_width() {
        let label = label::<()>("你").width(4).align(HorizontalAlign::Center);
        let buffer = render_widget(&label, 4, 1);

        assert_eq!(cell_char(&buffer, 1, 0), Some('你'));
    }

    #[test]
    fn char_wrapping_label_renders_wrapped_lines() {
        let label = label::<()>("abcdef").wrap_char();
        let buffer = render_widget(&label, 3, 2);

        assert_eq!(cell_char(&buffer, 0, 0), Some('a'));
        assert_eq!(cell_char(&buffer, 2, 0), Some('c'));
        assert_eq!(cell_char(&buffer, 0, 1), Some('d'));
        assert_eq!(cell_char(&buffer, 2, 1), Some('f'));
    }

    fn render_widget(widget: &Label<()>, width: u16, height: u16) -> Buffer {
        let mut buffer = Buffer::new((width, height).into());
        let area = Area::new((0, 0).into(), (width, height).into());
        let mut chunk = Chunk::new(&mut buffer, area).unwrap();
        let store = WidgetStore::new();
        let animation_store = AnimationStore::new();
        let theme = Theme::dark();
        let geometry = RefCell::new(HashMap::<WidgetPath, Area>::new());
        let ctx = RenderCtx::new(&store, &animation_store, &theme, None, &geometry);

        widget.render(&mut chunk, &ctx);
        drop(chunk);
        buffer
    }

    fn cell_char(buffer: &Buffer, x: u16, y: u16) -> Option<char> {
        let index = (y * buffer.size().width + x) as usize;
        buffer.content()[index].content.as_char()
    }
}
