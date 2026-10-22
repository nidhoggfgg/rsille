//! Split panes.
//!
//! Run with: `cargo run -p tui --example split`
//! Focus the divider and use arrow keys to resize.

use tui::prelude::*;

fn main() -> WidgetResult<()> {
    App::new(())
        .mode(Mode::Inline)
        .update(|_, _, _ctx| {})
        .view(view)
        .run()
        .map(|_| ())
}

fn view(_: &()) -> impl Widget<()> {
    split(
        panel::<()>()
            .title("Sidebar")
            .padding(Padding::uniform(1))
            .gap(1)
            .child(label("fixed: 24 columns"))
            .child(label("min: 16")),
        panel::<()>()
            .title("Content")
            .padding(Padding::uniform(1))
            .gap(1)
            .child(label("resizable split pane").bold())
            .child(label("The divider stores its size in widget state.")),
    )
    .sidebar(24)
    .min_first(16)
    .min_second(24)
    .divider_size(1)
    .resizable(true)
    .key("example-split")
}
