use rsille_tui::prelude::*;
use std::{cell::Cell, rc::Rc};

#[allow(dead_code)]
#[path = "../examples/basics/workflow.rs"]
mod workflow;

#[test]
fn real_application_edits_saves_and_returns_state() {
    let exit = workflow::headless().unwrap();
    assert_eq!(exit.state.saved, "中文👨‍👩‍👧‍👦");
}

struct Tracked(Rc<Cell<usize>>);
impl Widget<()> for Tracked {
    fn lifecycle(&self, ctx: &mut StateCtx) {
        if ctx.lifecycle() == Lifecycle::Unmount {
            self.0.set(self.0.get() + 1);
        }
    }
    fn render(&self, _: &mut rsille_render::chunk::Chunk, _: &RenderCtx) {}
    fn constraints(&self) -> Constraints {
        Constraints::content()
    }
}

#[test]
fn exit_unmounts_once_and_rejects_later_messages_in_the_batch() {
    let unmounts = Rc::new(Cell::new(0));
    let tracked = unmounts.clone();
    let mut app = App::new(0)
        .update(|state, (), ctx| {
            *state += 1;
            ctx.dispatch(Effect::batch([Effect::Exit, Effect::message(())]));
        })
        .view(move |_| Tracked(tracked.clone()))
        .build();
    app.step(&[]).unwrap();
    let handle = app.handle();
    handle.send(()).unwrap();
    handle.send(()).unwrap();
    assert!(app.step(&[]).unwrap().should_quit);
    assert_eq!(unmounts.get(), 1);
    assert!(app.step(&[]).unwrap().should_quit);
    let exit = app.into_exit();
    assert_eq!(exit.state, 1);
    assert_eq!(exit.reason, ExitReason::Requested);
    assert_eq!(unmounts.get(), 1);
    assert!(handle.send(()).is_err());
}

#[test]
fn quit_key_and_host_shutdown_have_distinct_reasons() {
    let mut app = App::new(String::from("final"))
        .with_quit_key(KeyCode::F(10))
        .update(|_, (), _| {})
        .view(|_| label("quit"))
        .build();
    app.step(&[Event::Key(KeyEvent::new(
        KeyCode::F(10),
        KeyModifiers::empty(),
    ))])
    .unwrap();
    let exit = app.into_exit();
    assert_eq!(exit.reason, ExitReason::QuitKey);
    assert_eq!(exit.state, "final");
    let app = App::new(42)
        .update(|_, (), _| {})
        .view(|_| label("host"))
        .build();
    assert_eq!(app.into_exit().reason, ExitReason::Shutdown);
}

#[tokio::test]
async fn sync_entry_in_tokio_returns_an_error_before_touching_the_terminal() {
    // Borrowed non-Send state and callbacks are accepted by terminal entry.
    let local = Rc::new(Cell::new(0));
    let app = App::new(local.clone())
        .update(|_, (), _| local.set(1))
        .view(|_| label("test"));
    let error = app.run().unwrap_err();
    assert!(error.to_string().contains("run_async"));
    assert_eq!(local.get(), 0);
}

#[test]
fn explicit_exit_cancels_pending_work_before_returning_state() {
    use rsille_tui::runtime::ManualExecutor;
    let executor = ManualExecutor::default();
    let executed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let marker = executed.clone();
    let mut app = App::new(false)
        .with_executor(executor.clone())
        .update(move |state, stop: bool, ctx| {
            if stop {
                ctx.exit();
            } else {
                let marker = marker.clone();
                ctx.spawn(Task::new(move |ctx| {
                    marker.store(true, std::sync::atomic::Ordering::SeqCst);
                    ctx.emit(false);
                    TaskOutcome::Complete
                }));
                ctx.after(std::time::Duration::from_secs(1), false);
                *state = true;
            }
        })
        .view(|_| label("pending"))
        .build();
    let handle = app.handle();
    handle.send(false).unwrap();
    app.step(&[]).unwrap();
    assert_eq!(executor.pending(), 1);
    handle.send(true).unwrap();
    app.step(&[]).unwrap();
    assert!(app.next_update_at().is_none());
    assert!(app
        .tasks()
        .iter()
        .all(|task| task.state == TaskState::Cancelled));
    assert!(executor.run_next());
    assert!(!executed.load(std::sync::atomic::Ordering::SeqCst));
    let exit = app.into_exit();
    assert!(exit.state);
    assert_eq!(exit.reason, ExitReason::Requested);
}

#[test]
fn plain_code_viewer_keeps_content_and_navigation() {
    let mut app = App::new(())
        .update(|_, (), _| {})
        .view(|_| {
            code_viewer("let count = 1;\n// 中文\nthird\nfourth")
                .height(2)
                .line_numbers(false)
                .borderless()
        })
        .build()
        .size(24, 2);
    app.step(&[]).unwrap();
    let text = |app: &Runtime<(), ()>| {
        app.frame()
            .content()
            .iter()
            .filter(|cell| !cell.is_occupied)
            .map(|cell| cell.content.symbol().into_owned())
            .collect::<String>()
    };
    assert!(text(&app).contains("let count = 1;"));
    app.step(&[Event::Key(KeyEvent::new(
        KeyCode::Down,
        KeyModifiers::empty(),
    ))])
    .unwrap();
    assert!(text(&app).contains("third"));
}

#[test]
fn styled_code_viewer_scrolls_numbers_and_clips_wide_graphemes() {
    let red = Style::default().fg(Color::Red).to_render_style();
    let lines: std::sync::Arc<[Line<'static>]> = vec![
        Line::from("first"),
        Line::from_texts(vec![Span::from("ab"), Span::new("界界", red)]),
        Line::from("last"),
    ]
    .into();
    let mut app = App::new(lines)
        .update(|_, (), _| {})
        .view(|lines| CodeViewer::from_lines(lines.clone()).height(1).borderless())
        .build()
        .size(8, 1);
    app.step(&[]).unwrap();
    app.step(&[Event::Key(KeyEvent::new(
        KeyCode::Down,
        KeyModifiers::empty(),
    ))])
    .unwrap();
    let cells = app.frame().content();
    assert_eq!(cells[1].content.as_char(), Some('2'));
    assert_eq!(cells[3].content.as_char(), Some('a'));
    assert_eq!(cells[5].content.as_char(), Some('界'));
    assert_eq!(
        cells[5].content.style.colors.unwrap().foreground,
        red.colors.unwrap().foreground
    );
    assert!(cells[6].is_occupied);
    assert_eq!(cells[7].content.as_char(), Some(' '));
}
