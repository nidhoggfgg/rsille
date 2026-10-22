//! Grid layout container

use crate::render::area::Area;
use crate::tui::layout::{ChildLayout, Layout};

use super::border_renderer::{render_background, render_border};
use super::grid_placement::GridPlacement;
use super::grid_track::GridTrack;
use super::taffy_bridge::TaffyBridge;
use crate::tui::event::Event;
use crate::tui::focus::{FocusConfig, FocusScope};
use crate::tui::layout::{AxisLimit, Constraints, MeasuredSize, SizeProposal};
use crate::tui::style::{BorderStyle, Padding, Style};
use crate::tui::widget::{EventCtx, IntoWidget, MeasureCtx, RenderCtx, Widget, WidgetKey};
use taffy::style::{AlignItems, JustifyItems};

/// Grid layout widget that arranges children in a 2D grid.
///
/// This is a pure layout container — it does not handle events or manage focus.
pub struct Grid<M = ()> {
    children: Vec<Box<dyn Widget<M>>>,
    placements: Vec<GridPlacement>,
    template_columns: Vec<GridTrack>,
    template_rows: Vec<GridTrack>,
    gap: u16,
    gap_row: Option<u16>,
    gap_column: Option<u16>,
    padding: Padding,
    border: Option<BorderStyle>,
    style: Style,
    align_items: Option<AlignItems>,
    justify_items: Option<JustifyItems>,
    focus_scope: Option<FocusScope>,
    widget_key: Option<String>,
}

impl<M> std::fmt::Debug for Grid<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Grid")
            .field("children", &self.children.len())
            .field("template_columns", &self.template_columns)
            .field("template_rows", &self.template_rows)
            .field("gap", &self.gap)
            .field("padding", &self.padding)
            .field("border", &self.border)
            .field("style", &self.style)
            .finish()
    }
}

