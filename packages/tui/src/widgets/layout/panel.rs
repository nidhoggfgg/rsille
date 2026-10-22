//! Panel container widget.

use crate::layout::{ChildLayout, Layout};
use render::area::Area;

use crate::event::Event;
use crate::layout::border_renderer;
use crate::layout::taffy_bridge::TaffyBridge;
use crate::layout::{AxisLimit, Constraints, Direction, MeasuredSize, SizeProposal};
use crate::style::{BorderStyle, Padding, Style};
use crate::widget::{EventCtx, IntoWidget, MeasureCtx, RenderCtx, Widget, WidgetKey};

/// Bordered container for grouping related content.
pub struct Panel<M = ()> {
    children: Vec<Box<dyn Widget<M>>>,
    title: Option<String>,
    border: Option<BorderStyle>,
    padding: Padding,
    gap: u16,
    style: Style,
    widget_key: Option<String>,
}

impl<M> std::fmt::Debug for Panel<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Panel")
            .field("children", &self.children.len())
            .field("title", &self.title)
            .field("border", &self.border)
            .field("padding", &self.padding)
            .field("gap", &self.gap)
            .field("style", &self.style)
            .finish()
    }
}

impl<M> Panel<M> {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            title: None,
            border: Some(BorderStyle::Single),
            padding: Padding::uniform(1),
            gap: 0,
            style: Style::default(),
            widget_key: None,
        }
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
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

    pub fn gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn child(mut self, widget: impl IntoWidget<M>) -> Self {
        self.children.push(widget.into_widget());
        self
    }

    pub fn children<I>(mut self, widgets: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoWidget<M>,
    {
        self.children
            .extend(widgets.into_iter().map(IntoWidget::into_widget));
        self
    }
}

