//! Shared selection behavior for collection widgets.
//!
//! Run with: `cargo run -p tui --example selection`

use tui::prelude::*;

#[derive(Debug, Clone)]
enum Msg {
    Highlighted(String),
    Selected(Vec<String>),
}

#[derive(Debug, Default)]
struct State {
    active: String,
    selected: Vec<String>,
}

fn main() -> WidgetResult<()> {
    App::new(State::default())
        .mode(Mode::Inline)
        .update(|state, msg, _ctx| update(state, msg))
        .view(view)
        .run()
        .map(|_| ())
}

fn update(state: &mut State, msg: Msg) {
    match msg {
        Msg::Highlighted(id) => state.active = id,
        Msg::Selected(ids) => state.selected = ids,
    }
}

fn view(state: &State) -> impl Widget<Msg> {
    panel::<Msg>()
        .title("Selection")
        .padding(Padding::uniform(1))
        .gap(1)
        .child(
            list::<Msg>()
                .key("selection-list")
                .height(7)
                .selection_mode(SelectionMode::Multiple)
                .items([
                    ListItem::new("alpha", "Alpha"),
                    ListItem::new("beta", "Beta"),
                    ListItem::new("gamma", "Gamma"),
                    ListItem::new("delta", "Delta"),
                ])
                .on_change(Msg::Highlighted)
                .on_selection_change(Msg::Selected),
        )
        .child(label(format!(
            "active={} selected={}",
            empty(&state.active),
            if state.selected.is_empty() {
                "none".to_owned()
            } else {
                state.selected.join(", ")
            }
        )))
}

fn empty(value: &str) -> &str {
    if value.is_empty() {
        "none"
    } else {
        value
    }
}
