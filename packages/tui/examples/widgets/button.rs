//! Button widget variants and click messages.
//!
//! Run with: `cargo run -p tui --example button`
//! Use Tab to focus buttons, Enter/Space to activate. Esc to quit.

use tui::prelude::*;

#[derive(Debug, Clone)]
enum Msg {
    Increment,
    Decrement,
    Reset,
}

fn main() -> WidgetResult<()> {
    let state = 0i32;

    App::new(state)
        .mode(Mode::Inline)
        .update(|state, msg, _ctx| update(state, msg))
        .view(view)
        .run()
        .map(|_| ())
}

fn update(state: &mut i32, msg: Msg) {
    match msg {
        Msg::Increment => *state += 1,
        Msg::Decrement => *state -= 1,
        Msg::Reset => *state = 0,
    }
}

fn view(state: &i32) -> impl Widget<Msg> {
    panel::<Msg>()
        .title("Button")
        .padding(Padding::uniform(1))
        .gap(1)
        .child(label(format!("Count: {}", state)))
        .child(divider())
        .child(
            row::<Msg>()
                .gap(2)
                .child(
                    button("+1")
                        .variant(ButtonVariant::Primary)
                        .on_click(|| Msg::Increment),
                )
                .child(
                    button("-1")
                        .variant(ButtonVariant::Secondary)
                        .on_click(|| Msg::Decrement),
                )
                .child(
                    button("Reset")
                        .variant(ButtonVariant::Ghost)
                        .on_click(|| Msg::Reset),
                ),
        )
        .child(
            row::<Msg>()
                .gap(2)
                .child(button("Link").variant(ButtonVariant::Link))
                .child(button("Delete").variant(ButtonVariant::Destructive))
                .child(button("Disabled").disabled(true)),
        )
}
