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
    panel()
        .title("Canvas")
        .padding(Padding::uniform(1))
        .gap(1)
        .child(
            canvas::<(), _>(|surface, ctx| {
                let width = ctx.dot_width().max(1);
                let height = ctx.dot_height().max(1);
                let mid = height as f64 / 2.0;

                for x in 0..width {
                    let y = mid + (x as f64 * 0.18).sin() * mid * 0.6;
                    surface.set(x as f64, y);
                }
            })
            .fixed(48, 12),
        )
}
