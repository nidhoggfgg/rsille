use std::sync::{Arc, Mutex};
use std::time::Duration;
use tui::prelude::*;
use tui::runtime::{Clock, ManualClock, ManualExecutor};

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
}

fn frame_text<M: Clone + std::fmt::Debug + Send + 'static, S>(app: &Runtime<S, M>) -> String {
    app.frame()
        .content()
        .iter()
        .filter(|cell| !cell.is_occupied)
        .filter_map(|cell| cell.content.as_char())
        .collect()
}

#[test]
fn input_update_layout_and_draw_work_without_a_terminal() {
    let mut app = App::new(String::new())
        .with_motion_policy(MotionPolicy::disabled())
        .build(
            |state, text| *state = text,
            |state| {
                col()
                    .child(
                        text_input()
                            .key("search")
                            .value(state)
                            .on_change(|text| text),
                    )
                    .child(button("clear").key("clear").on_click(String::new))
            },
        )
        .size(24, 6);

    let first = app.step(&[]).unwrap();
    assert!(first.rendered);
    assert_eq!(app.focused_key(), Some("search"));
    app.step(&[key(KeyCode::Char('中')), key(KeyCode::Char('a'))])
        .unwrap();
    assert_eq!(app.state(), "中a");
    assert!(frame_text(&app).contains("中a"));
    let state = app
        .widget_state::<tui::widgets::text_input::TextInputState>(&WidgetId::root().child("search"))
        .unwrap();
    assert_eq!(state.cursor_position, "中a".len());

    app.step(&[key(KeyCode::Tab)]).unwrap();
    assert_eq!(app.focused_key(), Some("clear"));
    app.step(&[key(KeyCode::Enter)]).unwrap();
    assert!(app.state().is_empty());

    app.step(&[Event::Resize(32, 8)]).unwrap();
    assert_eq!(app.frame().size(), (32, 8).into());
    let bounds = app.bounds(&WidgetPath::root().child("search")).unwrap();
    assert!(bounds.width() <= 32);
    assert!(bounds.height() <= 8);
}

#[test]
fn first_step_can_route_mouse_using_initial_geometry() {
    let mut app = App::new(0)
        .with_motion_policy(MotionPolicy::disabled())
        .build(|state, ()| *state += 1, |_| button("click").on_click(|| ()))
        .size(20, 3);
    app.step(&[Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 1,
        row: 1,
        modifiers: KeyModifiers::empty(),
    })])
    .unwrap();
    assert_eq!(*app.state(), 1);
}

#[test]
fn idle_runtime_has_no_periodic_deadline_or_redraw() {
    let mut app = App::new(0).build(|state, value| *state = value, |n| label(n.to_string()));
    assert_eq!(app.step(&[]).unwrap().next_update_at, None);
    let idle = app.step(&[]).unwrap();
    assert!(!idle.rendered);
    assert_eq!(idle.next_update_at, None);
}

#[test]
fn external_thread_can_submit_messages_and_closed_handle_returns_payload() {
    let mut app = App::new(0).build(|state, value| *state = value, |n| label(n.to_string()));
    app.step(&[]).unwrap();
    let handle = app.handle();
    let producer = handle.clone();
    std::thread::spawn(move || producer.send(42).unwrap())
        .join()
        .unwrap();
    assert!(app.step(&[]).unwrap().rendered);
    assert_eq!(*app.state(), 42);
    assert!(frame_text(&app).contains("42"));
    drop(app);
    assert_eq!(handle.send(7).unwrap_err().0, 7);
}

#[derive(Clone, Debug)]
enum TimerMsg {
    After,
    Debounce(u32),
    Cancel,
    Fired(u32),
}

