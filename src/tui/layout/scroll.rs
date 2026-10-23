//! Scroll containers and reusable scroll helpers.

use crate::render::area::Area;
use crate::render::text::TextWidth;
use crate::tui::layout::{ChildLayout, Layout};

use super::border_renderer::{render_background, render_border};
use crate::tui::event::{Event, KeyCode, MouseEventKind};
use crate::tui::focus::FocusConfig;
use crate::tui::layout::{AxisLimit, Constraints, MeasuredSize, SizeProposal};
use crate::tui::offscreen::{blit_region, render_to_offscreen, BlitOptions, BlitRegion};
use crate::tui::style::{BorderStyle, Padding, Style};
use crate::tui::widget::{
    EventCtx, EventPhase, IntoWidget, MeasureCtx, RenderCtx, Widget, WidgetKey,
};

/// Axis support for scrollable containers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollAxis {
    Horizontal,
    #[default]
    Vertical,
    Both,
}

impl ScrollAxis {
    fn allows_horizontal(self) -> bool {
        matches!(self, Self::Horizontal | Self::Both)
    }

    fn allows_vertical(self) -> bool {
        matches!(self, Self::Vertical | Self::Both)
    }
}

/// Controls when a scrollbar should be rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollbarVisibility {
    Hidden,
    #[default]
    Auto,
    Always,
}

/// Persistent scroll position state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScrollState {
    pub offset_x: usize,
    pub offset_y: usize,
}

impl ScrollState {
    pub fn clamp(
        &mut self,
        content_width: usize,
        content_height: usize,
        viewport_width: usize,
        viewport_height: usize,
    ) {
        self.offset_x = clamp_scroll_offset(self.offset_x, content_width, viewport_width);
        self.offset_y = clamp_scroll_offset(self.offset_y, content_height, viewport_height);
    }

    pub fn scroll_by(
        &mut self,
        delta_x: isize,
        delta_y: isize,
        content_width: usize,
        content_height: usize,
        viewport_width: usize,
        viewport_height: usize,
    ) {
        self.offset_x = apply_scroll_delta(self.offset_x, delta_x, content_width, viewport_width);
        self.offset_y = apply_scroll_delta(self.offset_y, delta_y, content_height, viewport_height);
    }

    pub fn scroll_to_offset(
        &mut self,
        offset_x: usize,
        offset_y: usize,
        content_width: usize,
        content_height: usize,
        viewport_width: usize,
        viewport_height: usize,
    ) {
        self.offset_x = clamp_scroll_offset(offset_x, content_width, viewport_width);
        self.offset_y = clamp_scroll_offset(offset_y, content_height, viewport_height);
    }

    pub fn scroll_to_item_y(
        &mut self,
        index: usize,
        item_height: usize,
        viewport_height: usize,
        content_height: usize,
    ) {
        self.offset_y = scroll_offset_for_item(
            self.offset_y,
            index,
            item_height,
            content_height,
            viewport_height,
        );
    }
}

/// Maximum offset allowed for the given content and viewport sizes.
pub fn max_scroll_offset(content_size: usize, viewport_size: usize) -> usize {
    content_size.saturating_sub(viewport_size)
}

/// Clamp a scroll offset into the valid range.
pub fn clamp_scroll_offset(offset: usize, content_size: usize, viewport_size: usize) -> usize {
    offset.min(max_scroll_offset(content_size, viewport_size))
}

/// Ensure a row-like item is visible inside a viewport.
pub fn ensure_item_visible(scroll_offset: usize, item_index: usize, visible_items: usize) -> usize {
    scroll_offset_for_item(scroll_offset, item_index, 1, item_index + 1, visible_items)
}

/// Compute the offset needed to reveal an item of fixed extent.
pub fn scroll_offset_for_item(
    scroll_offset: usize,
    item_index: usize,
    item_extent: usize,
    content_extent: usize,
    viewport_extent: usize,
) -> usize {
    if viewport_extent == 0 {
        return 0;
    }

    let item_start = item_index.saturating_mul(item_extent.max(1));
    let item_end = (item_start + item_extent.max(1)).min(content_extent.max(item_start + 1));

    if item_start < scroll_offset {
        item_start
    } else if item_end > scroll_offset + viewport_extent {
        item_end.saturating_sub(viewport_extent)
    } else {
        scroll_offset
    }
}

