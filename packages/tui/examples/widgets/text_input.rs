//! TextInput — stateful text input with on_change
//!
//! Run with: `cargo run -p tui --example text_input`
//! Use Tab to focus the input. Type to edit. Enter to submit. Esc to quit.

use tui::prelude::*;

#[derive(Debug, Clone)]
enum Msg {
    NameChanged(String),
    Submitted(String),
}

fn main() -> WidgetResult<()> {
    let state = String::new();

    App::new(state)
        .mode(Mode::Inline)
        .update(|state, msg, _ctx| update(state, msg))
        .view(view)
        .run()
        .map(|_| ())
}

fn update(state: &mut String, msg: Msg) {
    match msg {
        Msg::NameChanged(s) => *state = s,
        Msg::Submitted(s) => *state = s, // Could show a confirmation, etc.
    }
}

fn view(state: &String) -> impl Widget<Msg> {
    col::<Msg>()
        .padding(Padding::uniform(2))
        .gap(1)
        .child(label("Enter your name:"))
        .child(
            text_input::<Msg>()
                .value(state.as_str())
                .placeholder("Type here...")
                .on_change(Msg::NameChanged)
                .on_submit(Msg::Submitted),
        )
        .child(label(format!(
            "Hello, {}!",
            if state.is_empty() { "..." } else { state }
        )))
}
