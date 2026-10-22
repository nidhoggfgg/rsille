//! Stack layers in the same layout area.
//!
//! Run with: `cargo run -p tui --example stack`

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
    stack::<()>()
        .child(
            panel::<()>()
                .title("Layer 1")
                .padding(Padding::uniform(1))
                .style(Style::default().fg(Color::Indexed(8)))
                .child(label("background layer")),
        )
        .child(
            col::<()>().padding(Padding::new(3, 4, 1, 8)).child(
                panel::<()>()
                    .title("Layer 2")
                    .padding(Padding::uniform(1))
                    .child(label("drawn over the same area")),
            ),
        )
}
