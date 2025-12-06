//! Hello World Example
//!
//! The simplest possible TUI application.
//! Demonstrates minimal setup with a single label.
//!
//! Controls:
//! - Esc: Quit (default quit key)
//!
//! Run with: cargo run --example hello

use tui::prelude::*;

/// Minimal state - just a greeting
#[derive(Debug)]
struct State {
    greeting: String,
}

/// Update function - no changes for this example
fn update(_state: &mut State, _msg: ()) {}

/// View function - displays a simple greeting
fn view(state: &State) -> impl Layout<()> {
    col()
        .gap(1)
        .padding(Padding::new(4, 4, 2, 2))
        .child(label(&state.greeting).fg(Color::Green).bold())
        .child(label("Welcome to the rsille!").fg(Color::Cyan))
        .child(label("Press Esc to quit").fg(Color::Indexed(8)))
}

fn main() -> WidgetResult<()> {
    let app = App::new(State {
        greeting: "👋 Hello, World!".to_string(),
    });
    // The app now has a default quit key (Esc) built-in
    app.run_inline(update, view)?;
    Ok(())
}
