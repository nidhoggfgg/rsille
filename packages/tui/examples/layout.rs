use tui::prelude::*;

#[derive(Debug)]
struct State {}

fn update(_state: &mut State, _msg: ()) {}

fn view(_state: &State) -> impl Layout<()> {
    col()
        .gap(1)
        .child(
            label("Layout Examples (Press Esc to exit)")
                .bold()
                .fg(Color::Cyan),
        )
        .child(divider().horizontal().variant(DividerVariant::Heavy))
        .child(
            label("Layout Examples (Press Esc to exit)")
                .bold()
                .fg(Color::Cyan),
        )
}

fn main() {
    let app = App::new(State {});
    app.run_inline(update, view).unwrap();
}
