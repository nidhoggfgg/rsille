//! RadioGroup widget.
//!
//! Run with: `cargo run -p tui --example radio_group`

use tui::prelude::*;

#[derive(Debug, Clone)]
struct State {
    density: String,
}

fn main() -> WidgetResult<()> {
    App::new(State {
        density: "comfortable".to_owned(),
    })
    .mode(Mode::Inline)
    .update(|state, value, _ctx| state.density = value)
    .view(view)
    .run()
    .map(|_| ())
}

fn view(state: &State) -> impl Widget<String> {
    panel::<String>()
        .title("RadioGroup")
        .padding(Padding::uniform(1))
        .gap(1)
        .child(
            radio_group()
                .key("density")
                .selected(state.density.clone())
                .options([
                    RadioOption::new("compact", "Compact"),
                    RadioOption::new("comfortable", "Comfortable"),
                    RadioOption::new("spacious", "Spacious"),
                    RadioOption::new("disabled", "Disabled").disabled(true),
                ])
                .on_change(|value| value),
        )
        .child(divider())
        .child(label(format!("Selected: {}", state.density)))
}
