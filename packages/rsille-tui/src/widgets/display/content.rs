//! Scrollable, read-only viewers for code, logs, diffs, and Markdown.

use std::sync::Arc;

use rsille_render::text::{Line, TextWidth};

use crate::event::{Event, KeyCode, MouseEventKind};
use crate::focus::FocusConfig;
use crate::layout::border_renderer;
use crate::layout::{ensure_item_visible, AxisLimit, Constraints, MeasuredSize, SizeProposal};
use crate::style::{BorderStyle, Color, Style};
use crate::widget::{EventCtx, EventPhase, MeasureCtx, RenderCtx, Widget};

/// Persistent vertical scroll position shared by the content viewers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContentViewerState {
    /// Zero-based index of the first visible row.
    pub scroll_offset: usize,
}

/// Severity used to style a log entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Fine-grained diagnostic detail.
    Trace,
    /// Debugging information.
    Debug,
    /// Informational messages.
    Info,
    /// Warnings requiring attention.
    Warn,
    /// Errors or failed operations.
    Error,
}

/// A log message paired with its severity.
///
/// String conversions infer severity by looking for ERROR, WARN, DEBUG, or TRACE
/// (case-insensitively, in that order), otherwise choosing [`LogLevel::Info`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    /// Severity used to choose the line style.
    pub level: LogLevel,
    /// Text of the log entry.
    pub message: String,
}

impl LogLine {
    /// Creates a log entry with an explicit severity and message.
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
        }
    }
}

impl From<&str> for LogLine {
    fn from(value: &str) -> Self {
        let upper = value.to_ascii_uppercase();
        let level = if upper.contains("ERROR") {
            LogLevel::Error
        } else if upper.contains("WARN") {
            LogLevel::Warn
        } else if upper.contains("DEBUG") {
            LogLevel::Debug
        } else if upper.contains("TRACE") {
            LogLevel::Trace
        } else {
            LogLevel::Info
        };
        Self::new(level, value)
    }
}

impl From<String> for LogLine {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

/// A scrollable, read-only source-code viewer with optional line numbers.
///
/// Use [`Self::new`] for plain source or [`Self::from_lines`] for styled source.
pub struct CodeViewer<M = ()> {
    lines: Arc<[Line<'static>]>,
    height: u16,
    border: Option<BorderStyle>,
    show_line_numbers: bool,
    custom_style: Option<Style>,
    widget_key: Option<String>,
    _phantom: std::marker::PhantomData<M>,
}

impl<M> std::fmt::Debug for CodeViewer<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodeViewer")
            .field("line_count", &self.lines.len())
            .field("height", &self.height)
            .field("border", &self.border)
            .field("show_line_numbers", &self.show_line_numbers)
            .finish()
    }
}

impl<M> CodeViewer<M> {
    /// Creates a twelve-row-high code viewer with a single border and line numbers.
    pub fn new(code: impl Into<String>) -> Self {
        let code = code.into();
        Self::from_lines(
            code.lines()
                .map(|text| Line::from(text.to_owned()))
                .collect::<Vec<_>>(),
        )
    }

