//! Bridge to Taffy layout engine

use crate::layout::Constraints;
use crate::widget::{AnyWidget, Rect as TuiRect};
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
    pub fn compute_layout<M>(
        &mut self,
        widgets: &[AnyWidget<M>],
        available: TuiRect,
        direction: super::container::Direction,
        gap: u16,
    ) -> Vec<TuiRect> {
        // Clear previous tree
        self.tree = TaffyTree::new();

        if widgets.is_empty() {
            return vec![];
        }

        // Create Taffy nodes for each widget
        let mut nodes = Vec::new();
        for widget in widgets {
            let constraints = widget.as_widget().constraints();
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
                width: length(available.width as f32),
                height: length(available.height as f32),
            },
            ..Default::default()
        };

        let container = self
            .tree
            .new_with_children(container_style, &nodes)
            .unwrap();

        // Compute layout
        let available_size = Size {
            width: AvailableSpace::Definite(available.width as f32),
            height: AvailableSpace::Definite(available.height as f32),
        };

        self.tree.compute_layout(container, available_size).unwrap();

        // Extract computed positions
        let mut results = Vec::new();
        for node in nodes.iter() {
            let layout = self.tree.layout(*node).unwrap();
            results.push(TuiRect::new(
                available.x + layout.location.x as u16,
                available.y + layout.location.y as u16,
                layout.size.width as u16,
                layout.size.height as u16,
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

        Style {
            size: Size { width, height },
            flex_grow,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::{Label, Widget, AnyWidget};
    use crate::layout::container::Direction;

    #[test]
    fn test_taffy_layout_calculation() {
        let mut bridge = TaffyBridge::new();

        // Create test labels with known sizes
        let label1 = Label::new("Hello"); // 5 chars
        let label2 = Label::new("World!"); // 6 chars

        let constraints1 = label1.constraints();
        let constraints2 = label2.constraints();

        println!("Label1 constraints: {:?}", constraints1);
        println!("Label2 constraints: {:?}", constraints2);

        // Available area: 80x24 terminal
        let available = TuiRect::new(0, 0, 80, 24);

        // Create widgets
        let widgets: Vec<AnyWidget> = vec![
            label1.into(),
            label2.into(),
        ];

        // Compute layout
        let results = bridge.compute_layout(&widgets, available, Direction::Vertical, 1);

        println!("Layout results:");
        for (i, rect) in results.iter().enumerate() {
            println!("  Child {}: {}x{} at ({}, {})", i, rect.width, rect.height, rect.x, rect.y);
        }

        // Assert reasonable values
        assert!(results[0].width > 0, "First child should have non-zero width");
        assert!(results[0].height > 0, "First child should have non-zero height");
        assert!(results[1].width > 0, "Second child should have non-zero width");
        assert!(results[1].height > 0, "Second child should have non-zero height");
    }
}
