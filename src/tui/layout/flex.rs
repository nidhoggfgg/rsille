//! Flex layout container

use crate::render::area::Area;

use super::border_renderer::{render_background, render_border};
use super::taffy_bridge::TaffyBridge;
use crate::tui::event::Event;
use crate::tui::focus::{FocusConfig, FocusScope};
use crate::tui::layout::{AxisLimit, ChildLayout, Constraints, Layout, MeasuredSize, SizeProposal};
use crate::tui::style::{BorderStyle, Padding, Style};
use crate::tui::widget::{EventCtx, IntoWidget, MeasureCtx, RenderCtx, Widget, WidgetKey};
use taffy::style::{AlignItems, JustifyContent};

/// Layout direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Arranges children from top to bottom (a column).
    Vertical,
    /// Arranges children from left to right (a row).
    Horizontal,
}

/// Flex widget that arranges children using flexbox layout.
///
/// Children are laid out inside the border and padding. Use [`Self::gap`] for
/// spacing, [`Self::align_items`] for cross-axis alignment, and
/// [`Self::justify_content`] for main-axis distribution. The framework routes
/// events to descendants and applies any configured focus scope.
///
/// # Examples
///
/// ```
/// use rsille::tui::{layout::{col, row}, widgets::label};
///
/// let view = col::<()>()
///     .gap(1)
///     .child(label("Title"))
///     .child(row().gap(2).child(label("Left")).child(label("Right")));
/// assert_eq!(view.len(), 2);
/// ```
pub struct Flex<M = ()> {
    children: Vec<Box<dyn Widget<M>>>,
    direction: Direction,
    gap: u16,
    padding: Padding,
    border: Option<BorderStyle>,
    style: Style,
    align_items: Option<AlignItems>,
    justify_content: Option<JustifyContent>,
    focus_scope: Option<FocusScope>,
    widget_key: Option<String>,
}

impl<M> std::fmt::Debug for Flex<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Flex")
            .field("children", &self.children.len())
            .field("direction", &self.direction)
            .field("gap", &self.gap)
            .field("padding", &self.padding)
            .field("border", &self.border)
            .field("style", &self.style)
            .field("align_items", &self.align_items)
            .field("justify_content", &self.justify_content)
            .finish()
    }
}

impl<M> Flex<M> {
    fn with_direction(children: Vec<Box<dyn Widget<M>>>, direction: Direction) -> Self {
        Self {
            children,
            direction,
            gap: 0,
            padding: Padding::ZERO,
            border: None,
            style: Style::default(),
            align_items: None,
            justify_content: None,
            focus_scope: None,
            widget_key: None,
        }
    }

    /// Creates a column with the supplied children, no gap, padding, or border.
    pub fn vertical(children: Vec<Box<dyn Widget<M>>>) -> Self {
        Self::with_direction(children, Direction::Vertical)
    }

    /// Creates a row with the supplied children, no gap, padding, or border.
    pub fn horizontal(children: Vec<Box<dyn Widget<M>>>) -> Self {
        Self::with_direction(children, Direction::Horizontal)
    }

    /// Creates an empty column. Equivalent to [`col`].
    pub fn new() -> Self {
        Self::vertical(Vec::new())
    }

    /// Sets the spacing between adjacent children in cells; defaults to zero.
    pub fn gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    /// Sets inner spacing in terminal cells, inside any border; defaults to zero.
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    /// Sets the container style, merged with the theme when rendering.
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Adds a one-cell border around the content and padding.
    pub fn border(mut self, border: BorderStyle) -> Self {
        self.border = Some(border);
        self
    }

    /// Sets cross-axis alignment: horizontal for columns, vertical for rows.
    ///
    /// The default is stretch, subject to each child's size limits.
    pub fn align_items(mut self, align: AlignItems) -> Self {
        self.align_items = Some(align);
        self
    }

    /// Sets how children share free space along the main axis.
    ///
    /// The default places children at the start of the axis.
    pub fn justify_content(mut self, justify: JustifyContent) -> Self {
        self.justify_content = Some(justify);
        self
    }

