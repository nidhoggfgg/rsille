//! TextArea widget.
//!
//! Run with: `cargo run -p tui --example textarea`

use tui::prelude::*;

#[derive(Debug, Clone)]
enum Msg {
    Change(String),
    Submit(String),
}

#[derive(Debug)]
struct State {
    value: String,
    submitted: String,
}

fn main() -> WidgetResult<()> {
    App::new(State {
        value: "Draft notes\nLine two".to_owned(),
        submitted: "none".to_owned(),
    })
    .mode(Mode::Inline)
    .update(|state, msg, _ctx| update(state, msg))
    .view(view)
    .run()
    .map(|_| ())
}

fn update(state: &mut State, msg: Msg) {
    match msg {
        Msg::Change(value) => state.value = value,
        Msg::Submit(value) => state.submitted = value,
    }
}

fn view(state: &State) -> impl Widget<Msg> {
    panel::<Msg>()
        .title("TextArea")
        .padding(Padding::uniform(1))
        .gap(1)
        .child(
            textarea::<Msg>()
                .key("notes")
                .value(state.value.clone())
                .placeholder("Write notes")
                .height(6)
                .on_change(Msg::Change)
                .on_submit(Msg::Submit),
        )
        .child(
            textarea::<Msg>()
                .value("Borderless textarea")
                .variant(TextAreaVariant::Borderless)
                .height(3),
        )
        .child(label(format!("Submitted: {}", state.submitted)))
}
