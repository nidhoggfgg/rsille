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
}

/// Messages for theme switching and interactions
#[derive(Clone, Debug)]
enum Message {
    SwitchToDark,
    SwitchToLight,
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
    }
}

/// View function - displays theme showcase
fn view(state: &State) -> impl Layout<Message> {
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
                .child(label("Dark (d)"))
                .child(label("Light (l)")),
        )
        .child(label("Press Esc to quit"))
}

fn main() -> WidgetResult<()> {
    let app = App::new(State {
        current_theme: "Dark".to_string(),
    })
    .with_theme(Theme::dark());

    app.on_key(KeyCode::Char('d'), || Message::SwitchToDark)
        .on_key(KeyCode::Char('l'), || Message::SwitchToLight)
        .run_inline(update, view)?;
    Ok(())
}
