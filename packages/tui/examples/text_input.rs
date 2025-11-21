//! TextInput Component Example
//!
//! Demonstrates:
//! - Basic text input with placeholder
//! - Text editing (insert, delete, backspace)
//! - Cursor navigation (arrows, Home, End)
//! - Focus management
//! - Submit on Enter
//! - Change event handling
//!
//! Controls:
//! - Type: Enter text
//! - Backspace/Delete: Remove characters
//! - Left/Right: Move cursor
//! - Home/End: Jump to start/end
//! - Tab: Focus next input
//! - Shift+Tab: Focus previous input
//! - Enter: Submit input
//! - Esc: Quit
//!
//! Run with: cargo run --example text_input

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct State {
    /// Name input value
    name: String,
    /// Email input value
    email: String,
    /// Message input value
    message: String,
    /// Submission status
    submitted: Option<String>,
}

/// Messages from UI interactions
#[derive(Clone, Debug)]
enum Message {
    NameChanged(String),
    EmailChanged(String),
    MessageChanged(String),
    Submit,
    Quit,
}

/// Update function - handles messages
fn update(state: &mut State, msg: Message) {
    match msg {
        Message::NameChanged(value) => {
            state.name = value;
        }
        Message::EmailChanged(value) => {
            state.email = value;
        }
        Message::MessageChanged(value) => {
            state.message = value;
        }
        Message::Submit => {
            state.submitted = Some(format!(
                "Submitted: {} <{}> - {}",
                state.name, state.email, state.message
            ));
        }
        Message::Quit => {
            std::process::exit(0);
        }
    }
}

/// View function - builds the UI
fn view(state: &State) -> Container<Message> {
    col()
        .padding(Padding::new(3, 3, 2, 2))
        .gap(1)
        // Header
        .child(label("Text Input Component Demo").fg(Color::Cyan).bold())
        // Name input
        .child(label("Name:").fg(Color::Indexed(8)))
        .child(
            text_input()
                .placeholder("Enter your name...")
                .value(state.name.clone())
                .on_change(|text| Message::NameChanged(text)),
        )
        // Email input
        .child(label("Email:").fg(Color::Indexed(8)))
        .child(
            text_input()
                .placeholder("your@email.com")
                .value(state.email.clone())
                .on_change(|text| Message::EmailChanged(text)),
        )
        .child(label(""))
        // Message input
        .child(label("Message:").fg(Color::Indexed(8)))
        .child(
            text_input()
                .placeholder("Type your message...")
                .value(state.message.clone())
                .on_change(|text| Message::MessageChanged(text))
                .on_submit(|_| Message::Submit),
        )
        .child(label(""))
        // Submit button
        .child(
            button("Submit")
                .variant(ButtonVariant::Primary)
                .on_click(|| Message::Submit),
        )
        // Disabled input (for demonstration)
        .child(label("Disabled input:").fg(Color::Indexed(8)))
        .child(
            text_input()
                .placeholder("This input is disabled")
                .disabled(true),
        )
        // Status display
        .child(if let Some(ref status) = state.submitted {
            label(status).fg(Color::Green)
        } else {
            label("Fill in the form and press Enter or click Submit").fg(Color::Indexed(8))
        })
        // Footer with instructions
        .child(
            label("Type to edit | Arrows/Home/End to navigate | Tab to switch | Esc to quit")
                .fg(Color::Indexed(8)),
        )
        // Keyboard controller for global shortcuts
        .child(keyboard_controller().on('q', || Message::Quit))
}

fn main() -> Result<()> {
    let app = App::new(State {
        name: String::new(),
        email: String::new(),
        message: String::new(),
        submitted: None,
    });
    app.run_inline(update, view)?;
    Ok(())
}
