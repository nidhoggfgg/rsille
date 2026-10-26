//! Canvas widget.
//!
//! Run with: `cargo run -p tui --example canvas`

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
    col()
        .gap(1)
        .child(scene("Braille: curves and points", Encoding::Braille))
        .child(scene(
            "Half-block: solid color samples",
            Encoding::HalfBlock,
        ))
}

fn scene(title: &str, encoding: Encoding) -> impl Widget<()> {
    panel().title(title).padding(Padding::uniform(1)).child(
        canvas::<(), _>(|surface, ctx| {
            // Both encodings display the same world extent at their native
            // sample resolutions. Color is shared with the TUI style API.
            let mut world = ctx.world(surface).color(Color::Rgb(90, 210, 255));
            for step in 0..160 {
                let x = step as f64 / 10.0;
                world.point(CanvasPoint::new(x, x.sin()));
            }
            world.line(CanvasPoint::new(0.0, -1.3), CanvasPoint::new(16.0, -1.3));
        })
        .encoding(encoding)
        .viewport(CanvasViewport::new(0.0, -1.5, 16.0, 3.0))
        .fixed(48, 8),
    )
}
