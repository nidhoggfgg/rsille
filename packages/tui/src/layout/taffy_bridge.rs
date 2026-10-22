//! Bridge to Taffy layout engine

use crate::layout::grid_track::GridTrack;
use crate::layout::{AxisLimit, Constraints, LayoutStyle, SizeProposal};
use crate::widget::{MeasureCtx, Widget, WidgetKey};
use render::area::Area;
use taffy::prelude::*;

/// Layout manager using Taffy for flexbox layout
pub struct TaffyBridge {
    tree: TaffyTree<usize>,
}

impl Default for TaffyBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl TaffyBridge {
    pub fn new() -> Self {
        Self {
            tree: TaffyTree::new(),
        }
    }

    /// Compute layout for a list of widgets
    pub fn compute_layout<M>(
        &mut self,
        widgets: &[Box<dyn Widget<M>>],
        available: Area,
        direction: super::flex::Direction,
        gap: u16,
        align_items: Option<AlignItems>,
        justify_content: Option<JustifyContent>,
    ) -> Result<Vec<Area>, crate::WidgetError> {
        if widgets.is_empty() {
            return Ok(vec![]);
        }

        // Reset tree for new layout calculation
        // We create a new tree as clearing is not exposed/efficient in this version
        self.tree = TaffyTree::new();

        // Create Taffy nodes for each widget
        let mut nodes = Vec::with_capacity(widgets.len());
        for widget in widgets {
            let constraints = widget.constraints();
            let style = self.constraints_to_style(constraints, direction, align_items);
            let node = self.tree.new_leaf(style)?;
            nodes.push(node);
        }

        // Create container node
        let flex_direction = match direction {
            super::flex::Direction::Vertical => FlexDirection::Column,
            super::flex::Direction::Horizontal => FlexDirection::Row,
        };

        let gap_size = gap as f32;
        let container_style = taffy::Style {
            display: Display::Flex,
            flex_direction,
            align_items,
            justify_content,
            gap: taffy::Size {
                width: length(gap_size),
                height: length(gap_size),
            },
            // IMPORTANT: Set the container size to match available space
            size: taffy::Size {
                width: length(available.width() as f32),
                height: length(available.height() as f32),
            },
            ..Default::default()
        };

        let container = self.tree.new_with_children(container_style, &nodes)?;

        // Compute layout
        let available_size = Size {
            width: AvailableSpace::Definite(available.width() as f32),
            height: AvailableSpace::Definite(available.height() as f32),
        };

        self.tree.compute_layout(container, available_size)?;

        // Extract computed positions
        let mut results = Vec::with_capacity(nodes.len());
        for node in nodes.iter() {
            let layout = self.tree.layout(*node)?;
            results.push(layout_to_area(available, layout));
        }

        Ok(results)
    }

