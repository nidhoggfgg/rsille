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
fn view(state: &AppState) -> impl Layout<Message> {
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

    col()
        .gap(0)
        .child(label(format!("{} {}", spinner, state.message)).fg(Color::Cyan))
        .child(label(bar).fg(Color::Green))
        .when(!state.items.is_empty(), |c| {
            c.children(
                state
                    .items
                    .iter()
                    .map(|item| label(format!("  • {}", item)).fg(Color::Yellow)),
            )
        })
        .child(label("Controls:").fg(Color::Magenta))
        .children([
            label("  t - tick progress").fg(Color::Indexed(8)),
            label("  a - add item (watch height grow!)").fg(Color::Indexed(8)),
            label("  r - remove item (watch height shrink!)").fg(Color::Indexed(8)),
            label("  Esc - quit").fg(Color::Indexed(8)),
        ])
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
    app.on_key(KeyCode::Char('t'), || Message::Tick)
        .on_key(KeyCode::Char('a'), || Message::AddItem)
        .on_key(KeyCode::Char('r'), || Message::RemoveItem)
        .run_inline(update, view)?;

    Ok(())
}
