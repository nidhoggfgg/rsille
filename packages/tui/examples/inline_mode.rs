//! Inline Mode Example
//!
//! Demonstrates inline mode for non-fullscreen CLI-style interaction.
//! This is similar to modern tools like npm, yarn, cargo, etc.
//!
//! Run with: cargo run --example inline_mode

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct ProgressState {
    spinner_frame: usize,
    progress: f32,
    message: String,
}

/// Messages that can update the state
#[derive(Clone, Debug)]
enum Message {
    Tick,
}

/// Update function - handles state changes
fn update(state: &mut ProgressState, msg: Message) {
    match msg {
        Message::Tick => {
            state.spinner_frame += 1;
            state.progress += 0.01;
            if state.progress >= 1.0 {
                state.progress = 1.0;
                state.message = "Complete!".to_string();
            }
        }
    }
}

/// View function - renders the UI from current state
fn view(state: &ProgressState) -> Container<Message> {
    let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner = spinner_chars[state.spinner_frame % spinner_chars.len()];

    let percentage = (state.progress * 100.0) as u32;
    let filled_width = (state.progress * 30.0) as usize;
    let bar = format!(
        "[{}{}] {}%",
        "█".repeat(filled_width),
        "░".repeat(30 - filled_width),
        percentage
    );

    Container::vertical(vec![
        Label::new(format!("{} {}", spinner, state.message))
            .style(Style::default().fg(Color::Cyan))
            .into(),
        Label::new(bar)
            .style(Style::default().fg(Color::Green))
            .into(),
        Label::new("Press 't' to tick, Esc to quit")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
        // Keyboard controller for manual ticking
        KeyboardController::new()
            .on_key(KeyCode::Char('t'), || Message::Tick)
            .into(),
    ])
    .gap(0)
}

fn main() -> Result<()> {
    let app = App::new(ProgressState {
        spinner_frame: 0,
        progress: 0.0,
        message: "Loading...".to_string(),
    });

    // Use run_inline() instead of run() for inline mode
    app.run_inline(update, view)?;

    Ok(())
}
