//! Counter Example
//!
//! A simple counter demonstrating basic TUI framework usage
//! with buttons and state management.
//!
//! Run with: cargo run --example counter

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct Counter {
    value: i32,
}

/// Messages that can update the state
#[derive(Clone, Debug)]
enum Message {
    Increment,
    Decrement,
    Reset,
}

/// Update function - handles state changes
fn update(state: &mut Counter, msg: Message) {
    match msg {
        Message::Increment => state.value += 1,
        Message::Decrement => state.value -= 1,
        Message::Reset => state.value = 0,
    }
}

/// View function - renders the UI from current state
fn view(state: &Counter) -> Container<Message> {
    Container::vertical(vec![
        // Title
        Label::new("🔢 Counter Example")
            .style(Style::default().fg(Color::Cyan).bold())
            .into(),
        // Current value display
        Label::new(format!("Count: {}", state.value))
            .style(Style::default().fg(Color::Green))
            .into(),
        // Keyboard controller for Up/Down keys and R for reset
        KeyboardController::new()
            .on_up(|| Message::Increment)
            .on_down(|| Message::Decrement)
            .on_key(KeyCode::Char('r'), || Message::Reset)
            .into(),
        // Instructions
        Label::new("Press Up/Down arrows to change count, R to reset, Esc/q to quit")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
    ])
    .gap(1)
    .padding(Padding::new(2, 2, 1, 1))
}

fn main() -> Result<()> {
    let app = App::new(Counter { value: 0 });
    app.run(update, view)?;
    Ok(())
}
