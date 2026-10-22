//! Collapsible widget.
//!
//! Run with: `cargo run -p tui --example collapsible`

use tui::prelude::*;

#[derive(Debug, Clone)]
struct Msg(bool);

fn main() -> WidgetResult<()> {
    App::new(true)
        .mode(Mode::Inline)
        .update(|open, msg: Msg, _ctx| *open = msg.0)
        .view(view)
        .run()
        .map(|_| ())
}

fn view(open: &bool) -> impl Widget<Msg> {
    panel::<Msg>()
        .title("Collapsible")
        .padding(Padding::uniform(1))
        .gap(1)
        .child(
            collapsible("Details")
                .expanded(*open)
                .padding(Padding::new(1, 0, 0, 2))
                .gap(1)
                .on_toggle(Msg)
                .child(label("Hidden while collapsed."))
                .child(label("Focus skips children when closed.")),
        )
        .child(collapsible("Disabled").expanded(false).disabled(true))
}
