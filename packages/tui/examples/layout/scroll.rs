//! Scroll views and scrollbars.
//!
//! Run with: `cargo run -p tui --example scroll`
//! Focus a viewport and use arrow keys or the mouse wheel.

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
    row::<()>()
        .padding(Padding::uniform(1))
        .gap(2)
        .child(
            scroll_lines((1..=40).map(|n| format!("vertical row {n:02}")))
                .key("vertical-scroll")
                .border(BorderStyle::Single)
                .padding(Padding::uniform(1))
                .scrollbars(ScrollbarVisibility::Always),
        )
        .child(
            scroll_view(
                label(
                    "This line is intentionally wide so horizontal scrolling has something to reveal. \
                     Keep moving right to inspect the rest of the content.",
                )
                .width(100),
            )
            .key("horizontal-scroll")
            .horizontal()
            .content_width(100)
            .border(BorderStyle::Rounded)
            .padding(Padding::uniform(1))
            .scrollbars(ScrollbarVisibility::Always),
        )
}
