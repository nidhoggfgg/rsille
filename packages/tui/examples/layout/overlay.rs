//! Overlay layers for popovers and floating surfaces.
//!
//! Run with: `cargo run -p tui --example overlay`

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
    overlay(
        panel::<()>()
            .title("Base")
            .padding(Padding::uniform(1))
            .gap(1)
            .child(label("The base widget keeps its normal layout."))
            .child(label(
                "Overlay layers render above it with independent placement.",
            ))
            .child(divider())
            .child(label("Anchor rect: x=4 y=5 w=24 h=1")),
    )
    .layer(
        OverlayLayer::new(
            panel::<()>()
                .title("Floating")
                .padding(Padding::uniform(1))
                .child(label("center")),
        )
        .floating(OverlayAnchor::Center)
        .size(24, 5),
    )
    .layer(
        OverlayLayer::new(
            panel::<()>()
                .title("Anchored")
                .padding(Padding::uniform(1))
                .child(label("below target")),
        )
        .anchored(
            (4, 5, 24, 1),
            OverlayAnchor::BottomLeft,
            OverlayAnchor::TopLeft,
        )
        .offset(0, 1)
        .size(28, 5)
        .z_index(2),
    )
}
