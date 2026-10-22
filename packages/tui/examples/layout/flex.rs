//! Flex layout with row and column composition.
//!
//! Run with: `cargo run -p tui --example flex`

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
    col::<()>()
        .padding(Padding::uniform(1))
        .gap(1)
        .child(label("Flex").bold())
        .child(divider())
        .child(
            row::<()>()
                .gap(2)
                .align_items(AlignItems::Start)
                .child(boxed("left", "natural width"))
                .child(boxed("center", "row gap: 2"))
                .child(boxed("right", "aligned start")),
        )
        .child(
            col::<()>()
                .border(BorderStyle::Rounded)
                .padding(Padding::uniform(1))
                .gap(1)
                .child(label("Nested column").fg(Color::Cyan))
                .child(label("Rows and columns are the same flex primitive."))
                .child(label("Use gap, padding, border, and alignment together.")),
        )
}

fn boxed(title: &'static str, body: &'static str) -> impl Widget<()> {
    panel::<()>()
        .title(title)
        .padding(Padding::uniform(1))
        .child(label(body))
}
