//! Button Component with Focus Navigation Example
//!
//! Demonstrates:
//! - All button variants (Primary, Secondary, Ghost, Link, Destructive)
//! - Focus navigation with Tab/Shift+Tab
//! - Keyboard activation with Enter/Space
//! - Click event handling
//! - Visual focus indicators
//!
//! Controls:
//! - Tab: Focus next button
//! - Shift+Tab: Focus previous button
//! - Enter/Space: Activate focused button
//! - Mouse Click: Activate button
//! - Esc: Quit
//!
//! Run with: cargo run --example button

use tui::{prelude::*, widget::common::StatefulWidgetBuilder};

/// Application state
#[derive(Debug)]
struct State {
    /// Last clicked button
    last_clicked: Option<String>,
    /// Click count
    click_count: usize,
}

/// Messages from UI interactions
#[derive(Clone, Debug)]
enum Message {
    ButtonClicked(String),
    Quit,
}

/// Update function - handles messages
fn update(state: &mut State, msg: Message) {
    match msg {
        Message::ButtonClicked(button_name) => {
            state.last_clicked = Some(button_name);
            state.click_count += 1;
        }
        Message::Quit => {
            std::process::exit(0);
        }
    }
}

/// View function - builds the UI
fn view(state: &State) -> impl Layout<Message> {
    col()
        .padding(Padding::new(3, 3, 2, 2))
        .gap(1)
        // Header
        .child(
            label("Button Variants & Focus Navigation")
                .fg(Color::Cyan)
                .bold(),
        )
        .child(spacer().height(1))
        // Status display
        .child(
            label(format!(
                "Clicks: {} | Last: {}",
                state.click_count,
                state.last_clicked.as_deref().unwrap_or("None")
            ))
            .fg(Color::Green),
        )
        .child(spacer().height(1))
        // Primary Button
        .child(
            row()
                .gap(2)
                .child(label("Primary:").fg(Color::Indexed(8)))
                .child(
                    button("Submit")
                        .variant(ButtonVariant::Primary)
                        .on_click(|| Message::ButtonClicked("Submit".to_string())),
                ),
        )
        // Secondary Button
        .child(
            row()
                .gap(2)
                .child(label("Secondary:").fg(Color::Indexed(8)))
                .child(
                    button("Cancel")
                        .variant(ButtonVariant::Secondary)
                        .on_click(|| Message::ButtonClicked("Cancel".to_string())),
                ),
        )
        // Ghost Button
        .child(
            row()
                .gap(2)
                .child(label("Ghost:").fg(Color::Indexed(8)))
                .child(
                    button("Optional")
                        .variant(ButtonVariant::Ghost)
                        .on_click(|| Message::ButtonClicked("Optional".to_string())),
                ),
        )
        // Link Button
        .child(
            row()
                .gap(2)
                .child(label("Link:").fg(Color::Indexed(8)))
                .child(
                    button("Learn More")
                        .variant(ButtonVariant::Link)
                        .on_click(|| Message::ButtonClicked("Learn More".to_string())),
                ),
        )
        // Destructive Button
        .child(
            row()
                .gap(2)
                .child(label("Destructive:").fg(Color::Indexed(8)))
                .child(
                    button("Delete")
                        .variant(ButtonVariant::Destructive)
                        .on_click(|| Message::ButtonClicked("Delete".to_string())),
                ),
        )
        // Custom Style
        .child(
            row()
                .gap(2)
                .child(label("Custom:").fg(Color::Indexed(8)))
                .child(
                    button("Custom")
                        .variant(ButtonVariant::Primary)
                        .style(Style::default().fg(Color::Black).bg(Color::Cyan))
                        .on_click(|| Message::ButtonClicked("Custom".to_string())),
                ),
        )
        // Disabled (should not be focusable)
        .child(
            row()
                .gap(2)
                .child(label("Disabled:").fg(Color::Indexed(8)))
                .child(
                    button("Disabled")
                        .variant(ButtonVariant::Primary)
                        .disabled(true),
                ),
        )
        // Footer with instructions
        .child(spacer().height(1))
        .child(
            label("Tab/Shift+Tab: Navigate | Enter/Space: Click | Esc: Quit").fg(Color::Indexed(8)),
        )
}

fn main() -> Result<()> {
    let app = App::new(State {
        last_clicked: None,
        click_count: 0,
    });
    app.on_key(KeyCode::Char('q'), || Message::Quit)
        .run_inline(update, view)?;
    Ok(())
}
