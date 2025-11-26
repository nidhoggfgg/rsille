use tui::prelude::*;

#[derive(Debug)]
struct State {}

#[derive(Clone, Debug)]
enum Message {}

fn update(_state: &mut State, _msg: Message) {}

fn view(_state: &State) -> impl Layout<Message> {
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
