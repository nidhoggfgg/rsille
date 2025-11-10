//! Inline Mode Example
//!
//! Demonstrates inline mode with dynamic height adjustment.
//! The display automatically adapts to content size - try adding/removing items!
//!
//! Run with: cargo run --example inline_mode

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct AppState {
    spinner_frame: usize,
    progress: f32,
    message: String,
    items: Vec<String>,
}

/// Messages that can update the state
#[derive(Clone, Debug)]
enum Message {
    Tick,
    AddItem,
    RemoveItem,
}

/// Update function - handles state changes
fn update(state: &mut AppState, msg: Message) {
    match msg {
        Message::Tick => {
            state.spinner_frame += 1;
            state.progress += 0.01;
            if state.progress >= 1.0 {
                state.progress = 1.0;
                state.message = "Complete!".to_string();
            }
        }
        Message::AddItem => {
            state.items.push(format!("Item {}", state.items.len() + 1));
        }
        Message::RemoveItem => {
            state.items.pop();
        }
    }
}

/// View function - renders the UI from current state
fn view(state: &AppState) -> Container<Message> {
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

    let mut widgets: Vec<AnyWidget<Message>> = vec![
        Label::new(format!("{} {}", spinner, state.message))
            .style(Style::default().fg(Color::Cyan))
            .into(),
        Label::new(bar)
            .style(Style::default().fg(Color::Green))
            .into(),
        Label::new("").into(), // Empty line for spacing
    ];

    // Add dynamic items (demonstrates height adaptation)
    if !state.items.is_empty() {
        let items = state
            .items
            .iter()
            .map(|item| Label::new(format!("  • {}", item)).into())
            .collect();
        widgets.push(Container::vertical(items).into());
        widgets.push(Label::new("").into()); // Empty line for spacing
    }

    // Controls
    widgets.extend(vec![
        Label::new("Controls:")
            .style(Style::default().fg(Color::Magenta))
            .into(),
        Label::new("  t - tick progress")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
        Label::new("  a - add item (watch height grow!)")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
        Label::new("  r - remove item (watch height shrink!)")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
        Label::new("  Esc - quit")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
        // Keyboard controller
        KeyboardController::new()
            .on_key(KeyCode::Char('t'), || Message::Tick)
            .on_key(KeyCode::Char('a'), || Message::AddItem)
            .on_key(KeyCode::Char('r'), || Message::RemoveItem)
            .into(),
    ]);

    Container::vertical(widgets).gap(0)
}

fn main() -> Result<()> {
    let app = App::new(AppState {
        spinner_frame: 0,
        progress: 0.0,
        message: "Loading...".to_string(),
        items: vec![],
    });

    // Use run_inline() - height adjusts automatically based on content!
    // Max height is limited to 50 lines by default (configurable in runtime.rs)
    app.run_inline(update, view)?;

    Ok(())
}
