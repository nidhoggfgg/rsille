//! Resizable split-pane layouts.

use crate::render::area::Area;
use crate::tui::layout::{ChildLayout, Layout};

use crate::tui::event::{Event, KeyCode, MouseButton, MouseEventKind};
use crate::tui::focus::FocusConfig;
use crate::tui::layout::{AxisLimit, Constraints, MeasuredSize, SizeProposal};
use crate::tui::style::Style;
use crate::tui::widget::{
    EventCtx, EventPhase, IntoWidget, MeasureCtx, RenderCtx, Widget, WidgetKey,
};

/// Split direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitDirection {
    #[default]
    Horizontal,
    Vertical,
}

/// Initial sizing mode for a split layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SplitSize {
    Ratio(f32),
    First(u16),
    Second(u16),
}

impl Default for SplitSize {
    fn default() -> Self {
        Self::Ratio(0.5)
    }
}

/// Persistent split state stored in the widget store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SplitState {
    pub first_size: Option<u16>,
    pub dragging: bool,
}

#[derive(Debug, Clone, Copy)]
struct SplitLayout {
    first: Area,
    divider: Area,
    second: Area,
    first_size: u16,
}

/// Two-pane resizable split container.
pub struct Split<M = ()> {
    children: Vec<Box<dyn Widget<M>>>,
    direction: SplitDirection,
    initial_size: SplitSize,
    min_first: Option<u16>,
    min_second: Option<u16>,
    divider_size: u16,
    divider_style: Option<Style>,
    handle_style: Option<Style>,
    widget_key: Option<String>,
    resizable: bool,
    resize_step: u16,
}

impl<M> std::fmt::Debug for Split<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Split")
            .field("direction", &self.direction)
            .field("initial_size", &self.initial_size)
            .field("min_first", &self.min_first)
            .field("min_second", &self.min_second)
            .field("divider_size", &self.divider_size)
            .field("resizable", &self.resizable)
            .field("resize_step", &self.resize_step)
            .finish()
    }
}

impl<M> Split<M> {
    pub fn new(first: impl IntoWidget<M>, second: impl IntoWidget<M>) -> Self {
        Self {
            children: vec![first.into_widget(), second.into_widget()],
            direction: SplitDirection::Horizontal,
            initial_size: SplitSize::Ratio(0.5),
            min_first: None,
            min_second: None,
            divider_size: 1,
            divider_style: None,
            handle_style: None,
            widget_key: None,
            resizable: true,
            resize_step: 2,
        }
    }

    pub fn direction(mut self, direction: SplitDirection) -> Self {
        self.direction = direction;
        self
    }

    pub fn horizontal(mut self) -> Self {
        self.direction = SplitDirection::Horizontal;
        self
    }

    pub fn vertical(mut self) -> Self {
        self.direction = SplitDirection::Vertical;
        self
    }

    pub fn ratio(mut self, ratio: f32) -> Self {
        self.initial_size = SplitSize::Ratio(ratio.clamp(0.0, 1.0));
        self
    }

    pub fn first_size(mut self, size: u16) -> Self {
        self.initial_size = SplitSize::First(size);
        self
    }

    pub fn second_size(mut self, size: u16) -> Self {
        self.initial_size = SplitSize::Second(size);
        self
    }

    pub fn sidebar(mut self, width: u16) -> Self {
        self.initial_size = SplitSize::First(width);
        self.direction = SplitDirection::Horizontal;
        self
    }

    pub fn min_first(mut self, size: u16) -> Self {
        self.min_first = Some(size);
        self
    }

    pub fn min_second(mut self, size: u16) -> Self {
        self.min_second = Some(size);
        self
    }

    pub fn divider_size(mut self, size: u16) -> Self {
        self.divider_size = size.max(1);
        self
    }

    pub fn divider_style(mut self, style: Style) -> Self {
        self.divider_style = Some(style);
        self
    }