impl<M> Default for Panel<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> Widget<M> for Panel<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        ctx.record_bounds(area);

        let surface_style = self.style.merge(ctx.theme().styles.surface_elevated);
        let border_style = ctx.theme().styles.border.merge(surface_style);
        let render_style = surface_style.to_render_style();
        let border_render_style = border_style.to_render_style();
        let _ = chunk.fill(0, 0, area.width(), area.height(), ' ', render_style);

        if let Some(border) = self.border {
            if area.width() < 2 || area.height() < 2 {
                return;
            }
            border_renderer::render_border(chunk, border, border_render_style);
            if let Some(title) = self.title.as_ref() {
                let available = area.width().saturating_sub(4) as usize;
                let display = truncate_to_width(title, available, chunk.text_metrics());
                if !display.is_empty() {
                    let _ = chunk.set_string(2, 0, &display, border_render_style);
                }
            }
        }

        ctx.render_children(chunk, self);
    }

    fn layout(&self, area: Area, ctx: &MeasureCtx) -> Layout {
        let inner = if self.border.is_some() {
            if area.width() < 2 || area.height() < 2 {
                return Layout::default();
            }
            area.shrink_saturating(1, 1, 1, 1)
        } else {
            area
        };
        let content = inner.shrink_saturating(
            self.padding.top,
            self.padding.bottom,
            self.padding.left,
            self.padding.right,
        );
        layout_children_vertical(&self.children, content, inner, self.gap, ctx)
    }

    fn handle_event(&self, _event: &Event, _ctx: &mut EventCtx<M>) {}

    fn constraints(&self) -> Constraints {
        container_constraints(
            &self.children,
            self.padding,
            self.border.is_some(),
            self.gap,
        )
    }

    fn measure(&self, proposal: SizeProposal, ctx: &MeasureCtx) -> MeasuredSize {
        let border_size = if self.border.is_some() { 2 } else { 0 };
        let horizontal_chrome = self.padding.horizontal_total().saturating_add(border_size);
        let vertical_chrome = self.padding.vertical_total().saturating_add(border_size);
        let inner_width = subtract_limit(proposal.width, horizontal_chrome);
        let measured = measure_vertical_children(&self.children, inner_width, self.gap, ctx);
        let natural_width = measured.width.saturating_add(horizontal_chrome);
        let natural_height = measured.height.saturating_add(vertical_chrome);

        self.layout_style().clamp_size(MeasuredSize::new(
            fit_axis(natural_width, proposal.width),
            fit_axis(natural_height, proposal.height),
        ))
    }

    fn children(&self) -> &[Box<dyn Widget<M>>] {
        &self.children
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

pub(crate) fn layout_children_vertical<M>(
    children: &[Box<dyn Widget<M>>],
    content: Area,
    clip: Area,
    gap: u16,
    ctx: &MeasureCtx,
) -> Layout {
    if content.width() == 0 || content.height() == 0 {
        return Layout::default();
    }
    let layout_content = Area::new(
        content.pos(),
        (
            content.width(),
            clip.y()
                .saturating_add(clip.height())
                .saturating_sub(content.y()),
        )
            .into(),
    );
    let areas = TaffyBridge::new()
        .compute_layout_measured(
            children,
            layout_content,
            Direction::Vertical,
            gap,
            None,
            None,
            ctx,
        )
        .unwrap_or_else(|_| {
            let mut y = content.y();
            children
                .iter()
                .map(|child| {
                    let height = child.constraints().min_height.min(
                        content
                            .y()
                            .saturating_add(content.height())
                            .saturating_sub(y),
                    );
                    let area = Area::new((content.x(), y).into(), (content.width(), height).into());
                    y = y.saturating_add(height).saturating_add(gap);
                    area
                })
                .collect()
        });
    Layout::new(areas.into_iter().enumerate().filter_map(|(i, area)| {
        let area = Area::new(
            (content.x(), area.y()).into(),
            (content.width(), area.height()).into(),
        );
        area.clamp_to(&clip).map(|area| ChildLayout::new(i, area))
    }))
}

fn measure_vertical_children<M>(
    children: &[Box<dyn Widget<M>>],
    width: AxisLimit,
    gap: u16,
    ctx: &MeasureCtx,
) -> MeasuredSize {
    let mut measured = MeasuredSize::ZERO;
    measured.height = gap.saturating_mul(children.len().saturating_sub(1) as u16);

    for (index, child) in children.iter().enumerate() {
        let child_ctx = ctx.child_ctx(WidgetKey::for_child(index, child.as_ref()));
        let child_size = child.measure(
            SizeProposal {
                width,
                height: AxisLimit::Unbounded,
            },
            &child_ctx,
        );
        measured.width = measured.width.max(child_size.width);
        measured.height = measured.height.saturating_add(child_size.height);
    }

    measured
}

pub(crate) fn container_constraints<M>(
    children: &[Box<dyn Widget<M>>],
    padding: Padding,
    has_border: bool,
    gap: u16,
) -> Constraints {
    let border_size = if has_border { 2 } else { 0 };
    let child_width = children
        .iter()
        .map(|child| child.constraints().min_width)
        .max()
        .unwrap_or(0);
    let child_height = children
        .iter()
        .map(|child| child.constraints().min_height)
        .sum::<u16>()
        .saturating_add(gap.saturating_mul(children.len().saturating_sub(1) as u16));

    Constraints {
        min_width: child_width
            .saturating_add(padding.horizontal_total())
            .saturating_add(border_size),
        max_width: None,
        min_height: child_height
            .saturating_add(padding.vertical_total())
            .saturating_add(border_size),
        max_height: None,
        flex: Some(1.0),
    }
}

fn subtract_limit(limit: AxisLimit, amount: u16) -> AxisLimit {
    match limit {
        AxisLimit::Unbounded => AxisLimit::Unbounded,
        AxisLimit::AtMost(value) => AxisLimit::AtMost(value.saturating_sub(amount)),
        AxisLimit::Exact(value) => AxisLimit::Exact(value.saturating_sub(amount)),
    }
}

fn fit_axis(natural: u16, proposal: AxisLimit) -> u16 {
    match proposal {
        AxisLimit::Unbounded => natural,
        AxisLimit::AtMost(max) => natural.min(max),
        AxisLimit::Exact(value) => value,
    }
}

fn truncate_to_width(text: &str, max_width: usize, metrics: render::text::TextMetrics) -> String {
    metrics.clip(text, 0, max_width)
}

pub fn panel<M>() -> Panel<M> {
    Panel::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::AnimationStore;
    use crate::event::Event;
    use crate::style::Theme;
    use crate::widget::{RenderCtx, Widget, WidgetPath, WidgetStore};
    use render::buffer::Buffer;
    use render::chunk::Chunk;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    #[test]
    fn panel_gives_flex_child_remaining_vertical_space() {
        let recorded = Rc::new(RefCell::new(None));
        let child = RecordingWidget {
            recorded: Rc::clone(&recorded),
            constraints: Constraints {
                min_width: 4,
                max_width: None,
                min_height: 2,
                max_height: None,
                flex: Some(1.0),
            },
        };
        let panel = panel::<()>()
            .borderless()
            .padding(Padding::ZERO)
            .gap(1)
            .child(crate::widgets::label("head"))
            .child(child);

        let mut buffer = Buffer::new((20, 8).into());
        let area = Area::new((0, 0).into(), (20, 8).into());
        let mut chunk = Chunk::new(&mut buffer, area).unwrap();
        let store = WidgetStore::new();
        let animation_store = AnimationStore::new();
        let theme = Theme::dark();
        let geometry = RefCell::new(HashMap::<WidgetPath, Area>::new());
        let ctx = RenderCtx::new(&store, &animation_store, &theme, None, &geometry);

        panel.render(&mut chunk, &ctx);

        assert_eq!(
            *recorded.borrow(),
            Some(Area::new((0, 2).into(), (20, 6).into()))
        );
    }

    #[test]
    fn panel_still_passes_full_content_width_to_fixed_children() {
        let recorded = Rc::new(RefCell::new(None));
        let child = RecordingWidget {
            recorded: Rc::clone(&recorded),
            constraints: Constraints::fixed(4, 2),
        };
        let panel = panel::<()>()
            .borderless()
            .padding(Padding::ZERO)
            .child(child);

        let mut buffer = Buffer::new((20, 8).into());
        let area = Area::new((0, 0).into(), (20, 8).into());
        let mut chunk = Chunk::new(&mut buffer, area).unwrap();
        let store = WidgetStore::new();
        let animation_store = AnimationStore::new();
        let theme = Theme::dark();
        let geometry = RefCell::new(HashMap::<WidgetPath, Area>::new());
        let ctx = RenderCtx::new(&store, &animation_store, &theme, None, &geometry);

        panel.render(&mut chunk, &ctx);

        assert_eq!(
            *recorded.borrow(),
            Some(Area::new((0, 0).into(), (20, 2).into()))
        );
    }

    #[test]
    fn panel_clips_children_to_content_area_when_space_is_tight() {
        let panel = panel::<()>()
            .padding(Padding::ZERO)
            .child(crate::widgets::label("first\nsecond\nthird"));

        let buffer = render_widget(&panel, 12, 4);
        let chars = BorderStyle::Single.chars();

        assert_eq!(cell_char(&buffer, 0, 3), Some(chars.bottom_left));
        for x in 1..11 {
            assert_eq!(cell_char(&buffer, x, 3), Some(chars.horizontal));
        }
        assert_eq!(cell_char(&buffer, 11, 3), Some(chars.bottom_right));
    }

    #[test]
    fn tight_panel_can_use_bottom_padding_without_covering_border() {
        let panel = panel::<()>()
            .padding(Padding::uniform(1))
            .child(crate::widgets::label("a\nb\nc"));

        let buffer = render_widget(&panel, 8, 5);
        let chars = BorderStyle::Single.chars();

        assert_eq!(cell_char(&buffer, 2, 2), Some('a'));
        assert_eq!(cell_char(&buffer, 2, 3), Some('b'));
        assert_eq!(cell_char(&buffer, 0, 4), Some(chars.bottom_left));
        for x in 1..7 {
            assert_eq!(cell_char(&buffer, x, 4), Some(chars.horizontal));
        }
        assert_eq!(cell_char(&buffer, 7, 4), Some(chars.bottom_right));
    }

    struct RecordingWidget {
        recorded: Rc<RefCell<Option<Area>>>,
        constraints: Constraints,
    }

    impl Widget<()> for RecordingWidget {
        fn render(&self, chunk: &mut render::chunk::Chunk, _ctx: &RenderCtx) {
            *self.recorded.borrow_mut() = Some(chunk.area());
        }

        fn handle_event(&self, _event: &Event, _ctx: &mut EventCtx<()>) {}

        fn constraints(&self) -> Constraints {
            self.constraints
        }
    }

    fn render_widget(widget: &impl Widget<()>, width: u16, height: u16) -> Buffer {
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
