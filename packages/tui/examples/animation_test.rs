//! Simple Animation Test
//!
//! A minimal test to verify animations are working
//!
//! Run with: cargo run --example animation_test

use tui::prelude::*;
use std::time::Duration;

#[derive(Debug)]
struct State {
    counter: u32,
    fade: OpacityTransition,
}

#[derive(Clone, Debug)]
enum Message {
    Increment,
}

fn update(state: &mut State, msg: Message) {
    match msg {
        Message::Increment => {
            state.counter += 1;
        }
    }
}

fn view(state: &State) -> Container<Message> {
    let opacity = state.fade.opacity();

    Container::vertical(vec![
        Label::new(format!("Counter: {}", state.counter))
            .style(Style::default().fg(Color::Green).bold())
            .into(),
        Label::new(format!("Opacity: {:.2} (should change over time)", opacity))
            .style(Style::default().fg(Color::Cyan))
            .into(),
        Label::new(format!("Alpha: {} / 255", state.fade.alpha_u8()))
            .style(Style::default().fg(Color::Yellow))
            .into(),
        Label::new("")
            .into(),
        KeyboardController::new()
            .on_key(KeyCode::Char('i'), || Message::Increment)
            .into(),
        Label::new("Press 'i' to increment counter, Esc to quit")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
    ])
    .gap(0)
    .padding(Padding::new(2, 2, 1, 1))
}

fn main() -> Result<()> {
    let mut fade = OpacityTransition::new();
    // Start a fade out animation
    fade.fade_out(Duration::from_secs(3));

    let app = App::new(State {
        counter: 0,
        fade,
    });

    app.run(update, view)?;
    Ok(())
}