fn apply_scroll_delta(
    offset: usize,
    delta: isize,
    content_size: usize,
    viewport_size: usize,
) -> usize {
    let max_offset = max_scroll_offset(content_size, viewport_size) as isize;
    let next = (offset as isize + delta).clamp(0, max_offset);
    next as usize
}

/// Scrollbar direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollbarOrientation {
    Horizontal,
    Vertical,
}

/// Standalone scrollbar widget.
pub struct Scrollbar {
    orientation: ScrollbarOrientation,
    content_size: usize,
    viewport_size: usize,
    offset: usize,
    track_style: Option<Style>,
    thumb_style: Option<Style>,
}

impl std::fmt::Debug for Scrollbar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scrollbar")
            .field("orientation", &self.orientation)
            .field("content_size", &self.content_size)
            .field("viewport_size", &self.viewport_size)
            .field("offset", &self.offset)
            .finish()
    }
}

impl Scrollbar {
    pub fn new(orientation: ScrollbarOrientation) -> Self {
        Self {
            orientation,
            content_size: 0,
            viewport_size: 0,
            offset: 0,
            track_style: None,
            thumb_style: None,
        }
    }

    pub fn vertical() -> Self {
        Self::new(ScrollbarOrientation::Vertical)
    }

    pub fn horizontal() -> Self {
        Self::new(ScrollbarOrientation::Horizontal)
    }

    pub fn content_size(mut self, content_size: usize) -> Self {
        self.content_size = content_size;
        self
    }

    pub fn viewport_size(mut self, viewport_size: usize) -> Self {
        self.viewport_size = viewport_size;
        self
    }

    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    pub fn track_style(mut self, style: Style) -> Self {
        self.track_style = Some(style);
        self
    }

    pub fn thumb_style(mut self, style: Style) -> Self {
        self.thumb_style = Some(style);
        self
    }

    fn metrics(&self, track_len: usize) -> Option<(usize, usize)> {
        if track_len == 0 || self.viewport_size == 0 || self.content_size <= self.viewport_size {
            return None;
        }

        let thumb_len = ((self.viewport_size * track_len) / self.content_size)
            .max(1)
            .min(track_len);
        let travel = track_len.saturating_sub(thumb_len);
        let max_offset = max_scroll_offset(self.content_size, self.viewport_size);
        let thumb_offset = if max_offset == 0 || travel == 0 {
            0
        } else {
            (self.offset.min(max_offset) * travel) / max_offset
        };

        Some((thumb_offset, thumb_len))
    }
}

impl<M> Widget<M> for Scrollbar {
    fn render(&self, chunk: &mut crate::render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        let track_style = self
            .track_style
            .unwrap_or(ctx.theme().styles.scrollbar_track)
            .to_render_style();
        let thumb_style = self
            .thumb_style
            .unwrap_or(ctx.theme().styles.scrollbar_thumb)
            .to_render_style();

        let track_len = match self.orientation {
            ScrollbarOrientation::Vertical => area.height() as usize,
            ScrollbarOrientation::Horizontal => area.width() as usize,
        };

        match self.orientation {
            ScrollbarOrientation::Vertical => {
                let _ = chunk.fill(0, 0, area.width(), area.height(), '│', track_style);
                if let Some((thumb_offset, thumb_len)) = self.metrics(track_len) {
                    for index in thumb_offset..thumb_offset + thumb_len {
                        let _ = chunk.set_char(0, index as u16, '█', thumb_style);
                    }
                }
            }
            ScrollbarOrientation::Horizontal => {
                let _ = chunk.fill(0, 0, area.width(), area.height(), '─', track_style);
                if let Some((thumb_offset, thumb_len)) = self.metrics(track_len) {
                    for index in thumb_offset..thumb_offset + thumb_len {
                        let _ = chunk.set_char(index as u16, 0, '█', thumb_style);
                    }
                }
            }
        }
    }

    fn constraints(&self) -> Constraints {
        match self.orientation {
            ScrollbarOrientation::Vertical => {
                Constraints::fixed(1, self.viewport_size.max(1) as u16)
            }
            ScrollbarOrientation::Horizontal => {
                Constraints::fixed(self.viewport_size.max(1) as u16, 1)
            }
        }
    }
}

