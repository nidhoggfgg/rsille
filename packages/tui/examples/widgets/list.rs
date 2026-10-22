//! List widget with internal arrow-key navigation.
//!
//! Run with: `cargo run -p tui --example list`
//! Use Tab to focus the list and button. Use arrows/Home/End/PageUp/PageDown in the list.

use tui::prelude::*;

#[derive(Debug, Clone)]
enum Msg {
    Highlighted(String),
    Submitted(String),
    Selected(Vec<String>),
    Confirm,
}

#[derive(Debug)]
struct State {
    active: String,
    submitted: String,
    selected: Vec<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            active: "focus".to_owned(),
            submitted: String::new(),
            selected: Vec::new(),
        }
    }
}

fn main() -> WidgetResult<()> {
    App::new(State::default())
        .mode(Mode::Inline)
        .update(|state, msg, _ctx| update(state, msg))
        .view(view)
        .run()
        .map(|_| ())
}

fn update(state: &mut State, msg: Msg) {
    match msg {
        Msg::Highlighted(id) => state.active = id,
        Msg::Submitted(id) => state.submitted = id,
        Msg::Selected(ids) => state.selected = ids,
        Msg::Confirm => state.submitted = state.active.clone(),
    }
}

fn view(state: &State) -> impl Widget<Msg> {
    col::<Msg>()
        .padding(Padding::uniform(1))
        .gap(1)
        .child(label("Pick a framework milestone:"))
        .child(label(
            "The list is focused initially. Use Up/Down right away, Tab moves to the button.",
        ))
        .child(
            list::<Msg>()
                .key("milestones")
                .height(8)
                .multi_select(true)
                .items([
                    ListItem::new("focus", "Scope-aware focus manager"),
                    ListItem::new("routing", "Capture/target/bubble event routing"),
                    ListItem::new("list", "Composite list widget"),
                    ListItem::new("modal", "Focus-trapped modal support"),
                    ListItem::new("tests", "Regression coverage"),
                ])
                .on_change(Msg::Highlighted)
                .on_selection_change(Msg::Selected)
                .on_submit(Msg::Submitted),
        )
        .child(
            button("Confirm active item")
                .variant(ButtonVariant::Secondary)
                .on_click(|| Msg::Confirm),
        )
        .child(label(format!(
            "Active: {}",
            if state.active.is_empty() {
                "none"
            } else {
                &state.active
            }
        )))
        .child(label(format!(
            "Submitted: {}",
            if state.submitted.is_empty() {
                "none"
            } else {
                &state.submitted
            }
        )))
        .child(label(format!("Selected count: {}", state.selected.len())))
}
