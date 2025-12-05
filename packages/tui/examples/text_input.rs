//! TextInput Component Example
//!
//! Demonstrates:
//! - Different text input variants (Default, Borderless, Password)
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
    /// Password input value
    password: String,
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
    PasswordChanged(String),
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
        Message::PasswordChanged(value) => {
            state.password = value;
        }
        Message::MessageChanged(value) => {
            state.message = value;
        }
        Message::Submit => {
            state.submitted = Some(format!(
                "Submitted: {} <{}> (password: {} chars)",
                state.name,
                state.email,
                state.password.len()
            ));
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
        .child(label("Text Input Variants Demo").fg(Color::Cyan).bold())
        .child(spacer().height(1))
        // Default variant
        .child(label("Default Variant:").fg(Color::Indexed(8)))
        .child(
            text_input()
                .variant(TextInputVariant::Default)
                .placeholder("Enter your name...")
                .value(state.name.clone())
                .on_change(|text| Message::NameChanged(text)),
        )
        // Default variant for email
        .child(label("Email (Default):").fg(Color::Indexed(8)))
        .child(
            text_input()
                .variant(TextInputVariant::Default)
                .placeholder("your@email.com")
                .value(state.email.clone())
                .on_change(|text| Message::EmailChanged(text)),
        )
        // Password variant
        .child(label("Password Variant (masked):").fg(Color::Indexed(8)))
        .child(
            text_input()
                .variant(TextInputVariant::Password)
                .placeholder("Enter password...")
                .value(state.password.clone())
                .on_change(|text| Message::PasswordChanged(text)),
        )
        // Borderless variant
        .child(label("Borderless Variant:").fg(Color::Indexed(8)))
        .child(
            text_input()
                .variant(TextInputVariant::Borderless)
                .placeholder("Type your message...")
                .value(state.message.clone())
                .on_change(|text| Message::MessageChanged(text))
                .on_submit(|_| Message::Submit),
        )
        .child(spacer().height(1))
        // Submit button
        .child(
            button("Submit")
                .variant(ButtonVariant::Primary)
                .on_click(|| Message::Submit),
        )
        // Disabled input (for demonstration)
        .child(label("Disabled Input:").fg(Color::Indexed(8)))
        .child(
            text_input()
                .variant(TextInputVariant::Default)
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
}

fn main() -> WidgetResult<()> {
    let app = App::new(State {
        name: String::new(),
        email: String::new(),
        password: String::new(),
        message: String::new(),
        submitted: None,
    });
    app.on_key(KeyCode::Char('q'), || Message::Quit)
        .run_inline(update, view)?;
    Ok(())
}
