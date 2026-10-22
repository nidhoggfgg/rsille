//! Hello World — minimal TUI example
//!
//! Run with: `cargo run -p tui --example hello`
//! Press Esc to quit.

use tui::prelude::*;

fn main() -> WidgetResult<()> {
    let state = ();

    App::new(state)
        .mode(Mode::Inline)
        .update(|state, msg, _ctx| update(state, msg))
        .view(view)
        .run()
        .map(|_| ())
}

fn update(_state: &mut (), _msg: ()) {}

fn view(_state: &()) -> impl Widget<()> {
    col::<()>()
        .child(label("Hello, TUI!"))
        .child(spacer())
        .child(label("Press Esc to quit."))
}
