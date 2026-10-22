//! Run without a TTY: cargo run -p tui --example headless
use std::time::Duration;
use tui::prelude::*;

#[derive(Clone, Debug)]
enum Msg {
    Edit(String),
    Save,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let clock = ManualClock::default();
    let mut app = App::new((String::new(), false))
        .with_clock(clock.clone())
        .build_with_effects(
            |state, message, ctx| match message {
                Msg::Edit(text) => {
                    state.0 = text;
                    ctx.debounce("save", Duration::from_millis(300), Msg::Save);
                }
                Msg::Save => state.1 = true,
            },
            |state| {
                text_input()
                    .key("editor")
                    .value(&state.0)
                    .on_change(Msg::Edit)
            },
        )
        .size(40, 3);

    app.step(&[Event::Key(KeyEvent::new(
        KeyCode::Char('a'),
        KeyModifiers::empty(),
    ))])?;
    assert_eq!(app.state().0, "a");
    assert_eq!(app.focused_key(), Some("editor"));
    assert!(!app.state().1);

    clock.advance(Duration::from_millis(300));
    app.step(&[])?;
    assert!(app.state().1);
    assert_eq!(app.next_update_at(), None);
    println!("Input, update, drawing and debounce completed without a terminal.");
    Ok(())
}