    pub fn handle_style(mut self, style: Style) -> Self {
        self.handle_style = Some(style);
        self
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn resize_step(mut self, step: u16) -> Self {
        self.resize_step = step.max(1);
        self
    }

    fn effective_min_first(&self) -> u16 {
        self.min_first.unwrap_or_else(|| match self.direction {
            SplitDirection::Horizontal => self.children[0].layout_style().min_width,
            SplitDirection::Vertical => self.children[0].layout_style().min_height,
        })
    }

    fn effective_min_second(&self) -> u16 {
        self.min_second.unwrap_or_else(|| match self.direction {
            SplitDirection::Horizontal => self.children[1].layout_style().min_width,
            SplitDirection::Vertical => self.children[1].layout_style().min_height,
        })
    }

    fn clamp_first_size(&self, first_size: u16, total_main: u16) -> u16 {
        let available = total_main.saturating_sub(self.divider_size);
        let min_first = self.effective_min_first().min(available);
        let max_first = available.saturating_sub(self.effective_min_second().min(available));
        first_size.clamp(min_first, max_first.max(min_first))
    }

    fn initial_first_size(&self, total_main: u16) -> u16 {
        let available = total_main.saturating_sub(self.divider_size);
        let desired = match self.initial_size {
            SplitSize::Ratio(ratio) => ((available as f32) * ratio).round() as u16,
            SplitSize::First(size) => size,
            SplitSize::Second(size) => available.saturating_sub(size),
        };
        self.clamp_first_size(desired, total_main)
    }

    fn layout(&self, area: Area, state: &SplitState) -> Option<SplitLayout> {
        let total_main = match self.direction {
            SplitDirection::Horizontal => area.width(),
            SplitDirection::Vertical => area.height(),
        };
        if total_main <= self.divider_size {
            return None;
        }

        let first_size = self.clamp_first_size(
            state
                .first_size
                .unwrap_or_else(|| self.initial_first_size(total_main)),
            total_main,
        );
        let second_size = total_main.saturating_sub(first_size + self.divider_size);

        let (first, divider, second) = match self.direction {
            SplitDirection::Horizontal => (
                Area::new(area.pos(), (first_size, area.height()).into()),
                Area::new(
                    (area.x() + first_size, area.y()).into(),
                    (self.divider_size, area.height()).into(),
                ),
                Area::new(
                    (area.x() + first_size + self.divider_size, area.y()).into(),
                    (second_size, area.height()).into(),
                ),
            ),
            SplitDirection::Vertical => (
                Area::new(area.pos(), (area.width(), first_size).into()),
                Area::new(
                    (area.x(), area.y() + first_size).into(),
                    (area.width(), self.divider_size).into(),
                ),
                Area::new(
                    (area.x(), area.y() + first_size + self.divider_size).into(),
                    (area.width(), second_size).into(),
                ),
            ),
        };

        Some(SplitLayout {
            first,
            divider,
            second,
            first_size,
        })
    }
}

impl<M> Widget<M> for Split<M> {
    fn layout(&self, area: Area, ctx: &MeasureCtx) -> Layout {
        let state = ctx.state::<SplitState>().copied().unwrap_or_default();
        let Some(layout) = self.layout(area, &state) else {
            return Layout::default();
        };
        Layout::new([
            ChildLayout::new(0, layout.first),
            ChildLayout::new(1, layout.second),
        ])
        .with_data(layout)
    }

