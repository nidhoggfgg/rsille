//! Divider widget.
//!
//! Run with: `cargo run -p tui --example divider`

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
    panel::<()>()
        .title("Divider")
        .padding(Padding::uniform(1))
        .gap(1)
        .child(divider().text("solid"))
        .child(divider().variant(DividerVariant::Dashed).text("dashed"))
        .child(divider().variant(DividerVariant::Dotted).text("dotted"))
        .child(divider().variant(DividerVariant::Heavy).text("heavy"))
        .child(divider().variant(DividerVariant::Double).text("double"))
        .child(
            row::<()>()
                .gap(1)
                .child(label("left"))
                .child(
                    divider()
                        .vertical()
                        .height(5)
                        .variant(DividerVariant::Double),
                )
                .child(label("right")),
        )
}
