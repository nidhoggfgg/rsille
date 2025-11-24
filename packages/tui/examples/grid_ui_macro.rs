//! Grid Layout Example - Using ui! Macro
//!
//! This is the same as grid.rs, but using the new `ui!` macro
//! for a more concise syntax with less type annotation noise.
//!
//! Demonstrates:
//! - Using ui! macro for cleaner code
//! - Less verbose than builder pattern
//! - Same functionality, better editor experience
//!
//! Controls:
//! - q/Esc: Quit
//!
//! Run with: cargo run --example grid_ui_macro

use tui::prelude::*;
use tui::ui;

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

/// View function - builds the UI using the new ui! macro
fn view(state: &State) -> Container<Message> {
    ui! {
        col [padding: (2, 2, 1, 1), gap: 1] {
            label("Grid Layout Demo - UI Macro Version") [fg: Color::Cyan, bold: true],
            label(""),
            label("Example 1: Equal columns (1fr each)") [fg: Color::Green, bold: true],
            grid [columns: "1fr 1fr 1fr", rows: "auto", gap: 1, border: BorderStyle::Rounded] {
                label("Column 1") [fg: Color::Yellow, bg: Color::Indexed(235)],
                label("Column 2") [fg: Color::Yellow, bg: Color::Indexed(235)],
                label("Column 3") [fg: Color::Yellow, bg: Color::Indexed(235)],
            },
            label(""),
            label("Example 2: Different widths (1fr 2fr 1fr)") [fg: Color::Green, bold: true],
            grid [columns: "1fr 2fr 1fr", rows: "auto", gap: 1, border: BorderStyle::Rounded] {
                label("Narrow") [fg: Color::Magenta, bg: Color::Indexed(235)],
                label("Wide (2x)") [fg: Color::Magenta, bg: Color::Indexed(235)],
                label("Narrow") [fg: Color::Magenta, bg: Color::Indexed(235)],
            },
            label(""),
            label("Example 3: Fixed + flexible (20 1fr 20)") [fg: Color::Green, bold: true],
            grid [columns: "20 1fr 20", rows: "auto", gap: 1, border: BorderStyle::Rounded] {
                label("Left 20") [fg: Color::Blue, bg: Color::Indexed(235)],
                label("Flexible middle") [fg: Color::Blue, bg: Color::Indexed(235)],
                label("Right 20") [fg: Color::Blue, bg: Color::Indexed(235)],
            },
            label(""),
            label("Example 4: 2x2 Grid") [fg: Color::Green, bold: true],
            grid [columns: "1fr 1fr", rows: "auto auto", gap: 1, border: BorderStyle::Rounded] {
                label("Cell (1,1)") [fg: Color::Cyan, bg: Color::Indexed(235)],
                label("Cell (1,2)") [fg: Color::Cyan, bg: Color::Indexed(235)],
                label("Cell (2,1)") [fg: Color::Cyan, bg: Color::Indexed(235)],
                label("Cell (2,2)") [fg: Color::Cyan, bg: Color::Indexed(235)],
            },
            label(""),
            label("Example 5: Interactive Grid") [fg: Color::Green, bold: true],
            grid [columns: "1fr 1fr 1fr", rows: "auto auto", gap: 1, border: BorderStyle::Rounded] {
                button("-") [on_click: || Message::Decrement],
                label(format!("Count: {}", state.counter)) [fg: Color::Yellow],
                button("+") [on_click: || Message::Increment],
                label("Counter controls"),
                label(""),
                label(""),
            },
            label(""),
            label("Press 'q' or Esc to quit") [fg: Color::Indexed(8)],
            keyboard_controller().on('q', || Message::Quit),
        }
    }
}

fn main() -> Result<()> {
    let app = App::new(State { counter: 0 });
    app.run(update, view)?;
    Ok(())
}
