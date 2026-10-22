//! Edit → debounce → background save → exit, with the same app in both modes.
//! cargo run -p tui --example workflow -- --headless
use std::time::Duration;
use tui::prelude::*;
use tui::runtime::{ManualClock, ManualExecutor};

// Messages deliberately require neither Clone nor Debug.
pub enum Msg {
    Edit(String),
    Save,
    Saved(String),
    Exit,
}

#[derive(Default)]
pub struct State {
    pub text: String,
    pub saved: String,
}

pub fn update(state: &mut State, message: Msg, ctx: &mut UpdateCtx<Msg>) {
    match message {
        Msg::Edit(text) => {
            state.text = text;
            ctx.debounce("save", Duration::from_millis(300), Msg::Save);
        }
        Msg::Save => {
            let text = state.text.clone();
            ctx.spawn(
                Task::new(move |ctx| {
                    ctx.emit(Msg::Saved(text.clone()));
                    TaskOutcome::Complete
                })
                .key("save"),
            );
        }
        Msg::Saved(text) => state.saved = text,
        Msg::Exit => ctx.exit(),
    }
}

pub fn view(state: &State) -> impl Widget<Msg> {
    col()
        .child(
            text_input()
                .key("editor")
                .value(&state.text)
                .on_change(Msg::Edit),
        )
        .child(label(format!("Saved: {}", state.saved)))
        .child(button("Exit").key("exit").on_click(|| Msg::Exit))
}

pub fn headless() -> Result<AppExit<State>, Box<dyn std::error::Error>> {
    let clock = ManualClock::default();
    let executor = ManualExecutor::default();
    let mut app = App::new(State::default())
        .mode(Mode::Inline)
        .with_clock(clock.clone())
        .with_executor(executor.clone())
        .update(update)
        .view(view)
        .build()
        .size(40, 6);
    app.step(&[Event::Paste("中文👨‍👩‍👧‍👦".into())])?;
    assert_eq!(app.focused_key(), Some("editor"));
    assert_eq!(app.state().text, "中文👨‍👩‍👧‍👦");
    clock.advance(Duration::from_millis(300));
    app.step(&[])?;
    assert_eq!(executor.pending(), 1);
    assert!(executor.run_next());
    app.step(&[])?;
    assert_eq!(app.state().saved, app.state().text);
    assert!(app
        .tasks()
        .iter()
        .any(|task| task.state == TaskState::Completed));
    app.step(&[Event::Key(KeyEvent::new(
        KeyCode::Tab,
        KeyModifiers::empty(),
    ))])?;
    assert_eq!(app.focused_key(), Some("exit"));
    assert!(
        app.step(&[Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::empty()
        ))])?
        .should_quit
    );
    let exit = app.into_exit();
    assert_eq!(exit.reason, ExitReason::Requested);
    Ok(exit)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let exit = if std::env::args().any(|arg| arg == "--headless") {
        headless()?
    } else {
        let app = App::new(State::default())
            .mode(Mode::Inline)
            .update(update)
            .view(view);
        if std::env::args().any(|arg| arg == "--async") {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(app.run_async())?
        } else {
            app.run()?
        }
    };
    println!("Exit: {:?}; saved: {}", exit.reason, exit.state.saved);
    Ok(())
}
