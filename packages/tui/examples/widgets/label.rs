//! Label widget.
//!
//! Run with: `cargo run -p tui --example label`

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
        .title("Label")
        .padding(Padding::uniform(1))
        .gap(1)
        .child(label("plain label"))
        .child(label("bold cyan").fg(Color::Cyan).bold())
        .child(label("underlined yellow").fg(Color::Yellow).underline())
        .child(
            panel::<()>()
                .border(BorderStyle::Single)
                .padding(Padding::uniform(1))
                .child(
                    label("Word wrapping keeps complete words together inside a fixed width.")
                        .width(28)
                        .wrap(TextWrap::Word),
                ),
        )
        .child(
            panel::<()>().border(BorderStyle::Rounded).child(
                label("centered")
                    .fixed(28, 3)
                    .align(HorizontalAlign::Center)
                    .valign(VerticalAlign::Middle),
            ),
        )
}