/// Create a vertical scrollbar widget.
pub fn scrollbar() -> Scrollbar {
    Scrollbar::vertical()
}

#[derive(Debug, Clone, Copy)]
struct ScrollMetrics {
    viewport: Area,
    content_width: usize,
    content_height: usize,
    show_horizontal_bar: bool,
    show_vertical_bar: bool,
}

/// Generic scrollable viewport container.
pub struct ScrollView<M = ()> {
    child: Box<dyn Widget<M>>,
    axis: ScrollAxis,
    border: Option<BorderStyle>,
    padding: Padding,
    style: Style,
    widget_key: Option<String>,
    content_width: Option<u16>,
    content_height: Option<u16>,
    scrollbar_visibility: ScrollbarVisibility,
    focusable: bool,
    scroll_step: u16,
}

impl<M> std::fmt::Debug for ScrollView<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScrollView")
            .field("axis", &self.axis)
            .field("border", &self.border)
            .field("padding", &self.padding)
            .field("style", &self.style)
            .field("content_width", &self.content_width)
            .field("content_height", &self.content_height)
            .field("scrollbar_visibility", &self.scrollbar_visibility)
            .field("focusable", &self.focusable)
            .field("scroll_step", &self.scroll_step)
            .finish()
    }
}

impl<M> ScrollView<M> {
    pub fn new(child: impl IntoWidget<M>) -> Self {
        Self {
            child: child.into_widget(),
            axis: ScrollAxis::Vertical,
            border: None,
            padding: Padding::ZERO,
            style: Style::default(),
            widget_key: None,
            content_width: None,
            content_height: None,
            scrollbar_visibility: ScrollbarVisibility::Auto,
            focusable: false,
            scroll_step: 3,
        }
    }

    pub fn axis(mut self, axis: ScrollAxis) -> Self {
        self.axis = axis;
        self
    }

    pub fn vertical(mut self) -> Self {
        self.axis = ScrollAxis::Vertical;
        self
    }

    pub fn horizontal(mut self) -> Self {
        self.axis = ScrollAxis::Horizontal;
        self
    }

    pub fn both(mut self) -> Self {
        self.axis = ScrollAxis::Both;
        self
    }

    pub fn border(mut self, border: BorderStyle) -> Self {
        self.border = Some(border);
        self
    }

    pub fn borderless(mut self) -> Self {
        self.border = None;
        self
    }

    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn content_width(mut self, width: u16) -> Self {
        self.content_width = Some(width);
        self
    }

    pub fn content_height(mut self, height: u16) -> Self {
        self.content_height = Some(height);
        self
    }