    /// Compute flex layout using widget measurement callbacks for leaf nodes.
    pub fn compute_layout_measured<M>(
        &mut self,
        widgets: &[Box<dyn Widget<M>>],
        available: Area,
        direction: super::flex::Direction,
        gap: u16,
        align_items: Option<AlignItems>,
        justify_content: Option<JustifyContent>,
        measure_ctx: &MeasureCtx,
    ) -> Result<Vec<Area>, crate::WidgetError> {
        if widgets.is_empty() {
            return Ok(vec![]);
        }

        self.tree = TaffyTree::new();

        let mut nodes = Vec::with_capacity(widgets.len());
        for (index, widget) in widgets.iter().enumerate() {
            let child_ctx = measure_ctx.child_ctx(WidgetKey::for_child(index, widget.as_ref()));
            let style = self.layout_style_to_style(
                widget.layout_style_with(&child_ctx),
                direction,
                align_items,
            );
            let node = self.tree.new_leaf_with_context(style, index)?;
            nodes.push(node);
        }

        let flex_direction = match direction {
            super::flex::Direction::Vertical => FlexDirection::Column,
            super::flex::Direction::Horizontal => FlexDirection::Row,
        };

        let gap_size = gap as f32;
        let container_style = taffy::Style {
            display: Display::Flex,
            flex_direction,
            align_items,
            justify_content,
            gap: taffy::Size {
                width: length(gap_size),
                height: length(gap_size),
            },
            size: taffy::Size {
                width: length(available.width() as f32),
                height: length(available.height() as f32),
            },
            ..Default::default()
        };

        let container = self.tree.new_with_children(container_style, &nodes)?;
        let available_size = Size {
            width: AvailableSpace::Definite(available.width() as f32),
            height: AvailableSpace::Definite(available.height() as f32),
        };

        self.tree.compute_layout_with_measure(
            container,
            available_size,
            |known_dimensions, available_space, _node, node_context, _style| {
                let Some(index) = node_context.map(|index| *index) else {
                    return Size::ZERO;
                };
                let Some(widget) = widgets.get(index) else {
                    return Size::ZERO;
                };

                let proposal = SizeProposal {
                    width: taffy_axis_to_proposal(known_dimensions.width, available_space.width),
                    height: taffy_axis_to_proposal(known_dimensions.height, available_space.height),
                };
                let child_ctx = measure_ctx.child_ctx(WidgetKey::for_child(index, widget.as_ref()));
                let measured = widget.measure(proposal, &child_ctx);
                let measured = widget.layout_style_with(&child_ctx).clamp_size(measured);

                Size {
                    width: measured.width as f32,
                    height: measured.height as f32,
                }
            },
        )?;

        let mut results = Vec::with_capacity(nodes.len());
        for node in nodes.iter() {
            let layout = self.tree.layout(*node)?;
            results.push(layout_to_area(available, layout));
        }

        Ok(results)
    }

    /// Compute grid layout for a list of widgets with placement information
    pub fn compute_grid_layout_with_placement<M>(
        &mut self,
        items: &[(&dyn Widget<M>, &super::grid_placement::GridPlacement)],
        available: Area,
        template_columns: &[GridTrack],
        template_rows: &[GridTrack],
        gap_row: u16,
        gap_column: u16,
        align_items: Option<AlignItems>,
        justify_items: Option<JustifyItems>,
        measure_ctx: &MeasureCtx,
    ) -> Result<Vec<Area>, crate::WidgetError> {
        if items.is_empty() {
            return Ok(vec![]);
        }

        // Reset tree for new layout calculation
        self.tree = TaffyTree::new();

        // Create Taffy nodes for each widget with placement
        let mut nodes = Vec::with_capacity(items.len());
        for (index, (widget, placement)) in items.iter().enumerate() {
            let child_ctx = measure_ctx.child_ctx(WidgetKey::for_child(index, *widget));
            let style = self.layout_style_to_grid_style_with_placement(
                widget.layout_style_with(&child_ctx),
                placement,
            );
            let node = self.tree.new_leaf_with_context(style, index)?;
            nodes.push(node);
        }

        // Convert GridTrack to Taffy track sizing
        let columns: Vec<TrackSizingFunction> = template_columns
            .iter()
            .map(|track| self.grid_track_to_taffy(*track))
            .collect();

        let rows: Vec<TrackSizingFunction> = template_rows
            .iter()
            .map(|track| self.grid_track_to_taffy(*track))
            .collect();

        // Create container node with grid layout
        let container_style = taffy::Style {
            display: Display::Grid,
            grid_template_columns: columns.iter().map(|&tsf| tsf.into()).collect(),
            grid_template_rows: rows.iter().map(|&tsf| tsf.into()).collect(),
            gap: taffy::Size {
                width: length(gap_column as f32),
                height: length(gap_row as f32),
            },
            align_items,
            justify_items,
            // IMPORTANT: Set the container size to match available space
            size: taffy::Size {
                width: length(available.width() as f32),
                height: length(available.height() as f32),
            },
            ..Default::default()
        };

        let container = self.tree.new_with_children(container_style, &nodes)?;

        // Compute layout
        let available_size = Size {
            width: AvailableSpace::Definite(available.width() as f32),
            height: AvailableSpace::Definite(available.height() as f32),
        };

        self.tree.compute_layout_with_measure(
            container,
            available_size,
            |known_dimensions, available_space, _node, node_context, _style| {
                let Some(index) = node_context.map(|index| *index) else {
                    return Size::ZERO;
                };
                let Some((widget, _)) = items.get(index) else {
                    return Size::ZERO;
                };

                let proposal = SizeProposal {
                    width: taffy_axis_to_proposal(known_dimensions.width, available_space.width),
                    height: taffy_axis_to_proposal(known_dimensions.height, available_space.height),
                };
                let child_ctx = measure_ctx.child_ctx(WidgetKey::for_child(index, *widget));
                let measured = widget.measure(proposal, &child_ctx);
                let measured = widget.layout_style_with(&child_ctx).clamp_size(measured);

                Size {
                    width: measured.width as f32,
                    height: measured.height as f32,
                }
            },
        )?;

        // Extract computed positions
        let mut results = Vec::with_capacity(nodes.len());
        for node in nodes.iter() {
            let layout = self.tree.layout(*node)?;
            results.push(layout_to_area(available, layout));
        }

        Ok(results)
    }