    fn render(&self, chunk: &mut crate::render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        ctx.record_bounds(area);

        let result = ctx.layout(self, area);
        let Some(layout) = result.data::<SplitLayout>() else {
            return;
        };
        ctx.render_children(chunk, self);

        if let Ok(mut divider_chunk) = chunk.from_area(layout.divider) {
            let divider_style = self
                .divider_style
                .unwrap_or(ctx.theme().styles.border.merge(ctx.theme().styles.surface))
                .to_render_style();
            let handle_style = self
                .handle_style
                .unwrap_or(ctx.theme().styles.selected_focused)
                .to_render_style();

            match self.direction {
                SplitDirection::Horizontal => {
                    let _ = divider_chunk.fill(
                        0,
                        0,
                        layout.divider.width(),
                        layout.divider.height(),
                        '│',
                        divider_style,
                    );
                    if layout.divider.height() >= 3 {
                        let middle = layout.divider.height() / 2;
                        let _ =
                            divider_chunk.set_char(0, middle.saturating_sub(1), '•', handle_style);
                        let _ = divider_chunk.set_char(0, middle, '•', handle_style);
                        let _ = divider_chunk.set_char(
                            0,
                            (middle + 1).min(layout.divider.height() - 1),
                            '•',
                            handle_style,
                        );
                    }
                }
                SplitDirection::Vertical => {
                    let _ = divider_chunk.fill(
                        0,
                        0,
                        layout.divider.width(),
                        layout.divider.height(),
                        '─',
                        divider_style,
                    );
                    if layout.divider.width() >= 3 {
                        let middle = layout.divider.width() / 2;
                        let _ =
                            divider_chunk.set_char(middle.saturating_sub(1), 0, '•', handle_style);
                        let _ = divider_chunk.set_char(middle, 0, '•', handle_style);
                        let _ = divider_chunk.set_char(
                            (middle + 1).min(layout.divider.width() - 1),
                            0,
                            '•',
                            handle_style,
                        );
                    }
                }
            }
        }
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if crate::tui::event::is_key_release(event) {
            return;
        }
        if !self.resizable || !matches!(ctx.phase(), EventPhase::Target | EventPhase::Bubble) {
            return;
        }
        if ctx.phase() == EventPhase::Bubble && ctx.was_handled() {
            return;
        }

        let (pointer_x, pointer_y) = ctx.layout_mouse_position(event).unwrap_or((0, 0));
        let Some(bounds) = ctx.bounds() else {
            return;
        };
        let snapshot = {
            let state = ctx.state_mut::<SplitState>();
            *state
        };
        let Some(layout) = self.layout(bounds, &snapshot) else {
            return;
        };

        let total_main = match self.direction {
            SplitDirection::Horizontal => bounds.width(),
            SplitDirection::Vertical => bounds.height(),
        };
        let mut next_size = layout.first_size;
        let mut dragging = snapshot.dragging;
        let mut handled = false;

        match event {
            Event::Key(key_event) => match (self.direction, key_event.code) {
                (SplitDirection::Horizontal, KeyCode::Left)
                | (SplitDirection::Vertical, KeyCode::Up) => {
                    next_size = next_size.saturating_sub(self.resize_step);
                    handled = true;
                }
                (SplitDirection::Horizontal, KeyCode::Right)
                | (SplitDirection::Vertical, KeyCode::Down) => {
                    next_size = next_size.saturating_add(self.resize_step);
                    handled = true;
                }
                (SplitDirection::Horizontal, KeyCode::Home)
                | (SplitDirection::Vertical, KeyCode::Home) => {
                    next_size = self.effective_min_first();
                    handled = true;
                }
                (SplitDirection::Horizontal, KeyCode::End)
                | (SplitDirection::Vertical, KeyCode::End) => {
                    next_size = total_main
                        .saturating_sub(self.divider_size)
                        .saturating_sub(self.effective_min_second());
                    handled = true;
                }
                _ => {}
            },
            Event::Mouse(mouse_event) => match mouse_event.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    let inside_divider = pointer_x >= layout.divider.x()
                        && pointer_x < layout.divider.x() + layout.divider.width()
                        && pointer_y >= layout.divider.y()
                        && pointer_y < layout.divider.y() + layout.divider.height();
                    if inside_divider {
                        dragging = true;
                        next_size = match self.direction {
                            SplitDirection::Horizontal => pointer_x.saturating_sub(bounds.x()),
                            SplitDirection::Vertical => pointer_y.saturating_sub(bounds.y()),
                        };
                        handled = true;
                        ctx.request_focus_self();
                    }
                }
                MouseEventKind::Drag(MouseButton::Left) if dragging => {
                    next_size = match self.direction {
                        SplitDirection::Horizontal => pointer_x.saturating_sub(bounds.x()),
                        SplitDirection::Vertical => pointer_y.saturating_sub(bounds.y()),
                    };
                    handled = true;
                }
                MouseEventKind::Up(MouseButton::Left) if dragging => {
                    dragging = false;
                    handled = true;
                }
                _ => {}
            },
            _ => {}
        }