    pub fn scrollbars(mut self, visibility: ScrollbarVisibility) -> Self {
        self.scrollbar_visibility = visibility;
        self
    }

    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }

    pub fn scroll_step(mut self, step: u16) -> Self {
        self.scroll_step = step.max(1);
        self
    }

    fn resolve_metrics(
        &self,
        outer: Area,
        measure_ctx: Option<&MeasureCtx>,
    ) -> Option<ScrollMetrics> {
        let border_area = if self.border.is_some() {
            if outer.width() < 2 || outer.height() < 2 {
                return None;
            }
            Area::new(
                (outer.x() + 1, outer.y() + 1).into(),
                (outer.width() - 2, outer.height() - 2).into(),
            )
        } else {
            outer
        };

        let inner = border_area.shrink_saturating(
            self.padding.top,
            self.padding.bottom,
            self.padding.left,
            self.padding.right,
        );
        if inner.width() == 0 || inner.height() == 0 {
            return None;
        }

        let mut viewport = inner;
        let mut show_vertical_bar = false;
        let mut show_horizontal_bar = false;

        for _ in 0..3 {
            let (content_width, content_height) = self.resolve_content_size(viewport, measure_ctx);
            let content_width = content_width.max(viewport.width() as usize);
            let content_height = content_height.max(viewport.height() as usize);

            let next_vertical_bar = self.axis.allows_vertical()
                && (matches!(self.scrollbar_visibility, ScrollbarVisibility::Always)
                    || (matches!(self.scrollbar_visibility, ScrollbarVisibility::Auto)
                        && content_height > viewport.height() as usize));
            let next_horizontal_bar = self.axis.allows_horizontal()
                && (matches!(self.scrollbar_visibility, ScrollbarVisibility::Always)
                    || (matches!(self.scrollbar_visibility, ScrollbarVisibility::Auto)
                        && content_width > viewport.width() as usize));

            if next_vertical_bar == show_vertical_bar && next_horizontal_bar == show_horizontal_bar
            {
                return Some(ScrollMetrics {
                    viewport,
                    content_width,
                    content_height,
                    show_horizontal_bar,
                    show_vertical_bar,
                });
            }

            show_vertical_bar = next_vertical_bar;
            show_horizontal_bar = next_horizontal_bar;
            viewport = inner.shrink_saturating(
                0,
                if show_horizontal_bar { 1 } else { 0 },
                0,
                if show_vertical_bar { 1 } else { 0 },
            );
        }

        let (content_width, content_height) = self.resolve_content_size(viewport, measure_ctx);
        Some(ScrollMetrics {
            viewport,
            content_width: content_width.max(viewport.width() as usize),
            content_height: content_height.max(viewport.height() as usize),
            show_horizontal_bar,
            show_vertical_bar,
        })
    }

    fn resolve_content_size(
        &self,
        viewport: Area,
        measure_ctx: Option<&MeasureCtx>,
    ) -> (usize, usize) {
        if let Some(ctx) = measure_ctx {
            let width = self.content_width.map(AxisLimit::Exact).unwrap_or_else(|| {
                if self.axis.allows_horizontal() {
                    AxisLimit::Unbounded
                } else {
                    AxisLimit::Exact(viewport.width())
                }
            });
            let height = self
                .content_height
                .map(AxisLimit::Exact)
                .unwrap_or_else(|| {
                    if self.axis.allows_vertical() {
                        AxisLimit::Unbounded
                    } else {
                        AxisLimit::Exact(viewport.height())
                    }
                });
            let child_ctx = ctx.child_ctx(WidgetKey::for_child(0, self.child.as_ref()));
            let measured = self
                .child
                .measure(SizeProposal { width, height }, &child_ctx);
            return (
                usize::from(self.content_width.unwrap_or(measured.width)),
                usize::from(self.content_height.unwrap_or(measured.height)),
            );
        }

        let child_constraints = self.child.constraints();
        (
            usize::from(self.content_width.unwrap_or(child_constraints.min_width)),
            usize::from(self.content_height.unwrap_or(child_constraints.min_height)),
        )
    }

    fn copy_visible_region(
        &self,
        target: &mut crate::render::chunk::Chunk,
        viewport: Area,
        source: &crate::render::buffer::Buffer,
        offset_x: usize,
        offset_y: usize,
    ) {
        let target_area = target.area();
        blit_region(
            target,
            source,
            BlitRegion::new(
                offset_x,
                offset_y,
                viewport.x() as i32 - target_area.x() as i32,
                viewport.y() as i32 - target_area.y() as i32,
                viewport.width() as usize,
                viewport.height() as usize,
            ),
            BlitOptions::default(),
        );
    }
}

impl<M> Widget<M> for ScrollView<M> {
    fn layout(&self, area: Area, ctx: &MeasureCtx) -> Layout {
        let Some(metrics) = self.resolve_metrics(area, Some(ctx)) else {
            return Layout::default();
        };
        let content = Area::new(
            (0, 0).into(),
            (
                metrics.content_width.min(u16::MAX as usize) as u16,
                metrics.content_height.min(u16::MAX as usize) as u16,
            )
                .into(),
        );
        Layout::new([ChildLayout::new(0, content)]).with_data(metrics)
    }

