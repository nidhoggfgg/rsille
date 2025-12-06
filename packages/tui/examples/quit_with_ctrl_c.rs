//! Quit with Ctrl+C Example
//!
//! Demonstrates using Ctrl+C as the quit key instead of Esc.
//! This is useful for applications where Esc is needed for other purposes.
//!
//! Controls:
//! - Ctrl+C: Quit
//!
//! Run with: cargo run --example quit_with_ctrl_c

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct State {
    counter: usize,
}

/// No-op update function
fn update(_state: &mut State, _msg: ()) {}

/// View function
fn view(state: &State) -> impl Layout<()> {
    col()
        .padding(Padding::new(3, 3, 2, 2))
        .gap(1)
        .child(
            label("Ctrl+C Quit Example")
                .fg(Color::Cyan)
                .bold(),
        )
        .child(spacer().height(1))
        .child(
            label("This app uses Ctrl+C to quit instead of Esc")
                .fg(Color::Green),
        )
        .child(
            label("This is useful when Esc is needed for other purposes")
                .fg(Color::Indexed(8)),
        )
        .child(spacer().height(1))
        .child(label(format!("Counter: {}", state.counter)).fg(Color::Yellow))
        .child(spacer().height(1))
        .child(label("Press Ctrl+C to quit").fg(Color::Red).bold())
}

fn main() -> WidgetResult<()> {
    let app = App::new(State { counter: 42 })
        .with_quit_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    app.run_inline(update, view)?;
    Ok(())
}