    /// Creates a viewer from styled logical lines, one per row.
    ///
    /// Pass a vector directly, or share an `Arc<[Line<'static>]>` across view rebuilds.
    pub fn from_lines(lines: impl Into<Arc<[Line<'static>]>>) -> Self {
        Self {
            lines: lines.into(),
            height: 12,
            border: Some(BorderStyle::Single),
            show_line_numbers: true,
            custom_style: None,
            widget_key: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Sets the stable widget key used to identify this widget across view rebuilds.
    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    /// Sets the height in terminal rows, clamped to at least 1.
    pub fn height(mut self, height: u16) -> Self {
        self.height = height.max(1);
        self
    }

    /// Sets the border drawn around the widget.
    pub fn border(mut self, border: BorderStyle) -> Self {
        self.border = Some(border);
        self
    }

    /// Removes the border and its reserved space.
    pub fn borderless(mut self) -> Self {
        self.border = None;
        self
    }

    /// Sets whether line numbers are displayed; enabled by default.
    pub fn line_numbers(mut self, show: bool) -> Self {
        self.show_line_numbers = show;
        self
    }

    /// Sets the custom rendering style.
    pub fn style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }
}

impl<M> Default for CodeViewer<M> {
    fn default() -> Self {
        Self::new("")
    }
}

impl<M: 'static> Widget<M> for CodeViewer<M> {
    fn render(&self, chunk: &mut rsille_render::chunk::Chunk, ctx: &RenderCtx) {
        render_text_viewer(
            chunk,
            ctx,
            self.border,
            self.height,
            self.custom_style,
            self.lines.iter(),
            self.lines.len(),
            self.show_line_numbers,
            |row, line, _| {
                let _ = row.set_line(0, 0, line);
            },
        );
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if crate::event::is_key_release(event) {
            return;
        }
        handle_scroll_event(
            event,
            ctx,
            self.lines.len(),
            self.height,
            self.border.is_some(),
        );
    }

    fn constraints(&self) -> Constraints {
        content_constraints(self.height, self.border, 24)
    }

    fn measure(&self, proposal: SizeProposal, _ctx: &MeasureCtx) -> MeasuredSize {
        measure_text_viewer(
            self.lines.iter().map(Line::width),
            self.height,
            self.border,
            proposal,
            code_line_number_width(self.lines.len(), self.show_line_numbers),
            24,
        )
    }

    fn focus_config(&self) -> FocusConfig {
        FocusConfig::Composite
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

/// A scrollable, read-only list of severity-styled log messages.
pub struct LogViewer<M = ()> {
    lines: Vec<LogLine>,
    height: u16,
    border: Option<BorderStyle>,
    custom_style: Option<Style>,
    widget_key: Option<String>,
    _phantom: std::marker::PhantomData<M>,
}

impl<M> std::fmt::Debug for LogViewer<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogViewer")
            .field("lines", &self.lines)
            .field("height", &self.height)
            .field("border", &self.border)
            .finish()
    }
}

impl<M> LogViewer<M> {
    /// Creates an empty, twelve-row-high log viewer with a single border.
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            height: 12,
            border: Some(BorderStyle::Single),
            custom_style: None,
            widget_key: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Sets the stable widget key used to identify this widget across view rebuilds.
    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    /// Appends a log entry; strings infer severity as described by [`LogLine`].
    pub fn line(mut self, line: impl Into<LogLine>) -> Self {
        self.lines.push(line.into());
        self
    }

    /// Appends log entries in iteration order.
    pub fn lines<I>(mut self, lines: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<LogLine>,
    {
        self.lines.extend(lines.into_iter().map(Into::into));
        self
    }

    /// Sets the height in terminal rows, clamped to at least 1.
    pub fn height(mut self, height: u16) -> Self {
        self.height = height.max(1);
        self
    }

    /// Sets the border drawn around the widget.
    pub fn border(mut self, border: BorderStyle) -> Self {
        self.border = Some(border);
        self
    }

    /// Removes the border and its reserved space.
    pub fn borderless(mut self) -> Self {
        self.border = None;
        self
    }

    /// Sets the custom rendering style.
    pub fn style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }
}

impl<M> Default for LogViewer<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: 'static> Widget<M> for LogViewer<M> {
    fn render(&self, chunk: &mut rsille_render::chunk::Chunk, ctx: &RenderCtx) {
        render_log(chunk, ctx, self.border, self.custom_style, &self.lines);
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if crate::event::is_key_release(event) {
            return;
        }
        handle_scroll_event(
            event,
            ctx,
            self.lines.len(),
            self.height,
            self.border.is_some(),
        );
    }

    fn constraints(&self) -> Constraints {
        content_constraints(self.height, self.border, 24)
    }

    fn measure(&self, proposal: SizeProposal, _ctx: &MeasureCtx) -> MeasuredSize {
        measure_text_viewer(
            self.lines.iter().map(|line| line.message.width()),
            self.height,
            self.border,
            proposal,
            0,
            24,
        )
    }

    fn focus_config(&self) -> FocusConfig {
        FocusConfig::Composite
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

/// A scrollable, read-only viewer for precomputed diff text.
///
/// Added, removed, and hunk-header lines receive distinct styles; this widget
/// does not compute a diff between files.
pub struct DiffViewer<M = ()> {
    diff: String,
    height: u16,
    border: Option<BorderStyle>,
    custom_style: Option<Style>,
    widget_key: Option<String>,
    _phantom: std::marker::PhantomData<M>,
}

impl<M> std::fmt::Debug for DiffViewer<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiffViewer")
            .field("height", &self.height)
            .field("border", &self.border)
            .finish()
    }
}

