//! Calendar widget for date navigation and selection.
//!
//! Run with: `cargo run -p tui --example calendar`
//! Use arrows to move by day or week, PageUp/PageDown to change month, and Enter to submit.

use tui::prelude::*;

#[derive(Debug, Clone)]
enum Msg {
    Changed(CalendarDate),
    Submitted(CalendarDate),
}

#[derive(Debug)]
struct State {
    active: CalendarDate,
    submitted: Option<CalendarDate>,
}

fn main() -> WidgetResult<()> {
    let initial = CalendarDate::new(2026, 3, 20).expect("valid sample date");
    App::new(State {
        active: initial,
        submitted: None,
    })
    .mode(Mode::Inline)
    .update(|state, msg, _ctx| update(state, msg))
    .view(view)
    .run()
    .map(|_| ())
}

fn update(state: &mut State, msg: Msg) {
    match msg {
        Msg::Changed(date) => state.active = date,
        Msg::Submitted(date) => state.submitted = Some(date),
    }
}

fn view(state: &State) -> impl Widget<Msg> {
    row::<Msg>()
        .padding(Padding::uniform(1))
        .gap(2)
        .child(
            calendar::<Msg>(state.active)
                .key("date-picker")
                .on_change(Msg::Changed)
                .on_submit(Msg::Submitted),
        )
        .child(
            col::<Msg>()
                .border(BorderStyle::Rounded)
                .padding(Padding::uniform(1))
                .gap(1)
                .child(label("Calendar Example").bold())
                .child(label("Arrow keys move the selected date."))
                .child(label("PageUp/PageDown jumps month by month."))
                .child(divider().text("Selection"))
                .child(label(format!(
                    "Active date: {:04}-{:02}-{:02}",
                    state.active.year, state.active.month, state.active.day
                )))
                .child(label(match state.submitted {
                    Some(date) => format!(
                        "Submitted: {:04}-{:02}-{:02}",
                        date.year, date.month, date.day
                    ),
                    None => "Submitted: none".to_owned(),
                })),
        )
}
