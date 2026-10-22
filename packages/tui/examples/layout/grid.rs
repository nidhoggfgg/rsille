//! Grid — grid layout with columns and rows
//!
//! Run with: `cargo run -p tui --example grid`
//! Press Esc to quit.

use tui::prelude::*;

fn main() -> WidgetResult<()> {
    let state = ();

    App::new(state)
        .update(|state, msg, _ctx| update(state, msg))
        .view(view)
        .run()
        .map(|_| ())
}

fn update(_state: &mut (), _msg: ()) {}

fn view(_state: &()) -> impl Widget<()> {
    grid::<()>()
        .columns("1fr 1fr 1fr")
        .rows("auto auto auto")
        .gap(1)
        .padding(Padding::uniform(2))
        .border(BorderStyle::Single)
        .child_at(label("A1"), GridPlacement::new().area(1, 1))
        .child_at(label("B1"), GridPlacement::new().area(2, 1))
        .child_at(label("C1"), GridPlacement::new().area(3, 1))
        .child_at(label("A2"), GridPlacement::new().area(1, 2))
        .child_at(
            label("B2-C2 (span)"),
            GridPlacement::new().area_span(2, 2, 2, 1),
        )
        .child_at(label("A3"), GridPlacement::new().area(1, 3))
        .child_at(label("B3"), GridPlacement::new().area(2, 3))
        .child_at(label("C3"), GridPlacement::new().area(3, 3))
}