    fn render(&self, chunk: &mut crate::render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        ctx.record_bounds(area);

        let surface_style = self.style.merge(ctx.theme().styles.surface);
        let border_style = self
            .style
            .merge(ctx.theme().styles.border.merge(ctx.theme().styles.surface));
        let render_style = surface_style.to_render_style();
        let border_render_style = border_style.to_render_style();
        let should_fill_background = ctx.path().is_empty() || self.style.bg_color.is_some();

        if should_fill_background {
            render_background(chunk, render_style);
        }

        if let Some(border) = self.border {
            render_border(chunk, border, border_render_style);
        }

        let result = ctx.layout(self, area);
        let Some(metrics) = result.data::<ScrollMetrics>() else {
            return;
        };
        if metrics.viewport.width() == 0 || metrics.viewport.height() == 0 {
            return;
        }

        let mut scroll_state = *ctx.state_or_default::<ScrollState>();
        scroll_state.clamp(
            metrics.content_width,
            metrics.content_height,
            metrics.viewport.width() as usize,
            metrics.viewport.height() as usize,
        );

        let offscreen_size: crate::render::area::Size = (
            metrics.content_width.min(u16::MAX as usize) as u16,
            metrics.content_height.min(u16::MAX as usize) as u16,
        )
            .into();
        let child_ctx = ctx
            .child_ctx(WidgetKey::for_child(0, self.child.as_ref()))
            .with_viewport(metrics.viewport)
            .with_coordinate_offset(
                metrics.viewport.x() as i32 - scroll_state.offset_x.min(i32::MAX as usize) as i32,
                metrics.viewport.y() as i32 - scroll_state.offset_y.min(i32::MAX as usize) as i32,
            );
        let content_area = Area::new((0, 0).into(), offscreen_size);
        child_ctx.record_bounds(content_area);
        child_ctx.record_hit_bounds(
            content_area,
            content_area,
            crate::tui::animation::HitTestMode::Target,
        );
        let Some(offscreen) = render_to_offscreen(
            Area::new((0, 0).into(), offscreen_size),
            |offscreen_chunk| {
                offscreen_chunk.set_text_metrics(ctx.theme().text_metrics);
                self.child.render(offscreen_chunk, &child_ctx);
            },
        ) else {
            return;
        };
        self.copy_visible_region(
            chunk,
            metrics.viewport,
            &offscreen,
            scroll_state.offset_x,
            scroll_state.offset_y,
        );

        if metrics.show_vertical_bar {
            let scrollbar_area = Area::new(
                (
                    metrics.viewport.x() + metrics.viewport.width(),
                    metrics.viewport.y(),
                )
                    .into(),
                (1, metrics.viewport.height()).into(),
            );
            if let Ok(mut scrollbar_chunk) = chunk.from_area(scrollbar_area) {
                let scrollbar = Scrollbar::vertical()
                    .content_size(metrics.content_height)
                    .viewport_size(metrics.viewport.height() as usize)
                    .offset(scroll_state.offset_y);
                <Scrollbar as Widget<M>>::render(&scrollbar, &mut scrollbar_chunk, ctx);
            }
        }

        if metrics.show_horizontal_bar {
            let scrollbar_area = Area::new(
                (
                    metrics.viewport.x(),
                    metrics.viewport.y() + metrics.viewport.height(),
                )
                    .into(),
                (metrics.viewport.width(), 1).into(),
            );
            if let Ok(mut scrollbar_chunk) = chunk.from_area(scrollbar_area) {
                let scrollbar = Scrollbar::horizontal()
                    .content_size(metrics.content_width)
                    .viewport_size(metrics.viewport.width() as usize)
                    .offset(scroll_state.offset_x);
                <Scrollbar as Widget<M>>::render(&scrollbar, &mut scrollbar_chunk, ctx);
            }
        }

        if metrics.show_vertical_bar && metrics.show_horizontal_bar {
            let corner_area = Area::new(
                (
                    metrics.viewport.x() + metrics.viewport.width(),
                    metrics.viewport.y() + metrics.viewport.height(),
                )
                    .into(),
                (1, 1).into(),
            );
            if let Ok(mut corner_chunk) = chunk.from_area(corner_area) {
                let style = ctx
                    .theme()
                    .styles
                    .border
                    .merge(ctx.theme().styles.surface)
                    .to_render_style();
                let _ = corner_chunk.set_char(0, 0, '┘', style);
            }
        }
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if crate::tui::event::is_key_release(event) {
            return;
        }
        if !matches!(ctx.phase(), EventPhase::Target | EventPhase::Bubble) {
            return;
        }
        if ctx.phase() == EventPhase::Bubble && ctx.was_handled() {
            return;
        }

        let Some(bounds) = ctx.bounds() else {
            return;
        };
        let metrics = {
            let measure_ctx = ctx.measure_ctx();
            let Some(metrics) = self.resolve_metrics(bounds, measure_ctx.as_ref()) else {
                return;
            };
            metrics
        };

        let viewport_width = metrics.viewport.width() as usize;
        let viewport_height = metrics.viewport.height() as usize;
        let line_step = self.scroll_step as isize;
        let page_y = viewport_height.max(1) as isize;

        let state = ctx.state_mut::<ScrollState>();
        state.clamp(
            metrics.content_width,
            metrics.content_height,
            viewport_width,
            viewport_height,
        );

        let mut handled = false;

        match event {
            Event::Key(key_event) => match key_event.code {
                KeyCode::Up if self.axis.allows_vertical() => {
                    state.scroll_by(
                        0,
                        -line_step,
                        metrics.content_width,
                        metrics.content_height,
                        viewport_width,
                        viewport_height,
                    );
                    handled = true;
                }
                KeyCode::Down if self.axis.allows_vertical() => {
                    state.scroll_by(
                        0,
                        line_step,
                        metrics.content_width,
                        metrics.content_height,
                        viewport_width,
                        viewport_height,
                    );
                    handled = true;
                }
                KeyCode::Left if self.axis.allows_horizontal() => {
                    state.scroll_by(
                        -line_step,
                        0,
                        metrics.content_width,
                        metrics.content_height,
                        viewport_width,
                        viewport_height,
                    );
                    handled = true;
                }
                KeyCode::Right if self.axis.allows_horizontal() => {
                    state.scroll_by(
                        line_step,
                        0,
                        metrics.content_width,
                        metrics.content_height,
                        viewport_width,
                        viewport_height,
                    );
                    handled = true;
                }
                KeyCode::PageUp if self.axis.allows_vertical() => {
                    state.scroll_by(
                        0,
                        -page_y,
                        metrics.content_width,
                        metrics.content_height,
                        viewport_width,
                        viewport_height,
                    );
                    handled = true;
                }
                KeyCode::PageDown if self.axis.allows_vertical() => {
                    state.scroll_by(
                        0,
                        page_y,
                        metrics.content_width,
                        metrics.content_height,
                        viewport_width,
                        viewport_height,
                    );
                    handled = true;
                }
                KeyCode::Home => {
                    state.scroll_to_offset(
                        0,
                        0,
                        metrics.content_width,
                        metrics.content_height,
                        viewport_width,
                        viewport_height,
                    );
                    handled = true;
                }
                KeyCode::End => {
                    state.scroll_to_offset(
                        if self.axis.allows_horizontal() {
                            max_scroll_offset(metrics.content_width, viewport_width)
                        } else {
                            state.offset_x
                        },
                        if self.axis.allows_vertical() {
                            max_scroll_offset(metrics.content_height, viewport_height)
                        } else {
                            state.offset_y
                        },
                        metrics.content_width,
                        metrics.content_height,
                        viewport_width,
                        viewport_height,
                    );
                    handled = true;
                }
                _ => {}
            },
            Event::Mouse(mouse_event) => match mouse_event.kind {
                MouseEventKind::ScrollUp if self.axis.allows_vertical() => {
                    state.scroll_by(
                        0,
                        -line_step,
                        metrics.content_width,
                        metrics.content_height,
                        viewport_width,
                        viewport_height,
                    );
                    handled = true;
                }
                MouseEventKind::ScrollDown if self.axis.allows_vertical() => {
                    state.scroll_by(
                        0,
                        line_step,
                        metrics.content_width,
                        metrics.content_height,
                        viewport_width,
                        viewport_height,
                    );
                    handled = true;
                }
                MouseEventKind::ScrollLeft if self.axis.allows_horizontal() => {
                    state.scroll_by(
                        -line_step,
                        0,
                        metrics.content_width,
                        metrics.content_height,
                        viewport_width,
                        viewport_height,
                    );
                    handled = true;
                }
                MouseEventKind::ScrollRight if self.axis.allows_horizontal() => {
                    state.scroll_by(
                        line_step,
                        0,
                        metrics.content_width,
                        metrics.content_height,
                        viewport_width,
                        viewport_height,
                    );
                    handled = true;
                }
                _ => {}
            },
            _ => {}
        }

        if handled {
            ctx.set_handled();
        }
    }

