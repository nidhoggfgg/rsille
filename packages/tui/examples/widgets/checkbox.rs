//! Checkbox widget.
//!
//! Run with: `cargo run -p tui --example checkbox`

use tui::prelude::*;

#[derive(Debug, Clone)]
enum Msg {
    Remember(bool),
    Subscribe(bool),
}

#[derive(Debug)]
struct State {
    remember: bool,
    subscribe: bool,
}

fn main() -> WidgetResult<()> {
    App::new(State {
        remember: true,
        subscribe: false,
    })
    .mode(Mode::Inline)
    .update(|state, msg, _ctx| update(state, msg))
    .view(view)
    .run()
    .map(|_| ())
}

fn update(state: &mut State, msg: Msg) {
    match msg {
        Msg::Remember(value) => state.remember = value,
        Msg::Subscribe(value) => state.subscribe = value,
    }
}

fn view(state: &State) -> impl Widget<Msg> {
    panel::<Msg>()
        .title("Checkbox")
        .padding(Padding::uniform(1))
        .gap(1)
        .child(
            checkbox("Remember workspace")
                .checked(state.remember)
                .on_change(Msg::Remember),
        )
        .child(
            checkbox("Subscribe to updates")
                .checked(state.subscribe)
                .on_change(Msg::Subscribe),
        )
        .child(checkbox("Disabled option").checked(true).disabled(true))
        .child(divider())
        .child(label(format!(
            "remember={} subscribe={}",
            state.remember, state.subscribe
        )))
}