impl<M> Grid<M> {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            placements: Vec::new(),
            template_columns: vec![GridTrack::Fr(1.0)],
            template_rows: vec![GridTrack::Auto],
            gap: 0,
            gap_row: None,
            gap_column: None,
            padding: Padding::ZERO,
            border: None,
            style: Style::default(),
            align_items: None,
            justify_items: None,
            focus_scope: None,
            widget_key: None,
        }
    }

    pub fn columns(mut self, template: &str) -> Self {
        self.template_columns = GridTrack::parse_template(template);
        self
    }

    pub fn rows(mut self, template: &str) -> Self {
        self.template_rows = GridTrack::parse_template(template);
        self
    }

    pub fn template_columns(mut self, tracks: Vec<GridTrack>) -> Self {
        self.template_columns = tracks;
        self
    }

    pub fn template_rows(mut self, tracks: Vec<GridTrack>) -> Self {
        self.template_rows = tracks;
        self
    }

    pub fn gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    pub fn gap_row(mut self, gap: u16) -> Self {
        self.gap_row = Some(gap);
        self
    }

    pub fn gap_column(mut self, gap: u16) -> Self {
        self.gap_column = Some(gap);
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

    pub fn border(mut self, border: BorderStyle) -> Self {
        self.border = Some(border);
        self
    }

    pub fn align_items(mut self, align: AlignItems) -> Self {
        self.align_items = Some(align);
        self
    }

    pub fn justify_items(mut self, justify: JustifyItems) -> Self {
        self.justify_items = Some(justify);
        self
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn focus_scope(mut self, scope: FocusScope) -> Self {
        self.focus_scope = Some(scope);
        self
    }

    pub fn trap_focus(self) -> Self {
        self.focus_scope(
            FocusScope::new()
                .trap_tab(true)
                .entry(crate::tui::focus::ScopeEntry::LastFocused),
        )
    }

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

    pub fn len(&self) -> usize {
        self.children.len()
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
}

impl<M> Grid<M> {
    pub fn child(mut self, widget: impl IntoWidget<M>) -> Self {
        self.children.push(widget.into_widget());
        self.placements.push(GridPlacement::default());
        self
    }

    pub fn child_at(mut self, widget: impl IntoWidget<M>, placement: GridPlacement) -> Self {
        self.children.push(widget.into_widget());
        self.placements.push(placement);
        self
    }

    pub fn child_area(self, widget: impl IntoWidget<M>, column: i16, row: i16) -> Self {
        self.child_at(widget, GridPlacement::new().area(column, row))
    }

    pub fn child_span(
        self,
        widget: impl IntoWidget<M>,
        column_start: i16,
        row_start: i16,
        column_span: u16,
        row_span: u16,
    ) -> Self {
        self.child_at(
            widget,
            GridPlacement::new().area_span(column_start, row_start, column_span, row_span),
        )
    }

    pub fn children<I>(mut self, widgets: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoWidget<M>,
    {
        for widget in widgets {
            self.children.push(widget.into_widget());
            self.placements.push(GridPlacement::default());
        }
        self
    }
}

impl<M> Widget<M> for Grid<M> {
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

        let mut content_area = area;

        if should_fill_background {
            render_background(chunk, render_style);
        }

        if let Some(ref border) = self.border {
            render_border(chunk, *border, border_render_style);
            content_area = Area::new(
                (area.x() + 1, area.y() + 1).into(),
                (
                    area.width().saturating_sub(2),
                    area.height().saturating_sub(2),
                )
                    .into(),
            );
        }

        let padded_area = Area::new(
            (
                content_area.x() + self.padding.left,
                content_area.y() + self.padding.top,
            )
                .into(),
            (
                content_area
                    .width()
                    .saturating_sub(self.padding.left + self.padding.right),
                content_area
                    .height()
                    .saturating_sub(self.padding.top + self.padding.bottom),
            )
                .into(),
        );

        if padded_area.width() == 0 || padded_area.height() == 0 {
            return;
        }

        ctx.render_children(chunk, self);
    }

    fn layout(&self, area: Area, ctx: &MeasureCtx) -> Layout {
        let content_area = if self.border.is_some() {
            area.shrink_saturating(1, 1, 1, 1)
        } else {
            area
        };
        let padded_area = content_area.shrink_saturating(
            self.padding.top,
            self.padding.bottom,
            self.padding.left,
            self.padding.right,
        );
        if padded_area.width() == 0 || padded_area.height() == 0 {
            return Layout::default();
        }
        let gap_row = self.gap_row.unwrap_or(self.gap);
        let gap_column = self.gap_column.unwrap_or(self.gap);

        let mut bridge = TaffyBridge::new();

        let items: Vec<(&dyn Widget<M>, &GridPlacement)> = self
            .children
            .iter()
            .zip(self.placements.iter())
            .map(|(w, p)| (w.as_ref() as &dyn Widget<M>, p))
            .collect();

        let child_areas = match bridge.compute_grid_layout_with_placement(
            &items,
            padded_area,
            &self.template_columns,
            &self.template_rows,
            gap_row,
            gap_column,
            self.align_items,
            self.justify_items,
            ctx,
        ) {
            Ok(areas) => areas,
            Err(_) => return Layout::default(),
        };

        Layout::new(
            child_areas
                .into_iter()
                .enumerate()
                .filter_map(|(index, area)| {
                    area.clamp_to(&content_area)
                        .map(|area| ChildLayout::new(index, area))
                }),
        )
    }

    fn handle_event(&self, _event: &Event, _ctx: &mut EventCtx<M>) {
        // Containers don't handle events — the framework routes directly to focused widgets.
    }

    fn constraints(&self) -> Constraints {
        Constraints {
            min_width: 0,
            max_width: None,
            min_height: 0,
            max_height: None,
            flex: Some(1.0),
        }
    }

    fn measure(&self, proposal: SizeProposal, ctx: &MeasureCtx) -> MeasuredSize {
        let border_size = if self.border.is_some() { 2 } else { 0 };
        let horizontal_chrome = self.padding.horizontal_total().saturating_add(border_size);
        let vertical_chrome = self.padding.vertical_total().saturating_add(border_size);
        let inner_width = subtract_limit(proposal.width, horizontal_chrome);
        let inner_height = subtract_limit(proposal.height, vertical_chrome);
        let gap_row = self.gap_row.unwrap_or(self.gap);
        let gap_column = self.gap_column.unwrap_or(self.gap);

        let columns = measure_grid_axis(
            &self.children,
            &self.placements,
            &self.template_columns,
            &self.template_rows,
            Axis::Column,
            gap_column,
            gap_row,
            inner_width,
            ctx,
        );
        let rows = measure_grid_axis(
            &self.children,
            &self.placements,
            &self.template_rows,
            &self.template_columns,
            Axis::Row,
            gap_row,
            gap_column,
            inner_height,
            ctx,
        );

        let natural_width = sum_tracks(&columns, gap_column).saturating_add(horizontal_chrome);
        let natural_height = sum_tracks(&rows, gap_row).saturating_add(vertical_chrome);

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

impl<M> Default for Grid<M> {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a new grid layout.
pub fn grid<M>() -> Grid<M> {
    Grid::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::style::Theme;
    use crate::tui::widget::{MeasureCtx, WidgetStore};
    use crate::tui::widgets::label;

    #[test]
    fn grid_measures_fixed_tracks_gaps_and_chrome() {
        let grid = grid::<()>()
            .columns("10 5")
            .rows("3")
            .gap_column(2)
            .padding(Padding::horizontal(1))
            .border(BorderStyle::Single)
            .child(label("ignored"));
        let store = WidgetStore::new();
        let theme = Theme::dark();
        let ctx = MeasureCtx::new(&store, &theme);

        let measured = grid.measure(SizeProposal::UNBOUNDED, &ctx);

        assert_eq!(measured.width, 21);
        assert_eq!(measured.height, 5);
    }

    #[test]
    fn grid_measures_auto_tracks_from_child_content() {
        let grid = grid::<()>()
            .columns("4 auto")
            .rows("auto")
            .gap_column(1)
            .child_at(label("fixed"), GridPlacement::new().area(1, 1))
            .child_at(label("natural"), GridPlacement::new().area(2, 1));
        let store = WidgetStore::new();
        let theme = Theme::dark();
        let ctx = MeasureCtx::new(&store, &theme);

        let measured = grid.measure(SizeProposal::UNBOUNDED, &ctx);

        assert_eq!(measured.width, 12);
        assert_eq!(measured.height, 1);
    }
}

#[derive(Clone, Copy)]
enum Axis {
    Column,
    Row,
}

fn measure_grid_axis<M>(
    children: &[Box<dyn Widget<M>>],
    placements: &[GridPlacement],
    tracks: &[GridTrack],
    cross_tracks: &[GridTrack],
    axis: Axis,
    gap: u16,
    cross_gap: u16,
    limit: AxisLimit,
    ctx: &MeasureCtx,
) -> Vec<u16> {
    let mut sizes: Vec<u16> = tracks
        .iter()
        .map(|track| match *track {
            GridTrack::Fixed(size) => size,
            GridTrack::Auto | GridTrack::Fr(_) => 0,
        })
        .collect();

    if sizes.is_empty() {
        return sizes;
    }

    for (index, child) in children.iter().enumerate() {
        let placement = placements.get(index).copied().unwrap_or_default();
        let Some((track_index, span)) = placement_track(placement, index, tracks.len(), axis)
        else {
            continue;
        };

        let end = track_index
            .saturating_add(usize::from(span))
            .min(sizes.len());
        if track_index >= end {
            continue;
        }

        let includes_auto = tracks[track_index..end]
            .iter()
            .any(|track| matches!(track, GridTrack::Auto));
        if !includes_auto {
            continue;
        }

        let child_ctx = ctx.child_ctx(WidgetKey::for_child(index, child.as_ref()));
        let proposal = child_measure_proposal(
            placement,
            index,
            tracks,
            cross_tracks,
            axis,
            gap,
            cross_gap,
            limit,
        );
        let measured = child.measure(proposal, &child_ctx);
        let measured_axis = match axis {
            Axis::Column => measured.width,
            Axis::Row => measured.height,
        };
        let measured_axis = measured_axis.saturating_sub(gap.saturating_mul(span - 1));
        let share = measured_axis.div_ceil(span);

        for track_size in sizes.iter_mut().take(end).skip(track_index) {
            *track_size = (*track_size).max(share);
        }
    }

    sizes
}

fn child_measure_proposal(
    placement: GridPlacement,
    child_index: usize,
    tracks: &[GridTrack],
    cross_tracks: &[GridTrack],
    axis: Axis,
    gap: u16,
    cross_gap: u16,
    limit: AxisLimit,
) -> SizeProposal {
    let own_limit = span_limit(placement, child_index, tracks, axis, gap, limit);
    let cross_limit = span_limit(
        placement,
        child_index,
        cross_tracks,
        cross_axis(axis),
        cross_gap,
        AxisLimit::Unbounded,
    );

    match axis {
        Axis::Column => SizeProposal {
            width: own_limit,
            height: cross_limit,
        },
        Axis::Row => SizeProposal {
            width: cross_limit,
            height: own_limit,
        },
    }
}

fn span_limit(
    placement: GridPlacement,
    child_index: usize,
    tracks: &[GridTrack],
    axis: Axis,
    gap: u16,
    fallback: AxisLimit,
) -> AxisLimit {
    let Some((start, span)) = placement_track(placement, child_index, tracks.len(), axis) else {
        return AxisLimit::Unbounded;
    };
    let end = start.saturating_add(usize::from(span)).min(tracks.len());
    let mut total = gap.saturating_mul(span.saturating_sub(1));

    for track in &tracks[start..end] {
        match *track {
            GridTrack::Fixed(size) => total = total.saturating_add(size),
            GridTrack::Auto | GridTrack::Fr(_) => return fallback,
        }
    }

    AxisLimit::Exact(total)
}

fn placement_track(
    placement: GridPlacement,
    child_index: usize,
    track_count: usize,
    axis: Axis,
) -> Option<(usize, u16)> {
    if track_count == 0 {
        return None;
    }

    let (start, end) = match axis {
        Axis::Column => (placement.column_start, placement.column_end),
        Axis::Row => (placement.row_start, placement.row_end),
    };

    match start {
        super::grid_placement::GridLine::Line(line) if line > 0 => {
            let start = (line - 1) as usize;
            let span = match end {
                super::grid_placement::GridLine::Line(end_line) if end_line > line => {
                    (end_line - line) as u16
                }
                _ => 1,
            };
            Some((start, span.max(1)))
        }
        _ => {
            let index = match axis {
                Axis::Column => child_index % track_count,
                Axis::Row => child_index / track_count,
            };
            if index < track_count {
                Some((index, 1))
            } else {
                None
            }
        }
    }
}

fn cross_axis(axis: Axis) -> Axis {
    match axis {
        Axis::Column => Axis::Row,
        Axis::Row => Axis::Column,
    }
}

fn sum_tracks(tracks: &[u16], gap: u16) -> u16 {
    tracks
        .iter()
        .copied()
        .fold(0u16, u16::saturating_add)
        .saturating_add(gap.saturating_mul(tracks.len().saturating_sub(1) as u16))
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
