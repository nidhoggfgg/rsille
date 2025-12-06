//! Quit Key Configuration Example
//!
//! Demonstrates different ways to configure the quit key behavior:
//! 1. Default quit key (Esc)
//! 2. Custom quit key (simple key)
//! 3. Custom quit key with modifiers (Ctrl+C)
//! 4. Disabled quit key (handle quit yourself)
//!
//! Run with: cargo run --example quit_key_config

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct State {
    mode: QuitMode,
}

/// Different quit mode demonstrations
#[derive(Debug, Clone, Copy, PartialEq)]
enum QuitMode {
    Default,
    CustomKey,
    CustomKeyEvent,
    Disabled,
}

/// Messages for custom quit handling
#[derive(Clone, Debug)]
enum Message {
    Quit,
}

/// Update function for disabled mode
fn update_with_quit(state: &mut State, msg: Message) {
    match msg {
        Message::Quit => {
            std::process::exit(0);
        }
    }
}

/// View function for disabled mode
fn view_with_quit(state: &State) -> impl Layout<Message> {
    col()
        .padding(Padding::new(3, 3, 2, 2))
        .gap(1)
        .child(label("Disabled Quit Key").fg(Color::Cyan).bold())
        .child(spacer().height(1))
        .child(label("Press 'q' to quit (handled via on_key)").fg(Color::Green))
        .child(spacer().height(1))
        .child(label("Quit key configuration:").fg(Color::Indexed(8)))
        .child(
            label(
                "  App::new(state).disable_quit_key().on_key(KeyCode::Char('q'), || Message::Quit)",
            )
            .fg(Color::Indexed(8)),
        )
}

/// No-op update for modes that don't need message handling
fn update_noop(_state: &mut State, _msg: ()) {}

/// View function for modes without custom messages
fn view_default(state: &State) -> impl Layout<()> {
    let (title, instructions, config) = match state.mode {
        QuitMode::Default => (
            "Default Quit Key (Esc)",
            "Press Esc to quit (default behavior)",
            "  App::new(state)",
        ),
        QuitMode::CustomKey => (
            "Custom Quit Key (q)",
            "Press 'q' to quit (custom key without modifiers)",
            "  App::new(state).with_quit_key(KeyCode::Char('q'))",
        ),
        QuitMode::CustomKeyEvent => (
            "Custom Quit Key with Modifiers (Ctrl+C)",
            "Press Ctrl+C to quit (custom key with modifiers)",
            "  App::new(state).with_quit_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL))",
        ),
        _ => unreachable!(),
    };

    col()
        .padding(Padding::new(3, 3, 2, 2))
        .gap(1)
        .child(label(title).fg(Color::Cyan).bold())
        .child(spacer().height(1))
        .child(label(instructions).fg(Color::Green))
        .child(spacer().height(1))
        .child(label("Quit key configuration:").fg(Color::Indexed(8)))
        .child(label(config).fg(Color::Indexed(8)))
}

fn main() -> WidgetResult<()> {
    println!("Choose a quit mode:");
    println!("1. Default (Esc)");
    println!("2. Custom key (q)");
    println!("3. Custom key with modifiers (Ctrl+C)");
    println!("4. Disabled (handle with 'q' key via on_key)");
    println!();
    print!("Enter choice (1-4): ");
    use std::io::{self, Write};
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let choice = input.trim().parse::<u8>().unwrap_or(1);

    let mode = match choice {
        2 => QuitMode::CustomKey,
        3 => QuitMode::CustomKeyEvent,
        4 => QuitMode::Disabled,
        _ => QuitMode::Default,
    };

    let state = State { mode };

    match mode {
        QuitMode::Default => {
            // Default behavior - Esc key quits automatically
            let app = App::new(state);
            app.run_inline(update_noop, view_default)?;
        }
        QuitMode::CustomKey => {
            // Custom quit key - 'q' key without modifiers
            let app = App::new(state).with_quit_key(KeyCode::Char('q'));
            app.run_inline(update_noop, view_default)?;
        }
        QuitMode::CustomKeyEvent => {
            // Custom quit key with modifiers - Ctrl+C
            let app = App::new(state)
                .with_quit_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
            app.run_inline(update_noop, view_default)?;
        }
        QuitMode::Disabled => {
            // Disabled - handle quit yourself via on_key
            let app = App::new(state)
                .disable_quit_key()
                .on_key(KeyCode::Char('q'), || Message::Quit);
            app.run_inline(update_with_quit, view_with_quit)?;
        }
    }

    Ok(())
}
