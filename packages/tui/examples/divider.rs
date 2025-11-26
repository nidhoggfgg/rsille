//! Divider widget example
//!
//! This example demonstrates the usage of the divider widget,
//! showing both horizontal and vertical dividers with different variants,
//! and dividers with text labels.

use tui::prelude::*;

/// Simple state for the example
#[derive(Debug)]
struct State;

/// No messages needed
#[derive(Clone, Debug)]
enum Message {}

/// Update function - no changes needed
fn update(_state: &mut State, _msg: Message) {}

/// View function - demonstrates divider usage
fn view(_state: &State) -> impl Layout<Message> {
    col()
        .gap(1)
        // Title
        .child(
            label("Divider Examples (Press Esc to exit)")
                .bold()
                .fg(Color::Cyan),
        )
        .child(divider().horizontal().variant(DividerVariant::Heavy))
        // Main content in two columns
        .child(
            row()
                .gap(1)
                // Left column: Dividers with text
                .child(
                    col()
                        .gap(1)
                        .child(label("Dividers with Text").bold().fg(Color::Yellow))
                        .child(divider().horizontal().text("Centered Text"))
                        .child(
                            divider()
                                .horizontal()
                                .text("Left Aligned")
                                .text_position(DividerTextPosition::Left),
                        )
                        .child(
                            divider()
                                .horizontal()
                                .text("Right Aligned")
                                .text_position(DividerTextPosition::Right),
                        )
                        .child(spacer().height(1))
                        .child(label("Different Variants").bold().fg(Color::Yellow))
                        .child(
                            divider()
                                .horizontal()
                                .text("Heavy")
                                .variant(DividerVariant::Heavy),
                        )
                        .child(
                            divider()
                                .horizontal()
                                .text("Double")
                                .variant(DividerVariant::Double),
                        )
                        .child(
                            divider()
                                .horizontal()
                                .text("Dashed")
                                .variant(DividerVariant::Dashed),
                        )
                        .child(
                            divider()
                                .horizontal()
                                .text("◆ Custom ◆")
                                .variant(DividerVariant::Dotted)
                                .text_spacing(2),
                        )
                        .child(spacer().height(1))
                        .child(label("Vertical Dividers:").bold().fg(Color::Yellow))
                        .child(
                            row()
                                .gap(0)
                                .child(
                                    col()
                                        .child(label("Solid"))
                                        .child(label("Line"))
                                        .child(label("Demo")),
                                )
                                .child(divider().vertical().variant(DividerVariant::Solid))
                                .child(spacer().width(1).height(1))
                                .child(
                                    col()
                                        .child(label("Dashed"))
                                        .child(label("Line"))
                                        .child(label("Demo")),
                                )
                                .child(divider().vertical().variant(DividerVariant::Dashed))
                                .child(spacer().width(1).height(1))
                                .child(
                                    col()
                                        .child(label("Heavy"))
                                        .child(label("Line"))
                                        .child(label("Demo")),
                                )
                                .child(divider().vertical().variant(DividerVariant::Heavy))
                                .child(spacer().width(1).height(1))
                                .child(
                                    col()
                                        .child(label("Double"))
                                        .child(label("Line"))
                                        .child(label("Demo")),
                                ),
                        ),
                )
                // Vertical divider separator
                .child(divider().vertical().variant(DividerVariant::Heavy))
                // Right column: Basic variants
                .child(
                    col()
                        .gap(1)
                        .child(label("Basic Variants").bold().fg(Color::Yellow))
                        .child(label("Solid:").bold())
                        .child(divider().horizontal().variant(DividerVariant::Solid))
                        .child(label("Dashed:").bold())
                        .child(divider().horizontal().variant(DividerVariant::Dashed))
                        .child(label("Dotted:").bold())
                        .child(divider().horizontal().variant(DividerVariant::Dotted))
                        .child(label("Heavy:").bold())
                        .child(divider().horizontal().variant(DividerVariant::Heavy))
                        .child(label("Double:").bold())
                        .child(divider().horizontal().variant(DividerVariant::Double))
                        .child(label("Faded:").bold())
                        .child(divider().horizontal().variant(DividerVariant::Faded))
                        .child(spacer().height(1)),
                ),
        )
        .child(
            divider()
                .horizontal()
                .text("End")
                .variant(DividerVariant::Heavy),
        )
}

fn main() -> Result<()> {
    let app = App::new(State);
    app.run_inline(update, view)?;
    Ok(())
}