#[test]
fn timers_and_replaced_debounce_follow_virtual_time_without_jobs() {
    let clock = ManualClock::default();
    let executor = ManualExecutor::default();
    let mut app = App::new(Vec::new())
        .with_clock(clock.clone())
        .with_executor(executor.clone())
        .build_with_effects(
            |state, message, ctx| match message {
                TimerMsg::After => ctx.after(Duration::from_millis(100), TimerMsg::Fired(999)),
                TimerMsg::Debounce(n) => {
                    ctx.debounce("search", Duration::from_millis(50), TimerMsg::Fired(n))
                }
                TimerMsg::Cancel => ctx.cancel_debounce("search"),
                TimerMsg::Fired(n) => state.push((n, ctx.now())),
            },
            |_| label("timers"),
        );
    let handle = app.handle();
    let start = clock.now();
    handle.send(TimerMsg::After).unwrap();
    for n in 0..1000 {
        handle.send(TimerMsg::Debounce(n)).unwrap();
    }
    // Drain the bounded ingress across successive turns.
    for _ in 0..5 {
        app.step(&[]).unwrap();
    }
    assert_eq!(executor.pending(), 0);
    assert_eq!(
        app.next_update_at(),
        Some(start + Duration::from_millis(50))
    );
    clock.advance(Duration::from_millis(49));
    app.step(&[]).unwrap();
    assert!(app.state().is_empty());
    clock.advance(Duration::from_millis(1));
    app.step(&[]).unwrap();
    assert_eq!(app.state(), &[(999, clock.now())]);
    handle.send(TimerMsg::Debounce(5)).unwrap();
    app.step(&[]).unwrap();
    handle.send(TimerMsg::Cancel).unwrap();
    app.step(&[]).unwrap();
    clock.advance(Duration::from_millis(50));
    app.step(&[]).unwrap();
    assert_eq!(
        app.state(),
        &[(999, start + Duration::from_millis(50)), (999, clock.now())]
    );
    assert_eq!(app.next_update_at(), None);
}

#[test]
fn ticks_coalesce_and_frame_callbacks_use_the_injected_clock() {
    let clock = ManualClock::default();
    let mut app = App::new(Vec::new())
        .with_clock(clock.clone())
        .on_tick(Duration::from_millis(10), || None)
        .on_frame(Some)
        .build(|state, message| state.push(message), |_| label("clock"));
    app.step(&[]).unwrap();
    let start = clock.now();
    assert_eq!(app.state().len(), 1);
    assert_eq!(app.state()[0].unwrap().delta, Duration::ZERO);
    clock.advance(Duration::from_millis(100));
    app.step(&[]).unwrap();
    assert_eq!(app.state().len(), 3);
    assert!(app.state()[1].is_none());
    let frame = app.state()[2].unwrap();
    assert_eq!(frame.now, clock.now());
    assert_eq!(frame.since_start, clock.now() - start);
    assert_eq!(frame.delta, Duration::from_millis(100));
    assert_eq!(frame.frame, 1);
    app.step(&[]).unwrap();
    assert_eq!(app.state().len(), 3);
}

#[derive(Clone, Debug)]
enum TaskMsg {
    Start,
    Cancel,
    Result(u32),
    Status(TaskStatus),
}

#[test]
fn task_results_and_statuses_are_driven_by_the_controlled_executor() {
    let clock = ManualClock::default();
    let executor = ManualExecutor::default();
    let mut app = App::new(Vec::new())
        .with_clock(clock.clone())
        .with_executor(executor.clone())
        .build_with_effects(
            |state, message, ctx| match message {
                TaskMsg::Start => ctx.spawn(
                    Task::new(|task| {
                        task.emit(TaskMsg::Result(42));
                        TaskOutcome::Complete
                    })
                    .key("load")
                    .on_status(TaskMsg::Status),
                ),
                TaskMsg::Result(n) => state.push(n),
                TaskMsg::Status(status) => assert_eq!(status.updated_at, ctx.now()),
                TaskMsg::Cancel => ctx.cancel_task_key("load"),
            },
            |_| label("task"),
        );
    app.handle().send(TaskMsg::Start).unwrap();
    app.step(&[]).unwrap();
    assert_eq!(app.tasks()[0].state, TaskState::Running);
    assert_eq!(app.next_update_at(), None);
    assert_eq!(executor.pending(), 1);
    assert!(executor.run_next());
    app.step(&[]).unwrap();
    assert_eq!(app.state(), &[42]);
    assert_eq!(app.tasks()[0].state, TaskState::Completed);
    assert_eq!(app.next_update_at(), None);
}

