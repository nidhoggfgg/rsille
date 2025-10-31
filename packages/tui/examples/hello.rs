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
    Container::vertical(vec![
        Label::new(&state.greeting)
            .style(Style::default().fg(Color::Green).bold())
            .into(),
        Label::new("Welcome to the TUI Framework!")
            .style(Style::default().fg(Color::Cyan))
            .into(),
        Label::new("Press Esc or q to quit")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
    ])
    .gap(1)
    .padding(Padding::new(4, 4, 2, 2))
}

fn main() -> Result<()> {
    let app = App::new(State {
        greeting: "👋 Hello, World!".to_string(),
    });
    app.run(update, view)?;
    Ok(())
}

