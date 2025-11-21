//! Bridge to Taffy layout engine

use crate::layout::Constraints;
use crate::widget::Widget;
use render::area::Area;
use taffy::prelude::*;

/// Layout manager using Taffy for flexbox layout
pub struct TaffyBridge {
    tree: TaffyTree<()>,
}

impl TaffyBridge {
    pub fn new() -> Self {
        Self {
            tree: TaffyTree::new(),
        }
    }

    /// Compute layout for a list of widgets
    pub fn compute_layout<M: Clone>(
        &mut self,
        widgets: &[Box<dyn Widget<M>>],
        available: Area,
        direction: super::container::Direction,
        gap: u16,
    ) -> Vec<Area> {
        // Clear previous tree
        self.tree = TaffyTree::new();

        if widgets.is_empty() {
            return vec![];
        }

        // Create Taffy nodes for each widget
        let mut nodes = Vec::new();
        for widget in widgets {
            let constraints = widget.constraints();
            let style = self.constraints_to_style(constraints, direction);
            let node = self.tree.new_leaf(style).unwrap();
            nodes.push(node);
        }

        // Create container node
        let flex_direction = match direction {
            super::container::Direction::Vertical => FlexDirection::Column,
            super::container::Direction::Horizontal => FlexDirection::Row,
        };

        let gap_size = gap as f32;
        let container_style = Style {
            display: Display::Flex,
            flex_direction,
            gap: Size {
                width: length(gap_size),
                height: length(gap_size),
            },
            // IMPORTANT: Set the container size to match available space
            size: Size {
                width: length(available.width() as f32),
                height: length(available.height() as f32),
            },
            ..Default::default()
        };

        let container = self
            .tree
            .new_with_children(container_style, &nodes)
            .unwrap();

        // Compute layout
        let available_size = Size {
            width: AvailableSpace::Definite(available.width() as f32),
            height: AvailableSpace::Definite(available.height() as f32),
        };

        self.tree.compute_layout(container, available_size).unwrap();

        // Extract computed positions
        let mut results = Vec::new();
        for node in nodes.iter() {
            let layout = self.tree.layout(*node).unwrap();
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
    }

    fn constraints_to_style(
        &self,
        constraints: Constraints,
        direction: super::container::Direction,
    ) -> Style {
        let (width, height) = match direction {
            super::container::Direction::Vertical => {
                // In vertical layout, width should always fill the container
                let width = Dimension::percent(1.0);

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
            super::container::Direction::Horizontal => {
                // In horizontal layout, height fills, width is constrained
                let width = if let Some(max) = constraints.max_width {
                    if max == constraints.min_width {
                        Dimension::length(constraints.min_width as f32)
                    } else if constraints.flex.is_some() {
                        Dimension::percent(1.0)
                    } else {
                        Dimension::auto()
                    }
                } else if constraints.flex.is_some() {
                    Dimension::percent(1.0)
                } else {
                    Dimension::length(constraints.min_width as f32)
                };

                let height = Dimension::length(constraints.min_height as f32);

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
    use crate::layout::container::Direction;
    use crate::widget::Label;

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
        let results = bridge.compute_layout(&widgets, available, Direction::Vertical, 1);

        // Assert reasonable values
        assert!(
            results[0].width() > 0,
            "First child should have non-zero width"
        );
        assert!(
            results[0].height() > 0,
            "First child should have non-zero height"
        );
        assert!(
            results[1].width() > 0,
            "Second child should have non-zero width"
        );
        assert!(
            results[1].height() > 0,
            "Second child should have non-zero height"
        );
    }
}
