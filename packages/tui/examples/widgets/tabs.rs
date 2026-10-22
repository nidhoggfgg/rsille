//! Tabs widget.
//!
//! Run with: `cargo run -p tui --example tabs`

use tui::prelude::*;

#[derive(Debug, Clone)]
struct State {
    tab: String,
}

fn main() -> WidgetResult<()> {
    App::new(State {
        tab: "overview".to_owned(),
    })
    .mode(Mode::Inline)
    .update(|state, tab, _ctx| state.tab = tab)
    .view(view)
    .run()
    .map(|_| ())
}

fn view(state: &State) -> impl Widget<String> {
    panel::<String>()
        .title("Tabs")
        .padding(Padding::uniform(1))
        .gap(1)
        .child(
            tabs()
                .key("tabs")
                .selected(state.tab.clone())
                .tabs([
                    TabItem::new("overview", "Overview"),
                    TabItem::new("activity", "Activity"),
                    TabItem::new("billing", "Billing").disabled(true),
                    TabItem::new("settings", "Settings"),
                ])
                .on_change(|tab| tab),
        )
        .child(label(format!("Active tab: {}", state.tab)))
}