    /// Convert GridTrack to Taffy TrackSizingFunction
    fn grid_track_to_taffy(&self, track: GridTrack) -> TrackSizingFunction {
        match track {
            GridTrack::Fixed(size) => length(size as f32),
            GridTrack::Fr(fraction) => flex(fraction),
            GridTrack::Auto => auto(),
        }
    }

    fn layout_style_to_grid_style_with_placement(
        &self,
        layout_style: LayoutStyle,
        placement: &super::grid_placement::GridPlacement,
    ) -> Style {
        let mut style = self.layout_style_to_grid_style(layout_style);

        self.apply_grid_placement(&mut style, placement);

        style
    }

    fn layout_style_to_grid_style(&self, layout_style: LayoutStyle) -> Style {
        let width = if let Some(preferred) = layout_style.preferred_width {
            Dimension::length(preferred as f32)
        } else if let Some(max) = layout_style.max_width {
            if max == layout_style.min_width {
                Dimension::length(max as f32)
            } else {
                Dimension::auto()
            }
        } else {
            Dimension::auto()
        };

        let height = if let Some(preferred) = layout_style.preferred_height {
            Dimension::length(preferred as f32)
        } else if let Some(max) = layout_style.max_height {
            if max == layout_style.min_height {
                Dimension::length(max as f32)
            } else {
                Dimension::auto()
            }
        } else {
            Dimension::auto()
        };

        let min_size = Size {
            width: Dimension::length(layout_style.min_width as f32),
            height: Dimension::length(layout_style.min_height as f32),
        };

        let max_size = Size {
            width: layout_style
                .max_width
                .map(|w| Dimension::length(w as f32))
                .unwrap_or(Dimension::auto()),
            height: layout_style
                .max_height
                .map(|h| Dimension::length(h as f32))
                .unwrap_or(Dimension::auto()),
        };

        Style {
            size: Size { width, height },
            min_size,
            max_size,
            flex_grow: layout_style.flex_grow,
            flex_shrink: layout_style.flex_shrink,
            ..Default::default()
        }
    }

