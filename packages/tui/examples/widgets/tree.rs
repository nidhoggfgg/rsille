//! Tree widget with expandable nodes and keyboard navigation.
//!
//! Run with: `cargo run -p tui --example tree`
//! Use Left/Right to collapse or expand, Up/Down to move, and Enter to open leaf nodes.

use tui::prelude::*;

#[derive(Debug, Clone)]
enum Msg {
    Focused(String),
    Opened(String),
    Selected(Vec<String>),
}

#[derive(Debug, Default)]
struct State {
    active: String,
    opened: String,
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
        Msg::Focused(id) => state.active = id,
        Msg::Opened(id) => state.opened = id,
        Msg::Selected(ids) => state.selected = ids,
    }
}

fn view(state: &State) -> impl Widget<Msg> {
    row::<Msg>()
        .padding(Padding::uniform(1))
        .gap(2)
        .child(
            tree::<Msg>()
                .key("workspace")
                .height(16)
                .multi_select(true)
                .items([
                    TreeItem::new("src", "src")
                        .child(TreeItem::new("src/tui", "tui"))
                        .child(TreeItem::new("src/render", "render"))
                        .child(TreeItem::new("src/canvas", "canvas")),
                    TreeItem::new("packages", "packages")
                        .child(
                            TreeItem::new("packages/tui", "tui")
                                .child(
                                    TreeItem::new("packages/tui/src", "src")
                                        .child(TreeItem::new("packages/tui/src/lib.rs", "lib.rs")),
                                )
                                .child(
                                    TreeItem::new("packages/tui/examples", "examples")
                                        .child(
                                            TreeItem::new("packages/tui/examples/basics", "basics")
                                                .child(TreeItem::new(
                                                    "packages/tui/examples/basics/hello.rs",
                                                    "hello.rs",
                                                )),
                                        )
                                        .child(
                                            TreeItem::new("packages/tui/examples/layout", "layout")
                                                .child(TreeItem::new(
                                                    "packages/tui/examples/layout/grid.rs",
                                                    "grid.rs",
                                                ))
                                                .child(TreeItem::new(
                                                    "packages/tui/examples/layout/layout.rs",
                                                    "layout.rs",
                                                )),
                                        )
                                        .child(
                                            TreeItem::new(
                                                "packages/tui/examples/widgets",
                                                "widgets",
                                            )
                                            .child(TreeItem::new(
                                                "packages/tui/examples/widgets/button.rs",
                                                "button.rs",
                                            ))
                                            .child(TreeItem::new(
                                                "packages/tui/examples/widgets/tree.rs",
                                                "tree.rs",
                                            ))
                                            .child(
                                                TreeItem::new(
                                                    "packages/tui/examples/widgets/more",
                                                    "... 5 more",
                                                ),
                                            ),
                                        )
                                        .child(
                                            TreeItem::new("packages/tui/examples/scenes", "scenes")
                                                .child(TreeItem::new(
                                                    "packages/tui/examples/scenes/flame.rs",
                                                    "flame.rs",
                                                ))
                                                .child(TreeItem::new(
                                                    "packages/tui/examples/scenes/wave.rs",
                                                    "wave.rs",
                                                ))
                                                .child(TreeItem::new(
                                                    "packages/tui/examples/scenes/more",
                                                    "... 3 more",
                                                )),
                                        ),
                                ),
                        )
                        .child(TreeItem::new("packages/render", "render"))
                        .child(TreeItem::new("packages/canvas", "canvas")),
                    TreeItem::new("docs", "docs")
                        .child(TreeItem::new("docs/roadmap.md", "roadmap.md"))
                        .child(TreeItem::new("docs/theme.md", "theme.md")),
                    TreeItem::new("scripts", "scripts")
                        .child(TreeItem::new("scripts/release.sh", "release.sh")),
                ])
                .on_change(Msg::Focused)
                .on_selection_change(Msg::Selected)
                .on_submit(Msg::Opened),
        )
        .child(
            col::<Msg>()
                .border(BorderStyle::Rounded)
                .padding(Padding::uniform(1))
                .gap(1)
                .child(label("Tree Example").bold())
                .child(label("Right expands, Left collapses."))
                .child(label("Enter opens the current leaf node."))
                .child(divider().text("Focus"))
                .child(label(format!(
                    "Active node: {}",
                    if state.active.is_empty() {
                        "none"
                    } else {
                        &state.active
                    }
                )))
                .child(label(format!(
                    "Opened leaf: {}",
                    if state.opened.is_empty() {
                        "none"
                    } else {
                        &state.opened
                    }
                )))
                .child(label(format!("Selected nodes: {}", state.selected.len())))
                .child(
                    button("Open current leaf")
                        .variant(ButtonVariant::Secondary)
                        .on_click(|| Msg::Opened("triggered-by-button".to_owned())),
                ),
        )
}