impl<M> DiffViewer<M> {
    /// Creates a twelve-row-high viewer for existing diff text with a single border.
    pub fn new(diff: impl Into<String>) -> Self {
        Self {
            diff: diff.into(),
            height: 12,
            border: Some(BorderStyle::Single),
            custom_style: None,
            widget_key: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Sets the stable widget key used to identify this widget across view rebuilds.
    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    /// Sets the height in terminal rows, clamped to at least 1.
    pub fn height(mut self, height: u16) -> Self {
        self.height = height.max(1);
        self
    }

    /// Sets the border drawn around the widget.
    pub fn border(mut self, border: BorderStyle) -> Self {
        self.border = Some(border);
        self
    }

    /// Removes the border and its reserved space.
    pub fn borderless(mut self) -> Self {
        self.border = None;
        self
    }

    /// Sets the custom rendering style.
    pub fn style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }
}

/// A scrollable viewer with lightweight, line-based Markdown styling.
///
/// Headings, lists, quotes, and lines starting with backticks receive styles.
/// Markdown delimiters remain visible; fenced blocks are not parsed as a unit.
pub struct MarkdownViewer<M = ()> {
    markdown: String,
    height: u16,
    border: Option<BorderStyle>,
    custom_style: Option<Style>,
    widget_key: Option<String>,
    _phantom: std::marker::PhantomData<M>,
}

impl<M> std::fmt::Debug for MarkdownViewer<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MarkdownViewer")
            .field("height", &self.height)
            .field("border", &self.border)
            .finish()
    }
}

impl<M> MarkdownViewer<M> {
    /// Creates a twelve-row-high Markdown viewer with a single border.
    pub fn new(markdown: impl Into<String>) -> Self {
        Self {
            markdown: markdown.into(),
            height: 12,
            border: Some(BorderStyle::Single),
            custom_style: None,
            widget_key: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Sets the stable widget key used to identify this widget across view rebuilds.
    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    /// Sets the height in terminal rows, clamped to at least 1.
    pub fn height(mut self, height: u16) -> Self {
        self.height = height.max(1);
        self
    }

    /// Sets the border drawn around the widget.
    pub fn border(mut self, border: BorderStyle) -> Self {
        self.border = Some(border);
        self
    }

    /// Removes the border and its reserved space.
    pub fn borderless(mut self) -> Self {
        self.border = None;
        self
    }

    /// Sets the custom rendering style.
    pub fn style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }
}

impl<M> Default for MarkdownViewer<M> {
    fn default() -> Self {
        Self::new("")
    }
}

impl<M: 'static> Widget<M> for MarkdownViewer<M> {
    fn render(&self, chunk: &mut rsille_render::chunk::Chunk, ctx: &RenderCtx) {
        render_text_viewer(
            chunk,
            ctx,
            self.border,
            self.height,
            self.custom_style,
            self.markdown.lines(),
            self.markdown.lines().count(),
            false,
            draw_markdown_line,
        );
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if crate::event::is_key_release(event) {
            return;
        }
        handle_scroll_event(
            event,
            ctx,
            self.markdown.lines().count(),
            self.height,
            self.border.is_some(),
        );
    }

    fn constraints(&self) -> Constraints {
        content_constraints(self.height, self.border, 24)
    }

    fn measure(&self, proposal: SizeProposal, _ctx: &MeasureCtx) -> MeasuredSize {
        measure_text_viewer(
            self.markdown.lines().map(TextWidth::width),
            self.height,
            self.border,
            proposal,
            0,
            24,
        )
    }

