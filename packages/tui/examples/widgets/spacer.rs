//! Spacer widget.
//!
//! Run with: `cargo run -p tui --example spacer`

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
        .title("Spacer")
        .padding(Padding::uniform(1))
        .gap(1)
        .child(
            row::<()>()
                .border(BorderStyle::Single)
                .child(label("left"))
                .child(spacer().width(8))
                .child(label("right")),
        )
        .child(
            row::<()>()
                .border(BorderStyle::Single)
                .child(label("start"))
                .child(spacer().flex(1.0))
                .child(label("end")),
        )
        .child(spacer().height(2))
        .child(label("The blank area above is a fixed-height spacer."))
}
