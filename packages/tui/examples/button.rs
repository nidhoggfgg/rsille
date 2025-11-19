//! Button Component Example
//!
//! Demonstrates all button variants:
//! - Primary: Main actions
//! - Secondary: Supporting actions
//! - Ghost: Subtle actions
//! - Link: Text-style actions
//! - Destructive: Dangerous actions
//!
//! Run with: cargo run --example button

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct State {}

/// No messages needed for static display
#[derive(Clone, Debug)]
enum Message {}

/// Update function - no changes for this example
fn update(_state: &mut State, _msg: Message) {}

/// View function - builds the UI
fn view(_state: &State) -> Container<Message> {
    col()
        .padding(Padding::new(3, 3, 2, 2))
        .gap(1)
        // Header
        .child(label("Button Variants").fg(Color::Cyan).bold())
        // Primary Button
        .child(
            row()
                .gap(2)
                .child(label("Primary:").fg(Color::Indexed(8)))
                .child(button("Submit").variant(ButtonVariant::Primary)),
        )
        // Secondary Button
        .child(
            row()
                .gap(2)
                .child(label("Secondary:").fg(Color::Indexed(8)))
                .child(button("Cancel").variant(ButtonVariant::Secondary)),
        )
        // Ghost Button
        .child(
            row()
                .gap(2)
                .child(label("Ghost:").fg(Color::Indexed(8)))
                .child(button("Optional").variant(ButtonVariant::Ghost)),
        )
        // Link Button
        .child(
            row()
                .gap(2)
                .child(label("Link:").fg(Color::Indexed(8)))
                .child(button("Learn More").variant(ButtonVariant::Link)),
        )
        // Destructive Button
        .child(
            row()
                .gap(2)
                .child(label("Destructive:").fg(Color::Indexed(8)))
                .child(button("Delete").variant(ButtonVariant::Destructive)),
        )
        // Custom Style
        .child(
            row()
                .gap(2)
                .child(label("Custom:").fg(Color::Indexed(8)))
                .child(
                    button("Custom")
                        .variant(ButtonVariant::Primary)
                        .style(Style::default().fg(Color::Black).bg(Color::Cyan)),
                ),
        )
        // Disabled
        .child(
            row()
                .gap(2)
                .child(label("Disabled:").fg(Color::Indexed(8)))
                .child(button("Disabled").variant(ButtonVariant::Primary).disabled(true)),
        )
        // Footer
        .child(label("Press Esc to quit").fg(Color::Indexed(8)))
}

fn main() -> Result<()> {
    let app = App::new(State {});
    app.run_inline(update, view)?;
    Ok(())
}