#[test]
fn timeout_rejects_late_messages_and_clears_the_deadline() {
    let clock = ManualClock::default();
    let executor = ManualExecutor::default();
    // Keep a live producer after its runner returns, to exercise late emissions.
    let context = Arc::new(Mutex::new(None));
    let captured = context.clone();
    let mut app = App::new(Vec::new())
        .with_clock(clock.clone())
        .with_executor(executor.clone())
        .build_with_effects(
            move |state, message, ctx| match message {
                TaskMsg::Start => {
                    let captured = captured.clone();
                    ctx.spawn(
                        Task::new(move |task| {
                            *captured.lock().unwrap() = Some(task);
                            TaskOutcome::Complete
                        })
                        .timeout(Duration::from_secs(1)),
                    );
                }
                TaskMsg::Result(n) => state.push(n),
                _ => (),
            },
            |_| label("timeout"),
        );
    app.handle().send(TaskMsg::Start).unwrap();
    app.step(&[]).unwrap();
    executor.run_next(); // Completion is queued but not yet observed by the UI.
    clock.advance(Duration::from_secs(1));
    app.step(&[]).unwrap();
    assert_eq!(app.tasks()[0].state, TaskState::TimedOut);
    let task = context.lock().unwrap().take().unwrap();
    assert!(task.is_cancelled());
    task.emit(TaskMsg::Result(99));
    app.step(&[]).unwrap();
    assert!(app.state().is_empty());
    assert_eq!(app.next_update_at(), None);
}

#[test]
fn retry_wait_and_cancellation_use_virtual_time() {
    let clock = ManualClock::default();
    let executor = ManualExecutor::default();
    let mut app = App::new(0)
        .with_clock(clock.clone())
        .with_executor(executor.clone())
        .build_with_effects(
            |state, message, ctx| match message {
                TaskMsg::Start => ctx.spawn(
                    Task::new(|task| {
                        if task.attempt() == 1 {
                            TaskOutcome::Retry
                        } else {
                            task.emit(TaskMsg::Result(42));
                            TaskOutcome::Complete
                        }
                    })
                    .key("load")
                    .retry(RetryPolicy::fixed(1, Duration::from_secs(1))),
                ),
                TaskMsg::Cancel => ctx.cancel_task_key("load"),
                TaskMsg::Result(n) => *state = n,
                _ => (),
            },
            |_| label("retry"),
        );
    app.handle().send(TaskMsg::Start).unwrap();
    app.step(&[]).unwrap();
    executor.run_next();
    app.step(&[]).unwrap();
    assert_eq!(app.tasks()[0].state, TaskState::RetryScheduled);
    assert_eq!(
        app.next_update_at(),
        Some(clock.now() + Duration::from_secs(1))
    );
    clock.advance(Duration::from_millis(999));
    app.step(&[]).unwrap();
    assert_eq!(executor.pending(), 0);
    clock.advance(Duration::from_millis(1));
    app.step(&[]).unwrap();
    assert_eq!(app.tasks()[0].attempt, 2);
    executor.run_next();
    app.step(&[]).unwrap();
    assert_eq!(*app.state(), 42);
    assert_eq!(app.tasks()[0].state, TaskState::Completed);

    app.handle().send(TaskMsg::Start).unwrap();
    app.step(&[]).unwrap();
    executor.run_next();
    app.step(&[]).unwrap();
    app.handle().send(TaskMsg::Cancel).unwrap();
    app.step(&[]).unwrap();
    assert_eq!(app.tasks()[1].state, TaskState::Cancelled);
    assert_eq!(app.next_update_at(), None);
    clock.advance(Duration::from_secs(2));
    app.step(&[]).unwrap();
    assert_eq!(executor.pending(), 0);
}

#[test]
fn dropping_runtime_cancels_jobs_without_running_or_joining_them() {
    let executor = ManualExecutor::default();
    let ran = Arc::new(Mutex::new(false));
    let check = ran.clone();
    let mut app = App::new(())
        .with_executor(executor.clone())
        .build_with_effects(
            move |_, (), ctx| {
                let check = check.clone();
                ctx.spawn(Task::new(move |_| {
                    *check.lock().unwrap() = true;
                    TaskOutcome::Complete
                }));
            },
            |_| label("shutdown"),
        );
    app.handle().send(()).unwrap();
    app.step(&[]).unwrap();
    drop(app);
    executor.run_next();
    assert!(!*ran.lock().unwrap());
}