    fn apply_grid_placement(
        &self,
        style: &mut Style,
        placement: &super::grid_placement::GridPlacement,
    ) {
        // Add grid placement information
        // Convert our GridLine to Taffy's grid positioning
        use super::grid_placement::GridLine;

        // Grid column placement
        match (placement.column_start, placement.column_end) {
            (GridLine::Line(start), GridLine::Line(end)) => {
                // Explicit start and end
                style.grid_column = taffy::geometry::Line {
                    start: taffy::style::GridPlacement::Line(start.into()),
                    end: taffy::style::GridPlacement::Line(end.into()),
                };
            }
            (GridLine::Line(start), GridLine::Auto) => {
                // Just start position, auto end
                style.grid_column = taffy::geometry::Line {
                    start: taffy::style::GridPlacement::Line(start.into()),
                    end: taffy::style::GridPlacement::Auto,
                };
            }
            _ => {
                // Auto placement
                style.grid_column = taffy::geometry::Line {
                    start: taffy::style::GridPlacement::Auto,
                    end: taffy::style::GridPlacement::Auto,
                };
            }
        }

        // Grid row placement
        match (placement.row_start, placement.row_end) {
            (GridLine::Line(start), GridLine::Line(end)) => {
                style.grid_row = taffy::geometry::Line {
                    start: taffy::style::GridPlacement::Line(start.into()),
                    end: taffy::style::GridPlacement::Line(end.into()),
                };
            }
            (GridLine::Line(start), GridLine::Auto) => {
                style.grid_row = taffy::geometry::Line {
                    start: taffy::style::GridPlacement::Line(start.into()),
                    end: taffy::style::GridPlacement::Auto,
                };
            }
            _ => {
                style.grid_row = taffy::geometry::Line {
                    start: taffy::style::GridPlacement::Auto,
                    end: taffy::style::GridPlacement::Auto,
                };
            }
        }
    }

    fn constraints_to_style(
        &self,
        constraints: Constraints,
        direction: super::flex::Direction,
        align_items: Option<AlignItems>,
    ) -> Style {
        self.layout_style_to_style(
            LayoutStyle::from_constraints(constraints),
            direction,
            align_items,
        )
    }

    fn layout_style_to_style(
        &self,
        layout_style: LayoutStyle,
        direction: super::flex::Direction,
        align_items: Option<AlignItems>,
    ) -> Style {
        let is_stretch = align_items.unwrap_or(AlignItems::Stretch) == AlignItems::Stretch;

        let (width, height) = match direction {
            super::flex::Direction::Vertical => {
                // Vertical layout (Column)
                // Cross axis is Width.
                let width = if let Some(preferred) = layout_style.preferred_width {
                    Dimension::length(preferred as f32)
                } else if let Some(max) = layout_style.max_width {
                    if max == layout_style.min_width {
                        Dimension::length(max as f32)
                    } else {
                        Dimension::auto()
                    }
                } else if is_stretch {
                    // If stretch and no fixed width, use 100% or Auto?
                    // Taffy: Stretch requires Auto size on cross axis to work?
                    // Or we can force it with 100%.
                    // Using 100% forces it even if align-items changes (which we check here).
                    Dimension::percent(1.0)
                } else {
                    Dimension::auto()
                };

                // Main axis is Height
                let height = if let Some(preferred) = layout_style.preferred_height {
                    Dimension::length(preferred as f32)
                } else if let Some(max) = layout_style.max_height {
                    if max == layout_style.min_height {
                        Dimension::length(layout_style.min_height as f32)
                    } else {
                        Dimension::auto()
                    }
                } else {
                    Dimension::auto()
                };

                (width, height)
            }
            super::flex::Direction::Horizontal => {
                // Horizontal layout (Row)
                // Main axis is Width
                let width = if let Some(preferred) = layout_style.preferred_width {
                    Dimension::length(preferred as f32)
                } else if let Some(max) = layout_style.max_width {
                    if max == layout_style.min_width {
                        Dimension::length(max as f32)
                    } else {
                        Dimension::auto()
                    }
                } else if layout_style.flex_grow > 0.0 {
                    // Flex grow will handle it
                    Dimension::auto()
                } else {
                    Dimension::length(layout_style.min_width as f32)
                };

                // Cross axis is Height
                let height = if let Some(preferred) = layout_style.preferred_height {
                    Dimension::length(preferred as f32)
                } else if let Some(max) = layout_style.max_height {
                    if max == layout_style.min_height {
                        Dimension::length(max as f32)
                    } else {
                        Dimension::auto()
                    }
                } else if is_stretch {
                    // Stretch needs Auto or 100%
                    // If we use percent(1.0), it forces full height of container.
                    Dimension::percent(1.0)
                } else {
                    // If not stretch, default to auto (content size)
                    Dimension::length(layout_style.min_height as f32)
                };

                (width, height)
            }
        };

        // Set min_size and max_size to enforce constraints strictly
        let min_size = Size {
            width: Dimension::length(layout_style.min_width as f32),
            height: Dimension::length(layout_style.min_height as f32),
        };

        let max_size = Size {
            width: layout_style
                .max_width
                .map(|w| Dimension::length(w as f32))
                .unwrap_or(Dimension::auto()),
            height: layout_style
                .max_height
                .map(|h| Dimension::length(h as f32))
                .unwrap_or(Dimension::auto()),
        };

        Style {
            size: Size { width, height },
            min_size,
            max_size,
            flex_grow: layout_style.flex_grow,
            flex_shrink: layout_style.flex_shrink,
            ..Default::default()
        }
    }
}