    fn constraints(&self) -> Constraints {
        let child = self.child.constraints();
        let border_size = if self.border.is_some() { 2 } else { 0 };
        let scrollbar_cross = match self.scrollbar_visibility {
            ScrollbarVisibility::Hidden => 0,
            ScrollbarVisibility::Auto | ScrollbarVisibility::Always => 1,
        };
        let min_width = self
            .content_width
            .unwrap_or(child.min_width)
            .saturating_add(self.padding.horizontal_total())
            .saturating_add(border_size)
            .saturating_add(if self.axis.allows_vertical() {
                scrollbar_cross
            } else {
                0
            });
        let min_height = self
            .content_height
            .unwrap_or(child.min_height)
            .saturating_add(self.padding.vertical_total())
            .saturating_add(border_size)
            .saturating_add(if self.axis.allows_horizontal() {
                scrollbar_cross
            } else {
                0
            });

        Constraints {
            min_width,
            max_width: None,
            min_height,
            max_height: None,
            flex: Some(1.0),
        }
    }

    fn measure(&self, proposal: SizeProposal, ctx: &MeasureCtx) -> MeasuredSize {
        let chrome_width =
            self.padding.horizontal_total() + if self.border.is_some() { 2 } else { 0 };
        let chrome_height =
            self.padding.vertical_total() + if self.border.is_some() { 2 } else { 0 };

        let available_width = proposal.width.exact_or_at_most().unwrap_or_else(|| {
            self.content_width
                .unwrap_or_else(|| {
                    let child_ctx = ctx.child_ctx(WidgetKey::for_child(0, self.child.as_ref()));
                    self.child
                        .measure(SizeProposal::UNBOUNDED, &child_ctx)
                        .width
                })
                .saturating_add(chrome_width)
        });
        let available_height = proposal.height.exact_or_at_most().unwrap_or_else(|| {
            self.content_height
                .unwrap_or_else(|| {
                    let child_ctx = ctx.child_ctx(WidgetKey::for_child(0, self.child.as_ref()));
                    self.child
                        .measure(SizeProposal::UNBOUNDED, &child_ctx)
                        .height
                })
                .saturating_add(chrome_height)
        });

        let outer = Area::new((0, 0).into(), (available_width, available_height).into());
        let Some(metrics) = self.resolve_metrics(outer, Some(ctx)) else {
            return self.layout_style().resolve_fallback_size(proposal);
        };

        let scrollbar_width = u16::from(metrics.show_vertical_bar);
        let scrollbar_height = u16::from(metrics.show_horizontal_bar);
        let natural_width = (metrics.content_width.min(u16::MAX as usize) as u16)
            .saturating_add(chrome_width)
            .saturating_add(scrollbar_width);
        let natural_height = (metrics.content_height.min(u16::MAX as usize) as u16)
            .saturating_add(chrome_height)
            .saturating_add(scrollbar_height);

        self.layout_style().clamp_size(MeasuredSize::new(
            fit_axis(natural_width, proposal.width),
            fit_axis(natural_height, proposal.height),
        ))
    }

