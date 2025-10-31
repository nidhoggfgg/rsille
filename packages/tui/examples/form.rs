//! Form Example
//!
//! Demonstrates form widgets including TextInput and Checkbox
//! with validation and state management.
//!
//! Run with: cargo run --example form

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct FormState {
    name: String,
    email: String,
    accept_terms: bool,
    subscribe_newsletter: bool,
}

/// Messages that can update the state
#[derive(Clone, Debug)]
enum Message {
    NameChanged(String),
    EmailChanged(String),
    ToggleTerms,
    ToggleNewsletter,
    Submit,
    Clear,
}

/// Update function - handles state changes
fn update(state: &mut FormState, msg: Message) {
    match msg {
        Message::NameChanged(value) => state.name = value,
        Message::EmailChanged(value) => state.email = value,
        Message::ToggleTerms => state.accept_terms = !state.accept_terms,
        Message::ToggleNewsletter => state.subscribe_newsletter = !state.subscribe_newsletter,
        Message::Submit => {
            if state.accept_terms && !state.name.is_empty() && !state.email.is_empty() {
                println!("Form submitted!");
                println!("Name: {}", state.name);
                println!("Email: {}", state.email);
                println!("Newsletter: {}", state.subscribe_newsletter);
            }
        }
        Message::Clear => {
            state.name.clear();
            state.email.clear();
            state.accept_terms = false;
            state.subscribe_newsletter = false;
        }
    }
}

/// View function - renders the UI from current state
fn view(state: &FormState) -> Container<Message> {
    let can_submit = state.accept_terms && !state.name.is_empty() && !state.email.is_empty();

    Container::vertical(vec![
        // Title
        Label::new("📝 Registration Form")
            .style(Style::default().fg(Color::Cyan).bold())
            .into(),
        // Name field
        Label::new("Name:")
            .style(Style::default().fg(Color::Yellow))
            .into(),
        TextInput::new(&state.name)
            .on_change(|| Message::NameChanged(String::new())) // Will need to improve this
            .into(),
        // Email field
        Label::new("Email:")
            .style(Style::default().fg(Color::Yellow))
            .into(),
        TextInput::new(&state.email)
            .on_change(|| Message::EmailChanged(String::new())) // Will need to improve this
            .into(),
        // Checkboxes
        Checkbox::new("I accept the terms and conditions", state.accept_terms)
            .on_toggle(|| Message::ToggleTerms)
            .into(),
        Checkbox::new("Subscribe to newsletter", state.subscribe_newsletter)
            .on_toggle(|| Message::ToggleNewsletter)
            .into(),
        // Action buttons
        Container::horizontal(vec![
            Button::new(if can_submit { "[ Submit ]" } else { "[ Submit (disabled) ]" })
                .style(if can_submit {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Indexed(8))
                })
                .on_click(|| Message::Submit)
                .into(),
            Button::new("[ Clear ]")
                .on_click(|| Message::Clear)
                .into(),
        ])
        .gap(2)
        .into(),
        // Instructions
        Label::new("Tab: navigate | Enter: click/toggle | Esc/q: quit")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
    ])
    .gap(1)
    .padding(Padding::new(2, 2, 1, 1))
}

fn main() -> Result<()> {
    let app = App::new(FormState {
        name: String::new(),
        email: String::new(),
        accept_terms: false,
        subscribe_newsletter: false,
    });
    app.run(update, view)?;
    Ok(())
}