    /// Sets a stable key for widget identity and persistent state.
    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    /// Configures focus traversal for descendants of this container.
    pub fn focus_scope(mut self, scope: FocusScope) -> Self {
        self.focus_scope = Some(scope);
        self
    }

    /// Traps Tab traversal within this container and restores its last focused child on entry.
    pub fn trap_focus(self) -> Self {
        self.focus_scope(
            FocusScope::new()
                .trap_tab(true)
                .entry(crate::tui::focus::ScopeEntry::LastFocused),
        )
    }

    /// Applies `f` only when `condition` is true; otherwise returns this builder unchanged.
    pub fn when<F>(self, condition: bool, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        if condition {
            f(self)
        } else {
            self
        }
    }

    /// Appends an already boxed widget to this container.
    pub fn add_child(&mut self, child: Box<dyn Widget<M>>) {
        self.children.push(child);
    }

    /// Returns the number of children.
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Returns whether the container has no children.
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
}

impl<M> Flex<M> {
    /// Appends one child and returns the builder.
    pub fn child(mut self, widget: impl IntoWidget<M>) -> Self {
        self.children.push(widget.into_widget());
        self
    }

    /// Appends children in iteration order.
    pub fn children<I>(mut self, widgets: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoWidget<M>,
    {
        self.children
            .extend(widgets.into_iter().map(|w| w.into_widget()));
        self
    }
}

impl<M> Widget<M> for Flex<M> {
    fn render(&self, chunk: &mut crate::render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        let surface_style = self.style.merge(ctx.theme().styles.surface);
        let border_style = self
            .style
            .merge(ctx.theme().styles.border.merge(ctx.theme().styles.surface));
        let render_style = surface_style.to_render_style();
        let border_render_style = border_style.to_render_style();
        let should_fill_background = ctx.path().is_empty() || self.style.bg_color.is_some();

        let border_area = if self.border.is_some() {
            if area.width() < 2 || area.height() < 2 {
                return;
            }
            Area::new(
                (area.x() + 1, area.y() + 1).into(),
                (area.width() - 2, area.height() - 2).into(),
            )
        } else {
            area
        };

        if should_fill_background {
            render_background(chunk, render_style);
        }

        if let Some(border) = self.border {
            render_border(chunk, border, border_render_style);
        }

        let inner = border_area.shrink_saturating(
            self.padding.top,
            self.padding.bottom,
            self.padding.left,
            self.padding.right,
        );

        if inner.width() == 0 || inner.height() == 0 {
            return;
        }

        ctx.render_children(chunk, self);
    }

    fn layout(&self, area: Area, ctx: &MeasureCtx) -> Layout {
        let border_area = if self.border.is_some() {
            if area.width() < 2 || area.height() < 2 {
                return Layout::default();
            }
            area.shrink_saturating(1, 1, 1, 1)
        } else {
            area
        };
        let inner = border_area.shrink_saturating(
            self.padding.top,
            self.padding.bottom,
            self.padding.left,
            self.padding.right,
        );
        if inner.width() == 0 || inner.height() == 0 {
            return Layout::default();
        }
        let areas = TaffyBridge::new()
            .compute_layout_measured(
                &self.children,
                inner,
                self.direction,
                self.gap,
                self.align_items,
                self.justify_content,
                ctx,
            )
            .unwrap_or_default();
        Layout::new(areas.into_iter().enumerate().filter_map(|(index, area)| {
            area.clamp_to(&border_area)
                .map(|area| ChildLayout::new(index, area))
        }))
    }

    fn handle_event(&self, _event: &Event, _ctx: &mut EventCtx<M>) {
        // Containers don't handle events — the framework routes directly to focused widgets.
    }

