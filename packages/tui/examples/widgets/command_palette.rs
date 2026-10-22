//! CommandPalette widget.
//!
//! Run with: `cargo run -p tui --example command_palette`

use tui::prelude::*;

#[derive(Debug, Clone)]
enum Msg {
    Change(String),
    Submit(String),
    Close,
}

#[derive(Debug)]
struct State {
    active: String,
    submitted: String,
    closed: bool,
}

fn main() -> WidgetResult<()> {
    App::new(State {
        active: "none".to_owned(),
        submitted: "none".to_owned(),
        closed: false,
    })
    .mode(Mode::Inline)
    .update(|state, msg, _ctx| update(state, msg))
    .view(view)
    .run()
    .map(|_| ())
}

fn update(state: &mut State, msg: Msg) {
    match msg {
        Msg::Change(id) => state.active = id,
        Msg::Submit(id) => state.submitted = id,
        Msg::Close => state.closed = true,
    }
}

fn view(state: &State) -> impl Widget<Msg> {
    panel::<Msg>()
        .title("CommandPalette")
        .padding(Padding::uniform(1))
        .gap(1)
        .child(
            command_palette()
                .key("commands")
                .height(10)
                .title("Commands")
                .prompt(">")
                .placeholder("Search")
                .search_mode(SelectSearchMode::Fuzzy)
                .items([
                    CommandItem::new("open", "Open file").keywords(["file", "find"]),
                    CommandItem::new("rename", "Rename symbol").keyword("refactor"),
                    CommandItem::new("deploy", "Deploy").keyword("release"),
                    CommandItem::new("delete", "Delete").disabled(true),
                ])
                .on_change(Msg::Change)
                .on_submit(Msg::Submit)
                .on_close(|| Msg::Close),
        )
        .child(label(format!(
            "active={} submitted={} closed={}",
            state.active, state.submitted, state.closed
        )))
}
