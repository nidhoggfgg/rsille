//! Select widget with an inline option menu.
//!
//! Run with: `cargo run -p tui --example select`
//! Use Enter or Space to open the menu, type to filter, and Enter to confirm.

use tui::prelude::*;

#[derive(Debug, Clone)]
enum Msg {
    Changed(String),
}

#[derive(Debug, Default)]
struct State {
    selected: String,
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
        Msg::Changed(value) => state.selected = value,
    }
}

fn view(state: &State) -> impl Widget<Msg> {
    row::<Msg>()
        .padding(Padding::uniform(1))
        .gap(2)
        .child(
            select::<Msg>()
                .key("priority")
                .height(8)
                .placeholder("Pick a deployment priority")
                .searchable(true)
                .search_mode(SelectSearchMode::Fuzzy)
                .options([
                    SelectOption::new("low", "Low priority"),
                    SelectOption::new("normal", "Normal priority"),
                    SelectOption::new("high", "High priority"),
                    SelectOption::new("critical", "Critical response"),
                ])
                .on_change(Msg::Changed),
        )
        .child(
            col::<Msg>()
                .border(BorderStyle::Rounded)
                .padding(Padding::uniform(1))
                .gap(1)
                .child(label("Select Example").bold())
                .child(label("Open the menu with Enter or Space."))
                .child(label("Once open, typing filters the options inline."))
                .child(label(
                    "Selection is stored in widget state and emitted on confirm.",
                ))
                .child(divider().text("Value"))
                .child(label(format!(
                    "Current value: {}",
                    if state.selected.is_empty() {
                        "none"
                    } else {
                        &state.selected
                    }
                ))),
        )
}
