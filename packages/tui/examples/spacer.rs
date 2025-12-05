//! Example demonstrating the Spacer widget for layout spacing

use tui::prelude::*;

/// Simple state for the example
#[derive(Debug)]
struct State;

/// Update function - no changes needed
fn update(_state: &mut State, _msg: ()) {}

/// View function - demonstrates spacer usage
fn view(_state: &State) -> impl Layout<()> {
    col()
        .gap(0)
        // Title
        .child(label("Spacer Widget Demo").bold().fg(Color::Cyan))
        // Vertical spacer
        .child(spacer().height(1))
        // Content with horizontal spacers
        .child(
            row()
                .child(label("Left").fg(Color::Green))
                .child(spacer().width(5).height(1))
                .child(label("Middle").fg(Color::Yellow))
                .child(spacer().width(5).height(1))
                .child(label("Right").fg(Color::Red)),
        )
        // Vertical spacer
        .child(spacer().height(2))
        // Fixed size spacer example
        .child(
            col()
                .child(label("Fixed spacer (10x2) below:"))
                .child(spacer().fixed(10, 2))
                .child(label("Text after spacer")),
        )
        // Flexible spacer that fills remaining space
        .child(spacer().fill())
        // Footer at bottom
        .child(label("Press Esc to exit").fg(Color::Indexed(8)))
}

fn main() -> WidgetResult<()> {
    let app = App::new(State);
    app.run_inline(update, view)?;
    Ok(())
}