    fn constraints(&self) -> Constraints {
        let border_size = if self.border.is_some() { 2 } else { 0 };

        if self.children.is_empty() {
            return Constraints::fixed(
                self.padding.horizontal_total() + border_size,
                self.padding.vertical_total() + border_size,
            );
        }

        match self.direction {
            Direction::Vertical => {
                let total_height = self
                    .children
                    .iter()
                    .map(|c| c.constraints().min_height)
                    .sum::<u16>()
                    + (self.children.len() as u16 - 1) * self.gap
                    + self.padding.vertical_total()
                    + border_size;

                let max_width = self
                    .children
                    .iter()
                    .map(|c| c.constraints().min_width)
                    .max()
                    .unwrap_or(0)
                    + self.padding.horizontal_total()
                    + border_size;

                Constraints {
                    min_width: max_width,
                    max_width: None,
                    min_height: total_height,
                    max_height: None,
                    flex: Some(1.0),
                }
            }
            Direction::Horizontal => {
                let total_width = self
                    .children
                    .iter()
                    .map(|c| c.constraints().min_width)
                    .sum::<u16>()
                    + (self.children.len() as u16 - 1) * self.gap
                    + self.padding.horizontal_total()
                    + border_size;

                let max_height = self
                    .children
                    .iter()
                    .map(|c| c.constraints().min_height)
                    .max()
                    .unwrap_or(0)
                    + self.padding.vertical_total()
                    + border_size;

                Constraints {
                    min_width: total_width,
                    max_width: None,
                    min_height: max_height,
                    max_height: Some(max_height),
                    flex: None,
                }
            }
        }
    }