    fn focus_config(&self) -> FocusConfig {
        FocusConfig::Composite
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

impl<M> Default for DiffViewer<M> {
    fn default() -> Self {
        Self::new("")
    }
}

impl<M: 'static> Widget<M> for DiffViewer<M> {
    fn render(&self, chunk: &mut rsille_render::chunk::Chunk, ctx: &RenderCtx) {
        render_text_viewer(
            chunk,
            ctx,
            self.border,
            self.height,
            self.custom_style,
            self.diff.lines(),
            self.diff.lines().count(),
            false,
            draw_diff_line,
        );
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if crate::event::is_key_release(event) {
            return;
        }
        handle_scroll_event(
            event,
            ctx,
            self.diff.lines().count(),
            self.height,
            self.border.is_some(),
        );
    }

    fn constraints(&self) -> Constraints {
        content_constraints(self.height, self.border, 24)
    }

    fn measure(&self, proposal: SizeProposal, _ctx: &MeasureCtx) -> MeasuredSize {
        measure_text_viewer(
            self.diff.lines().map(TextWidth::width),
            self.height,
            self.border,
            proposal,
            0,
            24,
        )
    }

    fn focus_config(&self) -> FocusConfig {
        FocusConfig::Composite
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

fn render_text_viewer<T>(
    chunk: &mut rsille_render::chunk::Chunk,
    ctx: &RenderCtx,
    border: Option<BorderStyle>,
    height: u16,
    custom_style: Option<Style>,
    lines: impl Iterator<Item = T>,
    line_count: usize,
    show_line_numbers: bool,
    draw_line: impl Fn(&mut rsille_render::chunk::Chunk, T, rsille_render::style::Style),
) {
    let area = chunk.area();
    if area.width() == 0 || area.height() == 0 {
        return;
    }

    let theme = ctx.theme();
    let base_style = custom_style
        .map(|style| style.merge(theme.styles.surface))
        .unwrap_or(theme.styles.surface)
        .to_render_style();
    let border_style = if ctx.is_focused() {
        theme.styles.border_focused.to_render_style()
    } else {
        theme.styles.border.to_render_style()
    };
    let muted_style = theme.styles.text_muted.to_render_style();

    let _ = chunk.fill(0, 0, area.width(), area.height(), ' ', base_style);
    let (content_x, content_y, content_width, content_height) =
        content_area(chunk, border, border_style);
    if content_width == 0 || content_height == 0 || line_count == 0 {
        return;
    }

    let state = ctx.state_or_default::<ContentViewerState>();
    let visible_rows = content_height.min(height) as usize;
    let scroll_offset = ensure_item_visible(
        state.scroll_offset.min(line_count - 1),
        state.scroll_offset,
        visible_rows,
    );
    let number_width = code_line_number_width(line_count, show_line_numbers);
    let gutter = number_width.min(content_width as usize) as u16;
    let text_width = content_width - gutter;
    for (row, line) in lines.skip(scroll_offset).take(visible_rows).enumerate() {
        let y = content_y + row as u16;
        if show_line_numbers {
            let number = format!(
                "{:>width$} ",
                scroll_offset + row + 1,
                width = number_width - 1
            );
            let _ = chunk.set_string(content_x, y, &number[..gutter as usize], muted_style);
        }
        let row_area = rsille_render::area::Area::new(
            (area.x() + content_x + gutter, area.y() + y).into(),
            (text_width, 1).into(),
        );
        let _ = chunk.with_clip(row_area, |row_chunk| draw_line(row_chunk, line, base_style));
    }
}

fn render_log(
    chunk: &mut rsille_render::chunk::Chunk,
    ctx: &RenderCtx,
    border: Option<BorderStyle>,
    custom_style: Option<Style>,
    lines: &[LogLine],
) {
    let area = chunk.area();
    if area.width() == 0 || area.height() == 0 {
        return;
    }

    let theme = ctx.theme();
    let base_style = custom_style
        .map(|style| style.merge(theme.styles.surface))
        .unwrap_or(theme.styles.surface)
        .to_render_style();
    let border_style = if ctx.is_focused() {
        theme.styles.border_focused.to_render_style()
    } else {
        theme.styles.border.to_render_style()
    };

    let _ = chunk.fill(0, 0, area.width(), area.height(), ' ', base_style);
    let (content_x, content_y, content_width, content_height) =
        content_area(chunk, border, border_style);
    if content_width == 0 || content_height == 0 || lines.is_empty() {
        return;
    }

    let state = ctx.state_or_default::<ContentViewerState>();
    let visible_rows = content_height as usize;
    let scroll_offset = state.scroll_offset.min(lines.len().saturating_sub(1));

    for row in 0..visible_rows {
        let line_index = scroll_offset + row;
        if line_index >= lines.len() {
            break;
        }
        let line = &lines[line_index];
        let style = log_style(line.level);
        let text = truncate_to_width(&line.message, content_width as usize, chunk.text_metrics());
        let _ = chunk.set_string(
            content_x,
            content_y + row as u16,
            &text,
            style.to_render_style(),
        );
    }
}

fn content_area(
    chunk: &mut rsille_render::chunk::Chunk,
    border: Option<BorderStyle>,
    border_style: rsille_render::style::Style,
) -> (u16, u16, u16, u16) {
    let area = chunk.area();
    if let Some(border) = border {
        if area.width() < 2 || area.height() < 2 {
            return (0, 0, 0, 0);
        }
        border_renderer::render_border(chunk, border, border_style);
        (1, 1, area.width() - 2, area.height() - 2)
    } else {
        (0, 0, area.width(), area.height())
    }
}

fn handle_scroll_event<M>(
    event: &Event,
    ctx: &mut EventCtx<M>,
    total_lines: usize,
    height: u16,
    border: bool,
) {
    if ctx.phase() != EventPhase::Target || total_lines == 0 {
        return;
    }
    let visible_rows = crate::widgets::pointer::visible_rows(ctx, border, height, 0);
    if visible_rows == 0 {
        return;
    }
    let state = ctx.state_mut::<ContentViewerState>();
    let max_offset = total_lines.saturating_sub(visible_rows);
    let old = state.scroll_offset.min(max_offset);
    state.scroll_offset = match event {
        Event::Key(key) => match key.code {
            KeyCode::Up => old.saturating_sub(1),
            KeyCode::Down => old.saturating_add(1).min(max_offset),
            KeyCode::PageUp => old.saturating_sub(visible_rows),
            KeyCode::PageDown => old.saturating_add(visible_rows).min(max_offset),
            KeyCode::Home => 0,
            KeyCode::End => max_offset,
            _ => return,
        },
        Event::Mouse(mouse) => match mouse.kind {
            MouseEventKind::ScrollUp => old.saturating_sub(3),
            MouseEventKind::ScrollDown => old.saturating_add(3).min(max_offset),
            _ => return,
        },
        _ => return,
    };
    if state.scroll_offset != old || matches!(event, Event::Key(_)) {
        ctx.set_handled();
    }
}

fn content_constraints(height: u16, border: Option<BorderStyle>, min_width: u16) -> Constraints {
    let border_size = if border.is_some() { 2 } else { 0 };
    Constraints {
        min_width: min_width + border_size,
        max_width: None,
        min_height: height,
        max_height: Some(height),
        flex: Some(1.0),
    }
}

fn measure_text_viewer(
    widths: impl Iterator<Item = usize>,
    height: u16,
    border: Option<BorderStyle>,
    proposal: SizeProposal,
    prefix_width: usize,
    min_width: u16,
) -> MeasuredSize {
    let border_size = if border.is_some() { 2 } else { 0 };
    let widest = widths
        .max()
        .unwrap_or(0)
        .saturating_add(prefix_width)
        .max(min_width as usize);
    let width = fit_axis(
        (widest.min(u16::MAX as usize) as u16).saturating_add(border_size),
        proposal.width,
    );
    let height = fit_axis(height, proposal.height);

    MeasuredSize::new(width, height)
}

fn code_line_number_width(line_count: usize, show_line_numbers: bool) -> usize {
    if show_line_numbers {
        line_count.max(1).to_string().width() + 2
    } else {
        0
    }
}

fn fit_axis(natural: u16, proposal: AxisLimit) -> u16 {
    match proposal {
        AxisLimit::Unbounded => natural,
        AxisLimit::AtMost(max) => natural.min(max),
        AxisLimit::Exact(value) => value,
    }
}

fn draw_diff_line(
    chunk: &mut rsille_render::chunk::Chunk,
    line: &str,
    fallback: rsille_render::style::Style,
) {
    let style = if line.starts_with('+') && !line.starts_with("+++") {
        Style::default()
            .fg(Color::Rgb(134, 239, 172))
            .bg(Color::Rgb(20, 83, 45))
            .to_render_style()
    } else if line.starts_with('-') && !line.starts_with("---") {
        Style::default()
            .fg(Color::Rgb(252, 165, 165))
            .bg(Color::Rgb(127, 29, 29))
            .to_render_style()
    } else if line.starts_with("@@") {
        Style::default()
            .fg(Color::Rgb(125, 211, 252))
            .bg(Color::Rgb(12, 74, 110))
            .to_render_style()
    } else {
        fallback
    };
    let _ = chunk.fill(0, 0, chunk.area().width(), 1, ' ', style);
    let _ = chunk.set_string(0, 0, line, style);
}

fn draw_markdown_line(
    chunk: &mut rsille_render::chunk::Chunk,
    line: &str,
    fallback: rsille_render::style::Style,
) {
    let trimmed = line.trim_start();
    let style = if trimmed.starts_with("# ") {
        Style::default()
            .fg(Color::Rgb(125, 211, 252))
            .bold()
            .to_render_style()
    } else if trimmed.starts_with("## ") || trimmed.starts_with("### ") {
        Style::default()
            .fg(Color::Rgb(147, 197, 253))
            .bold()
            .to_render_style()
    } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        Style::default()
            .fg(Color::Rgb(226, 232, 240))
            .to_render_style()
    } else if trimmed.starts_with("```") || trimmed.starts_with('`') {
        Style::default()
            .fg(Color::Rgb(196, 181, 253))
            .bg(Color::Rgb(39, 39, 42))
            .to_render_style()
    } else if trimmed.starts_with('>') {
        Style::default()
            .fg(Color::Rgb(148, 163, 184))
            .italic()
            .to_render_style()
    } else {
        fallback
    };

    let _ = chunk.set_string(0, 0, line, style);
}

fn log_style(level: LogLevel) -> Style {
    match level {
        LogLevel::Trace => Style::default().fg(Color::Rgb(148, 163, 184)),
        LogLevel::Debug => Style::default().fg(Color::Rgb(147, 197, 253)),
        LogLevel::Info => Style::default().fg(Color::Rgb(226, 232, 240)),
        LogLevel::Warn => Style::default().fg(Color::Rgb(253, 224, 71)),
        LogLevel::Error => Style::default().fg(Color::Rgb(252, 165, 165)).bold(),
    }
}

fn truncate_to_width(
    text: &str,
    max_width: usize,
    metrics: rsille_render::text::TextMetrics,
) -> String {
    metrics.clip(text, 0, max_width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Theme;
    use crate::widget::WidgetStore;

    #[test]
    fn markdown_viewer_measure_uses_content_width_and_configured_height() {
        let viewer = MarkdownViewer::<()>::new("short\nthis line is definitely wider than minimum")
            .height(6);
        let store = WidgetStore::new();
        let theme = Theme::dark();
        let ctx = MeasureCtx::new(&store, &theme);

        let measured = viewer.measure(SizeProposal::UNBOUNDED, &ctx);

        assert!(measured.width > 26);
        assert_eq!(measured.height, 6);
    }

    #[test]
    fn code_viewer_measure_accounts_for_line_numbers() {
        let viewer = CodeViewer::<()>::new("a\nabc").height(4);
        let store = WidgetStore::new();
        let theme = Theme::dark();
        let ctx = MeasureCtx::new(&store, &theme);

        let measured = viewer.measure(SizeProposal::UNBOUNDED, &ctx);

        assert_eq!(measured.width, 26);
        assert_eq!(measured.height, 4);
    }
}

/// Creates a [`CodeViewer`] for source text.
pub fn code_viewer<M>(code: impl Into<String>) -> CodeViewer<M> {
    CodeViewer::new(code)
}

/// Creates an empty [`LogViewer`].
pub fn log_viewer<M>() -> LogViewer<M> {
    LogViewer::new()
}

/// Creates a [`DiffViewer`] for precomputed diff text.
pub fn diff_viewer<M>(diff: impl Into<String>) -> DiffViewer<M> {
    DiffViewer::new(diff)
}

/// Creates a [`MarkdownViewer`] for Markdown source text.
pub fn markdown_viewer<M>(markdown: impl Into<String>) -> MarkdownViewer<M> {
    MarkdownViewer::new(markdown)
}
