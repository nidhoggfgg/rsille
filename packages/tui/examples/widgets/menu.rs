//! Menu widget.
//!
//! Run with: `cargo run -p tui --example menu`

use tui::prelude::*;

#[derive(Debug, Clone)]
struct State {
    selected: String,
}

fn main() -> WidgetResult<()> {
    App::new(State {
        selected: "none".to_owned(),
    })
    .mode(Mode::Inline)
    .update(|state, id, _ctx| state.selected = id)
    .view(view)
    .run()
    .map(|_| ())
}

fn view(state: &State) -> impl Widget<String> {
    panel::<String>()
        .title("Menu")
        .padding(Padding::uniform(1))
        .gap(1)
        .child(
            menu()
                .key("actions")
                .height(7)
                .items([
                    MenuItem::new("open", "Open"),
                    MenuItem::new("rename", "Rename"),
                    MenuItem::new("delete", "Delete").disabled(true),
                    MenuItem::new("archive", "Archive"),
                ])
                .on_select(|id| id),
        )
        .child(label(format!("Selected: {}", state.selected)))
}
