//! Hello World Example
//!
//! The simplest possible TUI application.
//! Demonstrates minimal setup with a single label.
//!
//! Run with: cargo run --example hello

use tui::prelude::*;

/// Minimal state - just a greeting
#[derive(Debug)]
struct State {
    greeting: String,
}

/// No messages needed for a static view
#[derive(Clone, Debug)]
enum Message {}

/// Update function - no changes for this example
fn update(_state: &mut State, _msg: Message) {}

/// View function - displays a simple greeting
fn view(state: &State) -> Container<Message> {
    col()
        .gap(1)
        .padding(Padding::new(4, 4, 2, 2))
        .child(label(&state.greeting).fg(Color::Green).bold())
        .child(label("Welcome to the rsille!").fg(Color::Cyan))
        .child(label("Press Esc to quit").fg(Color::Indexed(8)))
}

fn main() -> Result<()> {
    let app = App::new(State {
        greeting: "👋 Hello, World!".to_string(),
    });
    app.run_inline(update, view)?;
    Ok(())
}