fn taffy_axis_to_proposal(known: Option<f32>, available: AvailableSpace) -> AxisLimit {
    if let Some(value) = known {
        return AxisLimit::Exact(f32_to_cells(value));
    }

    match available {
        AvailableSpace::Definite(value) => AxisLimit::AtMost(f32_to_cells(value)),
        AvailableSpace::MinContent | AvailableSpace::MaxContent => AxisLimit::Unbounded,
    }
}

fn f32_to_cells(value: f32) -> u16 {
    if !value.is_finite() || value <= 0.0 {
        0
    } else {
        value.floor().min(u16::MAX as f32) as u16
    }
}

fn layout_to_area(available: Area, layout: &taffy::tree::Layout) -> Area {
    let left = floor_cells(layout.location.x).min(available.width());
    let top = floor_cells(layout.location.y).min(available.height());
    let right = ceil_cells(layout.location.x + layout.size.width).min(available.width());
    let bottom = ceil_cells(layout.location.y + layout.size.height).min(available.height());

    Area::new(
        (
            available.x().saturating_add(left),
            available.y().saturating_add(top),
        )
            .into(),
        (right.saturating_sub(left), bottom.saturating_sub(top)).into(),
    )
}

fn floor_cells(value: f32) -> u16 {
    if !value.is_finite() || value <= 0.0 {
        0
    } else {
        value.floor().min(u16::MAX as f32) as u16
    }
}

fn ceil_cells(value: f32) -> u16 {
    if !value.is_finite() || value <= 0.0 {
        0
    } else {
        value.ceil().min(u16::MAX as f32) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Theme;
    use crate::widget::{MeasureCtx, RenderCtx, Widget, WidgetStore};

    #[test]
    fn measured_layout_areas_are_clamped_to_available_bounds() {
        let widgets: Vec<Box<dyn Widget<()>>> =
            vec![Box::new(StyleWidget::new(LayoutStyle::fixed(12, 4)))];
        let store = WidgetStore::new();
        let theme = Theme::dark();
        let ctx = MeasureCtx::new(&store, &theme);
        let available = Area::new((3, 2).into(), (5, 2).into());
        let mut bridge = TaffyBridge::new();

        let areas = bridge
            .compute_layout_measured(
                &widgets,
                available,
                crate::layout::Direction::Horizontal,
                0,
                None,
                None,
                &ctx,
            )
            .expect("layout");

        let area = areas[0];
        assert!(area.x() >= available.x());
        assert!(area.y() >= available.y());
        assert!(area.x().saturating_add(area.width()) <= available.x() + available.width());
        assert!(area.y().saturating_add(area.height()) <= available.y() + available.height());
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
        fn render(&self, _chunk: &mut render::chunk::Chunk, _ctx: &RenderCtx) {}

        fn constraints(&self) -> Constraints {
            Constraints {
                min_width: self.style.min_width,
                max_width: self.style.max_width,
                min_height: self.style.min_height,
                max_height: self.style.max_height,
                flex: None,
            }
        }

        fn layout_style(&self) -> LayoutStyle {
            self.style
        }
    }
}
