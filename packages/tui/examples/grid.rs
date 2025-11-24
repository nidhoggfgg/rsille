//! Grid Layout Example
//!
//! Demonstrates:
//! - Basic grid layout with fixed columns and rows
//! - Fr (fraction) units for flexible sizing
//! - Auto sizing based on content
//! - Gap between grid cells
//! - Padding and borders
//! - Nested grids
//!
//! Controls:
//! - q/Esc: Quit
//!
//! Run with: cargo run --example grid

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct State {
    counter: usize,
}

/// Messages from UI interactions
#[derive(Clone, Debug)]
enum Message {
    Increment,
    Decrement,
    Quit,
}

/// Update function - handles messages
fn update(state: &mut State, msg: Message) {
    match msg {
        Message::Increment => {
            state.counter = state.counter.saturating_add(1);
        }
        Message::Decrement => {
            state.counter = state.counter.saturating_sub(1);
        }
        Message::Quit => {
            std::process::exit(0);
        }
    }
}

/// View function - builds the UI
fn view(state: &State) -> Container<Message> {
    col()
        .padding(Padding::new(2, 2, 1, 1))
        .gap(1)
        // Header
        .child(label("Grid Layout Demo").fg(Color::Cyan).bold())
        .child(label(""))
        // Example 1: Basic 3-column grid
        .child(label("Example 1: Equal columns (1fr each)").fg(Color::Green).bold())
        .child(
            grid()
                .columns("1fr 1fr 1fr") // 3 equal columns
                .rows("auto") // Single row, auto height
                .gap(1)
                .border(BorderStyle::Rounded)
                .child(
                    label("Column 1")
                        .fg(Color::Yellow)
                        .bg(Color::Indexed(235)),
                )
                .child(
                    label("Column 2")
                        .fg(Color::Yellow)
                        .bg(Color::Indexed(235)),
                )
                .child(
                    label("Column 3")
                        .fg(Color::Yellow)
                        .bg(Color::Indexed(235)),
                ),
        )
        .child(label(""))
        // Example 2: Different column widths
        .child(
            label("Example 2: Different widths (1fr 2fr 1fr)")
                .fg(Color::Green)
                .bold(),
        )
        .child(
            grid()
                .columns("1fr 2fr 1fr") // Middle column is 2x wider
                .rows("auto")
                .gap(1)
                .border(BorderStyle::Rounded)
                .child(
                    label("Narrow")
                        .fg(Color::Magenta)
                        .bg(Color::Indexed(235)),
                )
                .child(
                    label("Wide (2x)")
                        .fg(Color::Magenta)
                        .bg(Color::Indexed(235)),
                )
                .child(
                    label("Narrow")
                        .fg(Color::Magenta)
                        .bg(Color::Indexed(235)),
                ),
        )
        .child(label(""))
        // Example 3: Fixed and flexible columns
        .child(
            label("Example 3: Fixed + flexible (20 1fr 20)")
                .fg(Color::Green)
                .bold(),
        )
        .child(
            grid()
                .columns("20 1fr 20") // Sidebar-content-sidebar layout
                .rows("auto")
                .gap(1)
                .border(BorderStyle::Rounded)
                .child(
                    label("Left 20")
                        .fg(Color::Blue)
                        .bg(Color::Indexed(235)),
                )
                .child(
                    label("Flexible middle")
                        .fg(Color::Blue)
                        .bg(Color::Indexed(235)),
                )
                .child(
                    label("Right 20")
                        .fg(Color::Blue)
                        .bg(Color::Indexed(235)),
                ),
        )
        .child(label(""))
        // Example 4: Multiple rows
        .child(label("Example 4: 2x2 Grid").fg(Color::Green).bold())
        .child(
            grid()
                .columns("1fr 1fr")
                .rows("auto auto")
                .gap(1)
                .border(BorderStyle::Rounded)
                .child(
                    label("Cell (1,1)")
                        .fg(Color::Cyan)
                        .bg(Color::Indexed(235)),
                )
                .child(
                    label("Cell (1,2)")
                        .fg(Color::Cyan)
                        .bg(Color::Indexed(235)),
                )
                .child(
                    label("Cell (2,1)")
                        .fg(Color::Cyan)
                        .bg(Color::Indexed(235)),
                )
                .child(
                    label("Cell (2,2)")
                        .fg(Color::Cyan)
                        .bg(Color::Indexed(235)),
                ),
        )
        .child(label(""))
        // Example 5: Interactive grid with buttons
        .child(
            label("Example 5: Interactive Grid")
                .fg(Color::Green)
                .bold(),
        )
        .child(
            grid()
                .columns("1fr 1fr 1fr")
                .rows("auto auto")
                .gap(1)
                .border(BorderStyle::Rounded)
                .child(button("-").on_click(|| Message::Decrement))
                .child(label(format!("Count: {}", state.counter)).fg(Color::Yellow))
                .child(button("+").on_click(|| Message::Increment))
                .child(label("Counter controls"))
                .child(label(""))
                .child(label("")),
        )
        .child(label(""))
        // Footer
        .child(label("Press 'q' or Esc to quit").fg(Color::Indexed(8)))
        // Keyboard controller for global shortcuts
        .child(keyboard_controller().on('q', || Message::Quit))
}

fn main() -> Result<()> {
    let app = App::new(State { counter: 0 });
    app.run(update, view)?;
    Ok(())
}