        if handled {
            let state = ctx.state_mut::<SplitState>();
            state.first_size = Some(self.clamp_first_size(next_size, total_main));
            state.dragging = dragging;
            ctx.request_layout();
            ctx.set_handled();
        }
    }

    fn constraints(&self) -> Constraints {
        let first = self.children[0].constraints();
        let second = self.children[1].constraints();

        match self.direction {
            SplitDirection::Horizontal => Constraints {
                min_width: first
                    .min_width
                    .saturating_add(second.min_width)
                    .saturating_add(self.divider_size),
                max_width: None,
                min_height: first.min_height.max(second.min_height),
                max_height: None,
                flex: Some(1.0),
            },
            SplitDirection::Vertical => Constraints {
                min_width: first.min_width.max(second.min_width),
                max_width: None,
                min_height: first
                    .min_height
                    .saturating_add(second.min_height)
                    .saturating_add(self.divider_size),
                max_height: None,
                flex: Some(1.0),
            },
        }
    }

    fn measure(&self, proposal: SizeProposal, ctx: &MeasureCtx) -> MeasuredSize {
        let first_ctx = ctx.child_ctx(WidgetKey::for_child(0, self.children[0].as_ref()));
        let second_ctx = ctx.child_ctx(WidgetKey::for_child(1, self.children[1].as_ref()));

        let measured = match self.direction {
            SplitDirection::Horizontal => {
                let first = self.children[0].measure(
                    SizeProposal {
                        width: AxisLimit::Unbounded,
                        height: proposal.height,
                    },
                    &first_ctx,
                );
                let second = self.children[1].measure(
                    SizeProposal {
                        width: AxisLimit::Unbounded,
                        height: proposal.height,
                    },
                    &second_ctx,
                );
                MeasuredSize::new(
                    first
                        .width
                        .saturating_add(second.width)
                        .saturating_add(self.divider_size),
                    first.height.max(second.height),
                )
            }
            SplitDirection::Vertical => {
                let first = self.children[0].measure(
                    SizeProposal {
                        width: proposal.width,
                        height: AxisLimit::Unbounded,
                    },
                    &first_ctx,
                );
                let second = self.children[1].measure(
                    SizeProposal {
                        width: proposal.width,
                        height: AxisLimit::Unbounded,
                    },
                    &second_ctx,
                );
                MeasuredSize::new(
                    first.width.max(second.width),
                    first
                        .height
                        .saturating_add(second.height)
                        .saturating_add(self.divider_size),
                )
            }
        };

        self.layout_style().clamp_size(MeasuredSize::new(
            fit_axis(measured.width, proposal.width),
            fit_axis(measured.height, proposal.height),
        ))
    }

    fn children(&self) -> &[Box<dyn Widget<M>>] {
        &self.children
    }

    fn focus_config(&self) -> FocusConfig {
        if self.resizable {
            FocusConfig::Composite
        } else {
            FocusConfig::None
        }
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

/// Create a new two-pane split container.
pub fn split<M>(first: impl IntoWidget<M>, second: impl IntoWidget<M>) -> Split<M> {
    Split::new(first, second)
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
    use crate::tui::style::Theme;
    use crate::tui::widget::{MeasureCtx, Widget, WidgetStore};
    use crate::tui::widgets::label;

    #[test]
    fn split_default_minimum_comes_from_child_layout_style() {
        let split = Split::new(label::<()>("wide label"), label("b")).ratio(0.0);
        let layout = split
            .layout(
                Area::new((0, 0).into(), (24, 4).into()),
                &SplitState::default(),
            )
            .expect("layout");

        assert_eq!(
            layout.first.width(),
            split.children[0].layout_style().min_width
        );
    }

    #[test]
    fn split_explicit_minimum_overrides_child_minimum() {
        let split = Split::new(label::<()>("wide label"), label("b"))
            .ratio(0.0)
            .min_first(3);
        let layout = split
            .layout(
                Area::new((0, 0).into(), (24, 4).into()),
                &SplitState::default(),
            )
            .expect("layout");

        assert_eq!(layout.first.width(), 3);
    }

    #[test]
    fn split_measure_uses_child_measurement() {
        let split = Split::new(label::<()>("left"), label("right")).horizontal();
        let store = WidgetStore::new();
        let theme = Theme::dark();
        let ctx = MeasureCtx::new(&store, &theme);

        let measured = split.measure(SizeProposal::UNBOUNDED, &ctx);

        assert_eq!(measured, MeasuredSize::new(10, 1));
    }
}
