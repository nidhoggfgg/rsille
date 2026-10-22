//! Dialog widget.
//!
//! Run with: `cargo run -p tui --example dialog`

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
    overlay(label("Dialog is centered by an overlay layer."))
        .layer(
            OverlayLayer::new(
                dialog::<()>()
                    .title("Dialog")
                    .padding(Padding::uniform(1))
                    .gap(1)
                    .child(label("Dialogs are panels intended for modal surfaces."))
                    .child(
                        row::<()>()
                            .gap(2)
                            .child(button("Cancel"))
                            .child(button("Apply")),
                    ),
            )
            .floating(OverlayAnchor::Center)
            .size(48, 9),
        )
        .trap_focus()
}
