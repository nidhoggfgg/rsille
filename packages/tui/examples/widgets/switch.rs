//! Switch and toggle widgets.
//!
//! Run with: `cargo run -p tui --example switch`

use tui::prelude::*;

#[derive(Debug, Clone)]
enum Msg {
    Enabled(bool),
    Compact(bool),
}

#[derive(Debug)]
struct State {
    enabled: bool,
    compact: bool,
}

fn main() -> WidgetResult<()> {
    App::new(State {
        enabled: true,
        compact: false,
    })
    .mode(Mode::Inline)
    .update(|state, msg, _ctx| update(state, msg))
    .view(view)
    .run()
    .map(|_| ())
}

fn update(state: &mut State, msg: Msg) {
    match msg {
        Msg::Enabled(value) => state.enabled = value,
        Msg::Compact(value) => state.compact = value,
    }
}

fn view(state: &State) -> impl Widget<Msg> {
    panel::<Msg>()
        .title("Switch")
        .padding(Padding::uniform(1))
        .gap(1)
        .child(
            switch("Notifications")
                .checked(state.enabled)
                .on_change(Msg::Enabled),
        )
        .child(
            toggle("Compact mode")
                .checked(state.compact)
                .on_change(Msg::Compact),
        )
        .child(switch("Disabled").checked(true).disabled(true))
        .child(divider())
        .child(label(format!(
            "notifications={} compact={}",
            state.enabled, state.compact
        )))
}
