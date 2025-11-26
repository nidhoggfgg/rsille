//! Bridge to Taffy layout engine

use crate::layout::grid_track::GridTrack;
use crate::layout::Constraints;
use crate::widget::Widget;
use render::area::Area;
use std::cell::RefCell;
use taffy::prelude::*;

thread_local! {
    static TAFFY: RefCell<TaffyTree<()>> = RefCell::new(TaffyTree::new());
}

/// Layout manager using Taffy for flexbox layout
pub struct TaffyBridge;

impl Default for TaffyBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl TaffyBridge {
    pub fn new() -> Self {
        Self
    }

    /// Compute layout for a list of widgets
    pub fn compute_layout<M: Clone>(
        &mut self,
        widgets: &[Box<dyn Widget<M>>],
        available: Area,
        direction: super::flex::Direction,
        gap: u16,
        align_items: Option<AlignItems>,
        justify_content: Option<JustifyContent>,
    ) -> Vec<Area> {
        if widgets.is_empty() {
            return vec![];
        }

        TAFFY.with(|t| {
            let mut tree = t.borrow_mut();
            // Reset tree for new layout calculation
            // We create a new tree as clearing is not exposed/efficient in this version
            *tree = TaffyTree::new();

            // Create Taffy nodes for each widget
            let mut nodes = Vec::with_capacity(widgets.len());
            for widget in widgets {
                let constraints = widget.constraints();
                let style = self.constraints_to_style(constraints, direction, align_items);
                let node = tree.new_leaf(style).unwrap();
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

            let container = tree.new_with_children(container_style, &nodes).unwrap();

            // Compute layout
            let available_size = Size {
                width: AvailableSpace::Definite(available.width() as f32),
                height: AvailableSpace::Definite(available.height() as f32),
            };

            tree.compute_layout(container, available_size).unwrap();

            // Extract computed positions
            let mut results = Vec::with_capacity(nodes.len());
            for node in nodes.iter() {
                let layout = tree.layout(*node).unwrap();
                results.push(Area::new(
                    (
                        available.x() + layout.location.x as u16,
                        available.y() + layout.location.y as u16,
                    )
                        .into(),
                    (layout.size.width as u16, layout.size.height as u16).into(),
                ));
            }

            results
        })
    }

    /// Compute grid layout for a list of widgets with placement information
    pub fn compute_grid_layout_with_placement<M: Clone>(
        &mut self,
        items: &[(&dyn Widget<M>, &super::grid_placement::GridPlacement)],
        available: Area,
        template_columns: &[GridTrack],
        template_rows: &[GridTrack],
        gap_row: u16,
        gap_column: u16,
        align_items: Option<AlignItems>,
        justify_items: Option<JustifyItems>,
    ) -> Vec<Area> {
        if items.is_empty() {
            return vec![];
        }

        TAFFY.with(|t| {
            let mut tree = t.borrow_mut();
            // Reset tree for new layout calculation
            *tree = TaffyTree::new();

            // Create Taffy nodes for each widget with placement
            let mut nodes = Vec::with_capacity(items.len());
            for (widget, placement) in items {
                let constraints = widget.constraints();
                let style = self.constraints_to_grid_style_with_placement(constraints, placement);
                let node = tree.new_leaf(style).unwrap();
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

            let container = tree.new_with_children(container_style, &nodes).unwrap();

            // Compute layout
            let available_size = Size {
                width: AvailableSpace::Definite(available.width() as f32),
                height: AvailableSpace::Definite(available.height() as f32),
            };

            tree.compute_layout(container, available_size).unwrap();

            // Extract computed positions
            let mut results = Vec::with_capacity(nodes.len());
            for node in nodes.iter() {
                let layout = tree.layout(*node).unwrap();
                results.push(Area::new(
                    (
                        available.x() + layout.location.x as u16,
                        available.y() + layout.location.y as u16,
                    )
                        .into(),
                    (layout.size.width as u16, layout.size.height as u16).into(),
                ));
            }

            results
        })
    }

    /// Convert GridTrack to Taffy TrackSizingFunction
    fn grid_track_to_taffy(&self, track: GridTrack) -> TrackSizingFunction {
        match track {
            GridTrack::Fixed(size) => length(size as f32),
            GridTrack::Fr(fraction) => flex(fraction),
            GridTrack::Auto => auto(),
        }
    }

    /// Convert widget constraints to grid item style
    fn constraints_to_grid_style(&self, constraints: Constraints) -> Style {
        let width = if let Some(max) = constraints.max_width {
            if max == constraints.min_width {
                Dimension::length(max as f32)
            } else {
                Dimension::auto()
            }
        } else {
            Dimension::auto()
        };

        let height = if let Some(max) = constraints.max_height {
            if max == constraints.min_height {
                Dimension::length(max as f32)
            } else {
                Dimension::auto()
            }
        } else {
            Dimension::auto()
        };

        let min_size = Size {
            width: Dimension::length(constraints.min_width as f32),
            height: Dimension::length(constraints.min_height as f32),
        };

        let max_size = Size {
            width: constraints
                .max_width
                .map(|w| Dimension::length(w as f32))
                .unwrap_or(Dimension::auto()),
            height: constraints
                .max_height
                .map(|h| Dimension::length(h as f32))
                .unwrap_or(Dimension::auto()),
        };

        Style {
            size: Size { width, height },
            min_size,
            max_size,
            ..Default::default()
        }
    }

    /// Convert widget constraints and grid placement to grid item style
    fn constraints_to_grid_style_with_placement(
        &self,
        constraints: Constraints,
        placement: &super::grid_placement::GridPlacement,
    ) -> Style {
        // Start with base style from constraints
        let mut style = self.constraints_to_grid_style(constraints);

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

        style
    }

    fn constraints_to_style(
        &self,
        constraints: Constraints,
        direction: super::flex::Direction,
        align_items: Option<AlignItems>,
    ) -> Style {
        let is_stretch = align_items.unwrap_or(AlignItems::Stretch) == AlignItems::Stretch;

        let (width, height) = match direction {
            super::flex::Direction::Vertical => {
                // Vertical layout (Column)
                // Cross axis is Width.
                let width = if let Some(max) = constraints.max_width {
                    if max == constraints.min_width {
                        // Fixed width
                        Dimension::length(max as f32)
                    } else {
                        // Max width constraint
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
                let height = if let Some(max) = constraints.max_height {
                    if max == constraints.min_height {
                        Dimension::length(constraints.min_height as f32)
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
                let width = if let Some(max) = constraints.max_width {
                    if max == constraints.min_width {
                        Dimension::length(max as f32)
                    } else {
                        Dimension::auto()
                    }
                } else if constraints.flex.is_some() {
                    // Flex grow will handle it
                    Dimension::auto()
                } else {
                    Dimension::length(constraints.min_width as f32)
                };

                // Cross axis is Height
                let height = if let Some(max) = constraints.max_height {
                    if max == constraints.min_height {
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
                    Dimension::length(constraints.min_height as f32)
                };

                (width, height)
            }
        };

        let flex_grow = constraints.flex.unwrap_or(0.0);

        // Set min_size and max_size to enforce constraints strictly
        let min_size = Size {
            width: Dimension::length(constraints.min_width as f32),
            height: Dimension::length(constraints.min_height as f32),
        };

        let max_size = Size {
            width: constraints
                .max_width
                .map(|w| Dimension::length(w as f32))
                .unwrap_or(Dimension::auto()),
            height: constraints
                .max_height
                .map(|h| Dimension::length(h as f32))
                .unwrap_or(Dimension::auto()),
        };

        Style {
            size: Size { width, height },
            min_size,
            max_size,
            flex_grow,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Event, EventResult};
    use crate::layout::Direction;
    use crate::widget::Label;

    struct TestWidget {
        constraints: Constraints,
    }

    impl TestWidget {
        fn new(constraints: Constraints) -> Self {
            Self { constraints }
        }
    }

    impl Widget<()> for TestWidget {
        fn render(&self, _chunk: &mut render::chunk::Chunk) {}
        fn handle_event(&mut self, _event: &Event) -> EventResult<()> {
            EventResult::Ignored
        }
        fn constraints(&self) -> Constraints {
            self.constraints
        }
    }

    #[test]
    fn test_taffy_layout_calculation() {
        let mut bridge = TaffyBridge::new();

        // Create test labels with known sizes
        let label1 = Label::new("Hello"); // 5 chars
        let label2 = Label::new("World!"); // 6 chars

        // Available area: 80x24 terminal
        let available = Area::new((0, 0).into(), (80, 24).into());

        // Create widgets
        let widgets: Vec<Box<dyn Widget<()>>> = vec![Box::new(label1), Box::new(label2)];

        // Compute layout
        let results =
            bridge.compute_layout(&widgets, available, Direction::Vertical, 1, None, None);

        // Assert reasonable values
        assert!(
            results[0].width() >= 5,
            "First child should have width at least 5"
        );
        assert!(
            results[0].height() >= 1,
            "First child should have height at least 1"
        );
        assert!(
            results[1].width() >= 6,
            "Second child should have width at least 6"
        );
        assert!(
            results[1].height() >= 1,
            "Second child should have height at least 1"
        );
    }

    #[test]
    fn test_align_items_center() {
        let mut bridge = TaffyBridge::new();
        let label = Label::new("Hello");
        let widgets: Vec<Box<dyn Widget<()>>> = vec![Box::new(label)];
        let available = Area::new((0, 0).into(), (80, 24).into());

        // Vertical layout with AlignItems::Center (should not stretch width)
        let results = bridge.compute_layout(
            &widgets,
            available,
            Direction::Vertical,
            0,
            Some(AlignItems::Center),
            None,
        );

        // Width should be content size (5), not full width (80)
        assert_eq!(
            results[0].width(),
            5,
            "Width should be content size when centered"
        );
        // Center of 80 is 40. Center of 5 is 2.5.
        // Left should be 37.5.
        // Taffy rounding might give 37 or 38.
        let x = results[0].x();
        assert!(
            x == 37 || x == 38,
            "Should be centered horizontally (approx), got {}",
            x
        );
    }

    #[test]
    fn test_align_items_stretch() {
        let mut bridge = TaffyBridge::new();
        // Use a widget without max_width to test stretching
        let widget = TestWidget::new(Constraints {
            min_width: 5,
            max_width: None,
            min_height: 1,
            max_height: Some(1),
            flex: None,
        });
        let widgets: Vec<Box<dyn Widget<()>>> = vec![Box::new(widget)];
        let available = Area::new((0, 0).into(), (80, 24).into());

        // Vertical layout with AlignItems::Stretch (default)
        let results = bridge.compute_layout(
            &widgets,
            available,
            Direction::Vertical,
            0,
            Some(AlignItems::Stretch),
            None,
        );

        // Width should be full width (80)
        assert_eq!(results[0].width(), 80, "Width should stretch");
    }

    #[test]
    fn test_justify_content_center() {
        let mut bridge = TaffyBridge::new();
        let label = Label::new("Hello");
        let widgets: Vec<Box<dyn Widget<()>>> = vec![Box::new(label)];
        let available = Area::new((0, 0).into(), (80, 24).into());

        // Vertical layout with JustifyContent::Center
        let results = bridge.compute_layout(
            &widgets,
            available,
            Direction::Vertical,
            0,
            None,
            Some(JustifyContent::Center),
        );

        // Should be centered vertically
        // Available height 24. Item height 1. Center is around 11/12.
        assert!(results[0].y() > 0, "Should be centered vertically");
        assert!(results[0].y() < 23, "Should be centered vertically");
    }
}
