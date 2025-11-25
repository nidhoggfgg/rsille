//! Grid Item Spanning Example
//!
//! Demonstrates:
//! - child_span() for spanning multiple rows/columns
//! - child_at() for explicit positioning
//! - child_area() for simple positioning
//! - Complex layouts with overlapping areas
//!
//! Controls:
//! - q/Esc: Quit
//!
//! Run with: cargo run --example grid_spanning

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct State {
    layout_type: usize,
}

/// Messages from UI interactions
#[derive(Clone, Debug)]
enum Message {
    NextLayout,
    Quit,
}

/// Update function - handles messages
fn update(state: &mut State, msg: Message) {
    match msg {
        Message::NextLayout => {
            state.layout_type = (state.layout_type + 1) % 3;
        }
        Message::Quit => {
            std::process::exit(0);
        }
    }
}

/// Create a colored panel with text
fn panel(text: &str, color: Color) -> Container<Message> {
    col()
        .border(BorderStyle::Rounded)
        .padding(Padding::new(1, 1, 0, 0))
        .style(Style::default().bg(color))
        .child(label(text).fg(Color::Black).bold())
}

/// Layout 1: Dashboard with header spanning all columns
fn layout_dashboard() -> Container<Message> {
    col()
        .gap(1)
        .child(
            label("Layout 1: Dashboard with Header Span")
                .fg(Color::Cyan)
                .bold(),
        )
        .child(
            grid()
                .columns("1fr 1fr 1fr")
                .rows("auto auto auto")
                .gap(1)
                .border(BorderStyle::Rounded)
                // Header spans all 3 columns
                .child_span(panel("Header (spans 3 columns)", Color::Blue), 1, 1, 3, 1)
                // Main content (column 1-2, row 2-3)
                .child_span(
                    panel("Main Content\n(spans 2 columns, 2 rows)", Color::Green),
                    1,
                    2,
                    2,
                    2,
                )
                // Sidebar (column 3, row 2-3)
                .child_span(panel("Sidebar\n(spans 2 rows)", Color::Yellow), 3, 2, 1, 2),
        )
}

/// Layout 2: Magazine style with featured article
fn layout_magazine() -> Container<Message> {
    col()
        .gap(1)
        .child(
            label("Layout 2: Magazine with Featured Article")
                .fg(Color::Cyan)
                .bold(),
        )
        .child(
            grid()
                .columns("1fr 1fr 1fr 1fr")
                .rows("auto auto auto")
                .gap(1)
                .border(BorderStyle::Rounded)
                // Featured article spans 2x2
                .child_span(panel("Featured Article\n(2x2)", Color::Magenta), 1, 1, 2, 2)
                // Side articles
                .child_area(panel("Article 1", Color::Cyan), 3, 1)
                .child_area(panel("Article 2", Color::Yellow), 4, 1)
                .child_area(panel("Article 3", Color::Green), 3, 2)
                .child_area(panel("Article 4", Color::Blue), 4, 2)
                // Footer spans all columns
                .child_span(
                    panel("Footer (spans 4 columns)", Color::Indexed(240)),
                    1,
                    3,
                    4,
                    1,
                ),
        )
}

/// Layout 3: Complex app layout
fn layout_complex() -> Container<Message> {
    col()
        .gap(1)
        .child(label("Layout 3: Complex App Layout").fg(Color::Cyan).bold())
        .child(
            grid()
                .columns("15 1fr 1fr 15")
                .rows("auto 1fr auto")
                .gap(1)
                .border(BorderStyle::Rounded)
                // Top navigation bar spans all columns
                .child_span(
                    panel("Navigation Bar (spans all 4 columns)", Color::Blue),
                    1,
                    1,
                    4,
                    1,
                )
                // Left sidebar (row 2 only)
                .child_area(panel("Left\nMenu", Color::Yellow), 1, 2)
                // Main content area (columns 2-3, row 2)
                .child_span(
                    panel("Main Content Area\n(spans 2 columns)", Color::Green),
                    2,
                    2,
                    2,
                    1,
                )
                // Right panel (row 2 only)
                .child_area(panel("Right\nPanel", Color::Cyan), 4, 2)
                // Status bar spans all columns
                .child_span(
                    panel("Status Bar (spans all 4 columns)", Color::Indexed(236)),
                    1,
                    3,
                    4,
                    1,
                ),
        )
}

/// View function - builds the UI
fn view(state: &State) -> Container<Message> {
    let layout = match state.layout_type {
        0 => layout_dashboard(),
        1 => layout_magazine(),
        2 => layout_complex(),
        _ => unreachable!(),
    };

    col()
        .padding(Padding::new(2, 2, 1, 1))
        .gap(2)
        // Title
        .child(label("Grid Item Spanning Demo").fg(Color::Green).bold())
        .child(label(
            "Demonstrates child_span(), child_at(), and child_area()",
        ))
        .child(spacer().height(1))
        // Current layout
        .child(layout)
        .child(spacer().height(1))
        // Controls
        .child(button("Next Layout (Space)").on_click(|| Message::NextLayout))
        .child(spacer().height(1))
        .child(label("Press 'q' or Esc to quit").fg(Color::Indexed(8)))
        // Keyboard controller for global shortcuts
        .child(
            keyboard_controller()
                .on(' ', || Message::NextLayout)
                .on('q', || Message::Quit),
        )
}

fn main() -> Result<()> {
    let app = App::new(State { layout_type: 0 });
    app.run(update, view)?;
    Ok(())
}