    fn measure(&self, proposal: SizeProposal, ctx: &MeasureCtx) -> MeasuredSize {
        let border_size = if self.border.is_some() { 2 } else { 0 };
        let horizontal_chrome = self.padding.horizontal_total().saturating_add(border_size);
        let vertical_chrome = self.padding.vertical_total().saturating_add(border_size);
        let inner_width = subtract_limit(proposal.width, horizontal_chrome);
        let inner_height = subtract_limit(proposal.height, vertical_chrome);
        let gap_total = self
            .gap
            .saturating_mul(self.children.len().saturating_sub(1) as u16);

        let measured = match self.direction {
            Direction::Vertical => {
                let mut width: u16 = 0;
                let mut height: u16 = gap_total;
                for (index, child) in self.children.iter().enumerate() {
                    let child_ctx = ctx.child_ctx(WidgetKey::for_child(index, child.as_ref()));
                    let child_size = child.measure(
                        SizeProposal {
                            width: inner_width,
                            height: AxisLimit::Unbounded,
                        },
                        &child_ctx,
                    );
                    width = width.max(child_size.width);
                    height = height.saturating_add(child_size.height);
                }
                MeasuredSize::new(width, height)
            }
            Direction::Horizontal => {
                let mut width: u16 = gap_total;
                let mut height: u16 = 0;
                for (index, child) in self.children.iter().enumerate() {
                    let child_ctx = ctx.child_ctx(WidgetKey::for_child(index, child.as_ref()));
                    let child_size = child.measure(
                        SizeProposal {
                            width: AxisLimit::Unbounded,
                            height: inner_height,
                        },
                        &child_ctx,
                    );
                    width = width.saturating_add(child_size.width);
                    height = height.max(child_size.height);
                }
                MeasuredSize::new(width, height)
            }
        };

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

    fn focus_config(&self) -> FocusConfig {
        self.focus_scope
            .clone()
            .map(FocusConfig::Scope)
            .unwrap_or(FocusConfig::None)
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

impl<M> Default for Flex<M> {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a new empty vertical flex layout.
pub fn col<M>() -> Flex<M> {
    Flex::new()
}

/// Create a new empty horizontal flex layout.
pub fn row<M>() -> Flex<M> {
    Flex::horizontal(Vec::new())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::buffer::Buffer;
    use crate::render::chunk::Chunk;
    use crate::tui::animation::AnimationStore;
    use crate::tui::layout::LayoutStyle;
    use crate::tui::style::Theme;
    use crate::tui::widget::{MeasureCtx, RenderCtx, Widget, WidgetPath, WidgetStore};
    use crate::tui::widgets::label;
    use std::cell::RefCell;
    use std::collections::HashMap;

    #[test]
    fn vertical_flex_measures_children_with_fixed_width_and_sums_height() {
        let flex = col::<()>()
            .gap(1)
            .child(label("head"))
            .child(label("body\ntail"));
        let store = WidgetStore::new();
        let theme = Theme::dark();
        let ctx = MeasureCtx::new(&store, &theme);

        let measured = flex.measure(
            SizeProposal {
                width: AxisLimit::Exact(12),
                height: AxisLimit::AtMost(20),
            },
            &ctx,
        );

        assert_eq!(measured.width, 12);
        assert_eq!(measured.height, 4);
    }

    #[test]
    fn horizontal_flex_measures_children_and_keeps_natural_height() {
        let flex = row::<()>()
            .gap(2)
            .child(label("left"))
            .child(label("right"));
        let store = WidgetStore::new();
        let theme = Theme::dark();
        let ctx = MeasureCtx::new(&store, &theme);

        let measured = flex.measure(
            SizeProposal {
                width: AxisLimit::AtMost(20),
                height: AxisLimit::Exact(3),
            },
            &ctx,
        );

        assert_eq!(measured.width, 11);
        assert_eq!(measured.height, 1);
    }

    #[test]
    fn horizontal_flex_layout_respects_preferred_width_before_growing_remainder() {
        let widgets: Vec<Box<dyn Widget<()>>> = vec![
            Box::new(StyleWidget::new(LayoutStyle::min(2, 1).preferred_width(6))),
            Box::new(StyleWidget::new(LayoutStyle::min(0, 1).flex(1.0))),
        ];
        let store = WidgetStore::new();
        let theme = Theme::dark();
        let ctx = MeasureCtx::new(&store, &theme);
        let mut bridge = TaffyBridge::new();

        let areas = bridge
            .compute_layout_measured(
                &widgets,
                Area::new((0, 0).into(), (20, 1).into()),
                Direction::Horizontal,
                0,
                None,
                None,
                &ctx,
            )
            .expect("layout");

        assert_eq!(areas[0].width(), 6);
        assert_eq!(areas[1].width(), 14);
    }

    #[test]
    fn root_flex_background_remains_under_child_label_text() {
        let flex = col::<()>().child(label("Hello, TUI!"));
        let mut buffer = Buffer::new((16, 2).into());
        let area = Area::new((0, 0).into(), (16, 2).into());
        let mut chunk = Chunk::new(&mut buffer, area).unwrap();
        let store = WidgetStore::new();
        let animation_store = AnimationStore::new();
        let theme = Theme::dark();
        let geometry = RefCell::new(HashMap::<WidgetPath, Area>::new());
        let ctx = RenderCtx::new(&store, &animation_store, &theme, None, &geometry);

        flex.render(&mut chunk, &ctx);

        let cell = &buffer.content()[0];
        let colors = cell.content.style.colors.unwrap();
        let surface_colors = theme.styles.surface.to_render_style().colors.unwrap();
        assert_eq!(cell.content.as_char(), Some('H'));
        assert_eq!(colors.background, surface_colors.background);
    }

    struct StyleWidget {
        style: LayoutStyle,
    }

    impl StyleWidget {
        fn new(style: LayoutStyle) -> Self {
            Self { style }
        }
    }

    impl Widget<()> for StyleWidget {
        fn render(&self, _chunk: &mut crate::render::chunk::Chunk, _ctx: &RenderCtx) {}

        fn constraints(&self) -> Constraints {
            Constraints {
                min_width: self.style.min_width,
                max_width: self.style.max_width,
                min_height: self.style.min_height,
                max_height: self.style.max_height,
                flex: (self.style.flex_grow > 0.0).then_some(self.style.flex_grow),
            }
        }

        fn layout_style(&self) -> LayoutStyle {
            self.style
        }

        fn measure(&self, proposal: SizeProposal, _ctx: &MeasureCtx) -> MeasuredSize {
            self.style.resolve_fallback_size(proposal)
        }
    }
}
