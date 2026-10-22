//! Interactive scoped modules: cargo run -p tui --example composition
//! Tab/Shift+Tab navigate, Enter loads, F1/F2 mount or remove, F3 swaps, Esc exits.
use tui::prelude::*;

mod editor {
    use std::time::Duration;
    use tui::prelude::*;

    #[derive(Clone, Debug)]
    pub enum Msg {
        Edit(String),
        Saved(String),
        Load,
        Loaded(String),
        Status(TaskStatus),
        Observed,
    }

    #[derive(Default)]
    pub struct State {
        text: String,
        saved: String,
        loaded: Option<String>,
        task: Option<TaskStatus>,
        events: usize,
    }

    pub fn view(state: &State, title: &str) -> impl Widget<Msg> {
        let task = state.task.as_ref().map_or("idle", |task| match task.state {
            TaskState::Queued | TaskState::Running => "loading...",
            TaskState::Completed => "complete",
            _ => "stopped",
        });
        panel()
            .title(title)
            .child(
                text_input()
                    .key("input")
                    .value(&state.text)
                    .placeholder("Type, then press Enter")
                    .on_change(Msg::Edit)
                    .on_submit(|_| Msg::Load),
            )
            .child(label(format!("Saved: {}", state.saved)).width(28))
            .child(
                button("Load / restart (3s)")
                    .key("load")
                    .on_click(|| Msg::Load),
            )
            .child(label(format!("Task: {task}")).width(28))
            .child(
                label(format!(
                    "Loaded: {}",
                    state.loaded.as_deref().unwrap_or("-")
                ))
                .width(28),
            )
            .child(label(format!("Stream events: {}", state.events)).width(28))
    }

    pub fn update(state: &mut State, msg: Msg) -> Effect<Msg> {
        match msg {
            Msg::Edit(text) => {
                state.text = text.clone();
                Effect::debounce("save", Duration::from_millis(500), Msg::Saved(text))
            }
            Msg::Saved(text) => {
                state.saved = text;
                Effect::none()
            }
            Msg::Load => {
                let text = state.text.clone();
                Effect::spawn(
                    Task::new(move |ctx| {
                        if !ctx.sleep(Duration::from_secs(3)) {
                            return TaskOutcome::Cancelled;
                        }
                        ctx.emit(Msg::Loaded(text.clone()));
                        TaskOutcome::Complete
                    })
                    .key("load")
                    .on_status(Msg::Status),
                )
            }
            Msg::Loaded(text) => {
                state.loaded = Some(text);
                Effect::none()
            }
            Msg::Status(status) => {
                if state
                    .task
                    .as_ref()
                    .is_none_or(|current| current.id.get() <= status.id.get())
                {
                    state.task = Some(status);
                }
                Effect::none()
            }
            Msg::Observed => {
                state.events += 1;
                Effect::none()
            }
        }
    }

    pub fn subscription() -> Subscription<Msg> {
        Subscription::run("events", |ctx| {
            while ctx.sleep(Duration::from_secs(1)) {
                ctx.emit(Msg::Observed);
            }
            TaskOutcome::Cancelled
        })
    }
}

#[derive(Clone, Debug)]
enum Msg {
    Left(editor::Msg),
    Right(editor::Msg),
    ToggleLeft,
    ToggleRight,
    Swap,
}

struct State {
    left: Option<editor::State>,
    right: Option<editor::State>,
    reversed: bool,
}

fn update(state: &mut State, msg: Msg, ctx: &mut UpdateCtx<Msg>) {
    match msg {
        Msg::Left(msg) => {
            if let Some(left) = &mut state.left {
                ctx.dispatch(editor::update(left, msg).map(Msg::Left).scope("left"));
            }
        }
        Msg::Right(msg) => {
            if let Some(right) = &mut state.right {
                ctx.dispatch(editor::update(right, msg).map(Msg::Right).scope("right"));
            }
        }
        Msg::ToggleLeft => {
            state.left = if state.left.is_some() {
                None
            } else {
                Some(editor::State::default())
            }
        }
        Msg::ToggleRight => {
            state.right = if state.right.is_some() {
                None
            } else {
                Some(editor::State::default())
            }
        }
        Msg::Swap => state.reversed = !state.reversed,
    }
}

fn view(state: &State) -> impl Widget<Msg> {
    let left: Box<dyn Widget<Msg>> = match &state.left {
        Some(left) => editor::view(left, "Left editor")
            .map(Msg::Left)
            .scope("left")
            .into_widget(),
        None => panel()
            .title("Left editor (removed)")
            .child(label("F1: mount a fresh instance").width(28))
            .into_widget(),
    };
    let right: Box<dyn Widget<Msg>> = match &state.right {
        Some(right) => editor::view(right, "Right editor")
            .map(Msg::Right)
            .scope("right")
            .into_widget(),
        None => panel()
            .title("Right editor (removed)")
            .child(label("F2: mount a fresh instance").width(28))
            .into_widget(),
    };
    let editors = if state.reversed {
        row().gap(2).child(right).child(left)
    } else {
        row().gap(2).child(left).child(right)
    };
    col()
        .padding(Padding::uniform(1))
        .gap(1)
        .child(
            label("Message mapping & scoped modules")
                .bold()
                .fg(Color::Cyan),
        )
        .child(label(
            "Edit independently; save after 500ms, load after 3s, stream every 1s.",
        ))
        .child(editors)
        .child(
            row()
                .gap(2)
                .child(
                    button(if state.left.is_some() {
                        "Remove left [F1]"
                    } else {
                        "Mount left [F1]"
                    })
                    .key("toggle-left")
                    .on_click(|| Msg::ToggleLeft),
                )
                .child(
                    button(if state.right.is_some() {
                        "Remove right [F2]"
                    } else {
                        "Mount right [F2]"
                    })
                    .key("toggle-right")
                    .on_click(|| Msg::ToggleRight),
                )
                .child(button("Swap [F3]").key("swap").on_click(|| Msg::Swap)),
        )
        .child(label(
            "Tab: focus | Enter: load | Remove cancels work | Esc: quit",
        ))
}

fn main() -> WidgetResult<()> {
    App::new(State {
        left: Some(editor::State::default()),
        right: Some(editor::State::default()),
        reversed: false,
    })
    .on_key(KeyCode::F(1), || Msg::ToggleLeft)
    .on_key(KeyCode::F(2), || Msg::ToggleRight)
    .on_key(KeyCode::F(3), || Msg::Swap)
    .subscriptions(|state| {
        Subscription::batch([
            if state.left.is_some() {
                editor::subscription().map(Msg::Left).scope("left")
            } else {
                Subscription::none()
            },
            if state.right.is_some() {
                editor::subscription().map(Msg::Right).scope("right")
            } else {
                Subscription::none()
            },
        ])
    })
    .update(update)
    .view(view)
    .run()
    .map(|_| ())
}
