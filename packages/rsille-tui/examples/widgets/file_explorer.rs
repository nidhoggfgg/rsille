//! FileExplorer widget with expandable and lazy directories.
//!
//! Run with: `cargo run -p rsille-tui --example file_explorer`

use rsille_tui::prelude::*;

#[derive(Debug, Clone)]
enum Msg {
    Focused(String),
    Opened(String),
    Load(String),
    Selected(Vec<String>),
}

#[derive(Debug, Default)]
struct State {
    focused: String,
    opened: String,
    load_request: String,
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
        Msg::Focused(id) => state.focused = id,
        Msg::Opened(id) => state.opened = id,
        Msg::Load(id) => state.load_request = id,
        Msg::Selected(ids) => state.selected = ids,
    }
}

fn view(state: &State) -> impl Widget<Msg> {
    row::<Msg>()
        .padding(Padding::uniform(1))
        .gap(2)
        .child(
            file_explorer::<Msg>()
                .key("workspace-files")
                .height(18)
                .multi_select(true)
                .items([
                    FileExplorerItem::directory("src", "src")
                        .child(FileExplorerItem::file("src/lib.rs", "lib.rs")),
                    FileExplorerItem::directory("packages", "packages")
                        .child(
                            FileExplorerItem::directory("packages/rsille-tui", "rsille-tui")
                                .child(FileExplorerItem::lazy_directory(
                                    "packages/rsille-tui/src",
                                    "src",
                                ))
                                .child(FileExplorerItem::file(
                                    "packages/rsille-tui/Cargo.toml",
                                    "Cargo.toml",
                                ))
                                .child(
                                    FileExplorerItem::directory(
                                        "packages/rsille-tui/examples",
                                        "examples",
                                    )
                                    .child(FileExplorerItem::file(
                                        "packages/rsille-tui/examples/widgets/data_table.rs",
                                        "data_table.rs",
                                    ))
                                    .child(
                                        FileExplorerItem::file(
                                            "packages/rsille-tui/examples/widgets/file_explorer.rs",
                                            "file_explorer.rs",
                                        ),
                                    ),
                                ),
                        )
                        .child(FileExplorerItem::directory(
                            "packages/rsille-render",
                            "rsille-render",
                        ))
                        .child(FileExplorerItem::directory(
                            "packages/rsille-canvas",
                            "rsille-canvas",
                        )),
                    FileExplorerItem::file("Cargo.toml", "Cargo.toml"),
                ])
                .on_change(Msg::Focused)
                .on_open(Msg::Opened)
                .on_load_children(Msg::Load)
                .on_selection_change(Msg::Selected),
        )
        .child(
            col::<Msg>()
                .border(BorderStyle::Rounded)
                .padding(Padding::uniform(1))
                .gap(1)
                .child(label("FileExplorer Example").bold())
                .child(label(format!(
                    "Focused: {}",
                    if state.focused.is_empty() {
                        "none"
                    } else {
                        &state.focused
                    }
                )))
                .child(label(format!(
                    "Opened: {}",
                    if state.opened.is_empty() {
                        "none"
                    } else {
                        &state.opened
                    }
                )))
                .child(label(format!(
                    "Lazy load: {}",
                    if state.load_request.is_empty() {
                        "none"
                    } else {
                        &state.load_request
                    }
                )))
                .child(label(format!("Selected: {}", state.selected.len()))),
        )
}
