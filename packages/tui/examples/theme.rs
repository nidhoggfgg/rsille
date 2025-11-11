//! Theme Switching Example
//!
//! Demonstrates the theme system with runtime theme switching.
//! Shows how widgets automatically apply theme styles.
//!
//! Controls:
//! - Press 'd' to switch to Dark theme
//! - Press 'l' to switch to Light theme
//! - Press Esc to quit
//!
//! Run with: cargo run --example theme

use tui::event::KeyCode;
use tui::prelude::*;
use tui::style::{Theme, ThemeManager};

/// Application state
#[derive(Debug)]
struct State {
    current_theme: String,
    counter: i32,
    checkbox_checked: bool,
    input_text: String,
}

/// Messages for theme switching and interactions
#[derive(Clone, Debug)]
enum Message {
    SwitchToDark,
    SwitchToLight,
    Increment,
    Decrement,
    ToggleCheckbox,
    InputChanged,
}

/// Update function - handles theme switching and interactions
fn update(state: &mut State, msg: Message) {
    match msg {
        Message::SwitchToDark => {
            ThemeManager::global().set_theme(Theme::dark());
            state.current_theme = "Dark".to_string();
        }
        Message::SwitchToLight => {
            ThemeManager::global().set_theme(Theme::light());
            state.current_theme = "Light".to_string();
        }
        Message::Increment => {
            state.counter += 1;
        }
        Message::Decrement => {
            state.counter -= 1;
        }
        Message::ToggleCheckbox => {
            state.checkbox_checked = !state.checkbox_checked;
        }
        Message::InputChanged => {
            // Input change handled by widget itself
        }
    }
}

/// View function - displays theme showcase
fn view(state: &State) -> Container<Message> {
    col()
        .gap(1)
        .padding(Padding::uniform(2))
        .border(BorderStyle::Rounded)
        .child(
            // Header
            label("🎨 Theme System Demo").bold(),
        )
        .child(label(format!("Current Theme: {}", state.current_theme)))
        .child(
            // Theme controls
            row()
                .gap(2)
                .child(label("Switch Theme:"))
                .child(button("Dark (d)").on_click(|| Message::SwitchToDark))
                .child(button("Light (l)").on_click(|| Message::SwitchToLight)),
        )
        .child(
            keyboard_controller()
                .on_key(KeyCode::Char('d'), || Message::SwitchToDark)
                .on_key(KeyCode::Char('l'), || Message::SwitchToLight),
        )
        .child(label("Press Esc to quit"))
}

fn main() -> Result<()> {
    let app = App::new(State {
        current_theme: "Dark".to_string(),
        counter: 0,
        checkbox_checked: false,
        input_text: "Type here...".to_string(),
    })
    .with_theme(Theme::dark());

    app.run_inline(update, view)?;
    Ok(())
}
