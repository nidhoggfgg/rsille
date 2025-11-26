//! Simple test to verify mouse events are working
//!
//! Note: Mouse events only work in fullscreen mode (run()), not inline mode (run_inline()).
//! This is by design because inline mode allows terminal scrolling which would break
//! coordinate tracking.
//!
//! Run with: cargo run --example mouse_test

use tui::prelude::*;

#[derive(Debug)]
struct State {
    click_count: usize,
    last_event: String,
}

#[derive(Clone, Debug)]
enum Message {
    Clicked,
    Quit,
}

fn update(state: &mut State, msg: Message) {
    match msg {
        Message::Clicked => {
            state.click_count += 1;
            state.last_event = format!("Clicked {} times", state.click_count);
        }
        Message::Quit => {
            std::process::exit(0);
        }
    }
}

fn view(state: &State) -> impl Layout<Message> {
    col()
        .padding(Padding::new(2, 2, 1, 1))
        .gap(1)
        .child(
            label("Mouse Event Test (Fullscreen Mode)")
                .fg(Color::Cyan)
                .bold(),
        )
        .child(spacer().height(1))
        .child(label(&state.last_event).fg(Color::Green))
        .child(spacer().height(1))
        .child(
            interactive(
                label(">>> CLICK ME <<<")
                    .fg(Color::Yellow)
                    .style(Style::default().bg(Color::Blue)),
            )
            .on_click(|| Message::Clicked),
        )
        .child(spacer().height(1))
        .child(label("Press 'q' or Esc to quit").fg(Color::Indexed(8)))
        .child(keyboard_controller().on('q', || Message::Quit))
}

fn main() -> Result<()> {
    let app = App::new(State {
        click_count: 0,
        last_event: "No clicks yet - try clicking the button!".to_string(),
    });
    // Use run() for fullscreen mode with mouse support
    // (run_inline() does NOT support mouse events)
    app.run(update, view)?;
    Ok(())
}