    fn children(&self) -> &[Box<dyn Widget<M>>] {
        std::slice::from_ref(&self.child)
    }

    fn focus_config(&self) -> FocusConfig {
        if self.focusable {
            FocusConfig::Composite
        } else {
            FocusConfig::None
        }
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

/// Create a new scrollable container around a single child widget.
pub fn scroll_view<M>(child: impl IntoWidget<M>) -> ScrollView<M> {
    ScrollView::new(child)
}

/// Build a simple line-based scroll view from strings.
pub fn scroll_lines<M, I, S>(lines: I) -> ScrollView<M>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
    M: 'static,
{
    let lines: Vec<String> = lines.into_iter().map(Into::into).collect();
    let content_width = lines
        .iter()
        .map(|line| line.width() as u16)
        .max()
        .unwrap_or(0);
    let content_height = lines.len() as u16;
    let mut content = crate::tui::layout::col::<M>().gap(0);
    for line in lines {
        content = content.child(crate::tui::widgets::label(line));
    }

    ScrollView::new(content)
        .both()
        .content_width(content_width)
        .content_height(content_height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    use crate::render::buffer::Buffer;
    use crate::render::chunk::Chunk;
    use crate::tui::animation::AnimationStore;
    use crate::tui::event::{KeyEvent, KeyModifiers};
    use crate::tui::style::Theme;
    use crate::tui::widget::{
        EventCtx, MeasureCtx, RenderCtx, Widget, WidgetId, WidgetPath, WidgetStore,
    };
    use crate::tui::widgets::visual::TerminalVisualCapabilities;
    use crate::tui::widgets::{label, TextWrap};

    #[test]
    fn clamp_offset_stays_in_range() {
        assert_eq!(clamp_scroll_offset(9, 20, 5), 9);
        assert_eq!(clamp_scroll_offset(99, 20, 5), 15);
        assert_eq!(clamp_scroll_offset(3, 4, 10), 0);
    }

    #[test]
    fn item_visibility_moves_minimally() {
        assert_eq!(ensure_item_visible(0, 2, 5), 0);
        assert_eq!(ensure_item_visible(0, 5, 5), 1);
        assert_eq!(ensure_item_visible(4, 2, 5), 2);
    }

    #[test]
    fn scroll_view_measures_wrapped_child_content_against_viewport_width() {
        let child = label::<()>("alpha beta gamma delta").wrap(TextWrap::Word);
        let scroll = ScrollView::new(child).vertical();
        let store = WidgetStore::new();
        let theme = Theme::dark();
        let ctx = MeasureCtx::new(&store, &theme);

        let measured = scroll.measure(SizeProposal::at_most(8, 3), &ctx);
        let metrics = scroll
            .resolve_metrics(Area::new((0, 0).into(), (8, 3).into()), Some(&ctx))
            .expect("scroll metrics");

        assert_eq!(measured, MeasuredSize::new(8, 3));
        assert!(metrics.content_height > metrics.viewport.height() as usize);
        assert!(metrics.show_vertical_bar);
    }

    #[test]
    fn scroll_view_event_uses_measurement_for_wrapped_child_content() {
        let child = label::<()>("alpha beta gamma delta").wrap(TextWrap::Word);
        let scroll = ScrollView::new(child).vertical().scroll_step(1);
        let mut store = WidgetStore::new();
        let animation_store = AnimationStore::new();
        let theme = Theme::dark();
        let geometry = RefCell::new(HashMap::<WidgetPath, Area>::new());
        let mut buffer = Buffer::new((8, 3).into());
        let area = Area::new((0, 0).into(), (8, 3).into());

        {
            let mut chunk = Chunk::new(&mut buffer, area).expect("chunk");
            let render_ctx = RenderCtx::new(&store, &animation_store, &theme, None, &geometry);
            scroll.render(&mut chunk, &render_ctx);
        }

        let geometry = geometry.borrow();
        let mut messages = Vec::new();
        let event = Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        let mut event_ctx = EventCtx::with_runtime(
            &mut store,
            &mut messages,
            WidgetPath::root(),
            WidgetId::root(),
            WidgetId::root(),
            None,
            &geometry,
            EventPhase::Target,
            false,
            &theme,
            std::time::Instant::now(),
            0,
            TerminalVisualCapabilities::default(),
        );

        scroll.handle_event(&event, &mut event_ctx);
        let outcome = event_ctx.finish();

        assert!(outcome.handled);
        let state = store
            .get::<ScrollState>(WidgetId::root())
            .expect("scroll state");
        assert_eq!(state.offset_y, 1);
    }

    #[test]
    fn both_axis_scrollbar_feedback_shrinks_viewport_on_second_pass() {
        let child = label::<()>("0123456789\n0123456789\n0123456789\n0123456789\n0123456789");
        let scroll = ScrollView::new(child).both();
        let store = WidgetStore::new();
        let theme = Theme::dark();
        let ctx = MeasureCtx::new(&store, &theme);

        let metrics = scroll
            .resolve_metrics(Area::new((0, 0).into(), (8, 4).into()), Some(&ctx))
            .expect("scroll metrics");

        assert!(metrics.show_horizontal_bar);
        assert!(metrics.show_vertical_bar);
        assert_eq!(metrics.viewport.width(), 7);
        assert_eq!(metrics.viewport.height(), 3);
    }
}

fn fit_axis(natural: u16, proposal: AxisLimit) -> u16 {
    match proposal {
        AxisLimit::Unbounded => natural,
        AxisLimit::AtMost(max) => natural.min(max),
        AxisLimit::Exact(value) => value,
    }
}