#[test]
fn replacing_a_task_key_rejects_an_old_result_already_in_the_queue() {
    let executor = ManualExecutor::default();
    let mut app = App::new(Vec::new())
        .with_executor(executor.clone())
        .build_with_effects(
            |state, message, ctx| match message {
                TaskMsg::Start => ctx.spawn(
                    Task::new(|task| {
                        task.emit(TaskMsg::Result(task.task_id().get() as u32));
                        TaskOutcome::Complete
                    })
                    .key("load"),
                ),
                TaskMsg::Result(n) => state.push(n),
                _ => (),
            },
            |_| label("replace"),
        );
    app.handle().send(TaskMsg::Start).unwrap();
    app.step(&[]).unwrap();
    app.handle().send(TaskMsg::Start).unwrap();
    executor.run_next(); // Old result follows the replacement message.
    app.step(&[]).unwrap();
    assert!(app.state().is_empty());
    executor.run_next();
    app.step(&[]).unwrap();
    assert_eq!(app.state(), &[1]);
}

#[test]
fn quitting_stops_pending_tasks_and_timers_while_the_runtime_is_retained() {
    let clock = ManualClock::default();
    let executor = ManualExecutor::default();
    let ran = Arc::new(Mutex::new(false));
    let check = ran.clone();
    let mut app = App::new(0)
        .with_clock(clock.clone())
        .with_executor(executor.clone())
        .build_with_effects(
            move |state, message, ctx| match message {
                TaskMsg::Start => {
                    let check = check.clone();
                    ctx.spawn(Task::new(move |_| {
                        *check.lock().unwrap() = true;
                        TaskOutcome::Complete
                    }));
                    ctx.after(Duration::from_secs(1), TaskMsg::Result(99));
                }
                TaskMsg::Result(n) => *state = n,
                _ => (),
            },
            |_| label("quit"),
        );
    app.handle().send(TaskMsg::Start).unwrap();
    app.step(&[]).unwrap();
    assert!(app.next_update_at().is_some());
    assert!(app.step(&[key(KeyCode::Esc)]).unwrap().should_quit);
    assert_eq!(app.tasks()[0].state, TaskState::Cancelled);
    assert_eq!(app.next_update_at(), None);
    clock.advance(Duration::from_secs(2));
    executor.run_next();
    app.step(&[]).unwrap();
    assert!(!*ran.lock().unwrap());
    assert_eq!(*app.state(), 0);
}

#[test]
fn message_flood_and_recursive_effects_leave_time_for_input_and_drawing() {
    let mut app = App::new(0u32)
        .on_key(KeyCode::Char('x'), || false)
        .build_with_effects(
            |state, again, ctx| {
                *state += 1;
                if again {
                    ctx.emit(true);
                }
            },
            |n| label(n.to_string()),
        );
    for _ in 0..1000 {
        app.handle().send(true).unwrap();
    }
    let first = app.step(&[]).unwrap();
    assert_eq!(*app.state(), 256);
    assert!(first.rendered);
    assert!(first.next_update_at.is_some());
    app.step(&[key(KeyCode::Char('x'))]).unwrap();
    assert_eq!(*app.state(), 512);
    let exit = app.step(&[key(KeyCode::Esc)]).unwrap();
    assert!(exit.should_quit);
}

#[test]
fn finite_presence_animation_can_be_advanced_without_sleeping() {
    let clock = ManualClock::default();
    let mut app = App::<(), ()>::new(()).with_clock(clock.clone()).build(
        |_, _| (),
        |_| {
            animate(label("hello")).enter(Transition::new(
                TransitionEffect::Fade,
                AnimationSpec::new(Duration::from_millis(100), Easing::Linear),
            ))
        },
    );
    app.step(&[]).unwrap();
    assert!(app.next_update_at().is_some());
    clock.advance(Duration::from_millis(50));
    assert!(app.step(&[]).unwrap().rendered);
    clock.advance(Duration::from_millis(100));
    app.step(&[]).unwrap();
    // One final update consumes the animation's final changed value.
    clock.advance(Duration::from_millis(20));
    app.step(&[]).unwrap();
    assert_eq!(app.next_update_at(), None);
}
