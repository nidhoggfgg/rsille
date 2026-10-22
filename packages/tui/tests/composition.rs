use std::sync::{Arc, Mutex};
use std::time::Duration;
use tui::prelude::*;
use tui::runtime::{ManualClock, ManualExecutor};
use tui::widgets::TextInputState;

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
}

// This module deliberately has no dependency on the application's Msg enum.
mod editor {
    use super::*;
    #[derive(Clone, Debug)]
    pub enum Msg {
        Edit(String),
        Work,
        Result,
        Saved,
        Status(TaskState),
        Stream,
    }
    #[derive(Default)]
    pub struct State {
        pub value: String,
        pub results: usize,
        pub saves: usize,
        pub stream: usize,
        pub statuses: Vec<TaskState>,
    }
    pub fn view(state: &State) -> impl Widget<Msg> {
        col()
            .child(
                text_input()
                    .key("input")
                    .value(&state.value)
                    .on_change(Msg::Edit),
            )
            .child(button("work").key("work").on_click(|| Msg::Work))
    }
    pub fn update(state: &mut State, msg: Msg) -> Effect<Msg> {
        match msg {
            Msg::Edit(value) => {
                state.value = value;
                Effect::debounce("save", Duration::from_millis(100), Msg::Saved)
            }
            Msg::Work => Effect::spawn(
                Task::new(|ctx| {
                    ctx.emit(Msg::Result);
                    TaskOutcome::Complete
                })
                .key("load")
                .on_status(|s| Msg::Status(s.state)),
            ),
            Msg::Result => {
                state.results += 1;
                Effect::none()
            }
            Msg::Saved => {
                state.saves += 1;
                Effect::none()
            }
            Msg::Stream => {
                state.stream += 1;
                Effect::none()
            }
            Msg::Status(s) => {
                state.statuses.push(s);
                Effect::none()
            }
        }
    }
    pub fn subscription() -> Subscription<Msg> {
        Subscription::run("events", |ctx| {
            ctx.emit(Msg::Stream);
            TaskOutcome::Complete
        })
    }
}

#[derive(Clone, Debug)]
enum Msg {
    Left(editor::Msg),
    Right(editor::Msg),
    Remove,
    Restore,
    Swap,
    CancelLeft,
}
#[derive(Default)]
struct State {
    left: editor::State,
    right: editor::State,
    removed: bool,
    swap: bool,
}
fn update(s: &mut State, msg: Msg, ctx: &mut UpdateCtx<Msg>) {
    match msg {
        Msg::Left(msg) => ctx.dispatch(
            editor::update(&mut s.left, msg)
                .map(Msg::Left)
                .scope("left"),
        ),
        Msg::Right(msg) => ctx.dispatch(
            editor::update(&mut s.right, msg)
                .map(Msg::Right)
                .scope("right"),
        ),
        Msg::Remove => s.removed = true,
        Msg::Restore => s.removed = false,
        Msg::Swap => s.swap = !s.swap,
        Msg::CancelLeft => {
            let left = ctx
                .task_status_in_scope(&Scope::new("left"), "load")
                .unwrap();
            assert_eq!(left.scope, Scope::new("left"));
            assert!(ctx.task_status_by_key("load").is_none());
            ctx.dispatch(Effect::cancel_task_key("load").scope("left"));
            ctx.dispatch(Effect::cancel_debounce("save").scope("left"));
        }
    }
}
fn view(s: &State) -> impl Widget<Msg> {
    let left = editor::view(&s.left).map(Msg::Left).scope("left");
    let right = editor::view(&s.right).map(Msg::Right).scope("right");
    if s.removed {
        col().child(right)
    } else if s.swap {
        col().child(right).child(col().child(left))
    } else {
        col().child(left).child(right)
    }
}

#[test]
fn two_modules_keep_local_keys_input_focus_and_state_when_reordered() {
    let mut app = App::new(State::default())
        .with_motion_policy(MotionPolicy::disabled())
        .build_with_effects(update, view)
        .size(30, 16);
    app.step(&[
        key(KeyCode::Char('a')),
        key(KeyCode::Char('b')),
        key(KeyCode::Char('中')),
    ])
    .unwrap();
    assert_eq!(app.state().left.value, "ab中");
    assert!(app.state().right.value.is_empty());
    let left_id = WidgetId::root().child("left").child("input");
    assert_eq!(
        app.widget_state::<TextInputState>(&left_id)
            .unwrap()
            .cursor_position,
        5
    );
    app.step(&[key(KeyCode::Left)]).unwrap();
    app.handle().send(Msg::Swap).unwrap();
    app.step(&[]).unwrap();
    assert_eq!(
        app.widget_state::<TextInputState>(&left_id)
            .unwrap()
            .cursor_position,
        2
    );
    assert_eq!(app.focused_key(), Some("input"));
    assert!(app
        .focused_path()
        .unwrap()
        .as_slice()
        .contains(&WidgetKey::from("left")));
    // Tab visits the left button, then the first input in the reordered tree.
    app.step(&[
        key(KeyCode::Tab),
        key(KeyCode::Tab),
        key(KeyCode::Char('x')),
    ])
    .unwrap();
    assert_eq!(app.state().right.value, "x");
    assert_eq!(app.state().left.value, "ab中");
    assert!(app.bounds(app.focused_path().unwrap()).unwrap().width() > 0);
}

#[test]
fn local_task_and_debounce_keys_and_cancellation_are_isolated() {
    let clock = ManualClock::default();
    let executor = ManualExecutor::default();
    let mut app = App::new(State::default())
        .with_clock(clock.clone())
        .with_executor(executor.clone())
        .build_with_effects(update, view);
    for msg in [
        Msg::Left(editor::Msg::Work),
        Msg::Right(editor::Msg::Work),
        Msg::Left(editor::Msg::Edit("a".into())),
        Msg::Right(editor::Msg::Edit("b".into())),
    ] {
        app.handle().send(msg).unwrap();
    }
    app.step(&[]).unwrap();
    assert_eq!(executor.pending(), 2);
    assert!(app.tasks().iter().all(|t| t.state == TaskState::Running));
    app.handle().send(Msg::CancelLeft).unwrap();
    app.step(&[]).unwrap();
    assert!(executor.run_next());
    assert!(executor.run_next());
    app.step(&[]).unwrap();
    clock.advance(Duration::from_millis(100));
    app.step(&[]).unwrap();
    assert_eq!(app.state().left.results, 0);
    assert_eq!(app.state().right.results, 1);
    assert_eq!(app.state().left.saves, 0);
    assert_eq!(app.state().right.saves, 1);
    assert!(app.state().left.statuses.contains(&TaskState::Cancelled));
    assert!(app.state().right.statuses.contains(&TaskState::Completed));
}

#[test]
fn subscriptions_keep_stable_ids_and_reject_results_from_unmounted_generations() {
    let executor = ManualExecutor::default();
    let mut app = App::new(State::default())
        .with_executor(executor.clone())
        .subscriptions(|s| {
            Subscription::batch([
                if s.removed {
                    Subscription::none()
                } else {
                    editor::subscription().map(Msg::Left).scope("left")
                },
                editor::subscription().map(Msg::Right).scope("right"),
            ])
        })
        .build_with_effects(update, view);
    app.step(&[]).unwrap();
    assert_eq!(executor.pending(), 2);
    let initial: Vec<_> = app.tasks().iter().map(|t| t.id).collect();
    app.handle().send(Msg::Swap).unwrap();
    app.step(&[]).unwrap();
    assert_eq!(executor.pending(), 2);
    assert_eq!(
        initial,
        app.tasks().iter().map(|t| t.id).collect::<Vec<_>>()
    );
    // Removal is queued before the old producer's result, all in one turn.
    app.handle().send(Msg::Remove).unwrap();
    assert!(executor.run_next());
    assert!(executor.run_next());
    app.handle().send(Msg::Restore).unwrap();
    app.step(&[]).unwrap();
    assert_eq!(app.state().left.stream, 0);
    assert_eq!(app.state().right.stream, 1);
    assert_eq!(executor.pending(), 1);
    assert!(executor.run_next());
    app.step(&[]).unwrap();
    assert_eq!(app.state().left.stream, 1);
    // Completed producers remain matched until removed, avoiding restart loops.
    app.step(&[]).unwrap();
    assert_eq!(executor.pending(), 0);
    assert_eq!(app.tasks().len(), 3);
}

#[test]
fn unmount_cancels_timers_and_queued_task_results_in_the_same_batch() {
    let clock = ManualClock::default();
    let executor = ManualExecutor::default();
    let mut app = App::new(State::default())
        .with_clock(clock.clone())
        .with_executor(executor.clone())
        .build_with_effects(
            |s, msg, ctx| {
                if matches!(msg, Msg::Left(editor::Msg::Work)) {
                    ctx.dispatch(
                        Effect::after(Duration::from_millis(100), Msg::Left(editor::Msg::Saved))
                            .scope("left"),
                    );
                }
                update(s, msg, ctx);
            },
            view,
        );
    app.handle().send(Msg::Left(editor::Msg::Work)).unwrap();
    app.handle()
        .send(Msg::Left(editor::Msg::Edit("old".into())))
        .unwrap();
    app.step(&[]).unwrap();
    app.handle().send(Msg::Remove).unwrap();
    executor.run_next();
    app.handle().send(Msg::Restore).unwrap();
    app.step(&[]).unwrap();
    clock.advance(Duration::from_millis(100));
    app.step(&[]).unwrap();
    assert_eq!(app.state().left.results, 0);
    assert_eq!(app.state().left.saves, 0);
    assert_eq!(app.next_update_at(), None);
}

#[test]
fn nested_scopes_use_segments_and_keep_task_and_subscription_names_separate() {
    let executor = ManualExecutor::default();
    let mut app = App::new(Vec::<&'static str>::new())
        .with_executor(executor.clone())
        .subscriptions(|_| {
            Subscription::run("same", |ctx| {
                ctx.emit("subscription");
                TaskOutcome::Complete
            })
            .scope("inner")
            .scope("outer")
        })
        .build_with_effects(
            |s, msg, ctx| {
                s.push(msg);
                if msg == "start" {
                    ctx.dispatch(
                        Effect::spawn(
                            Task::new(|ctx| {
                                ctx.emit("task");
                                TaskOutcome::Complete
                            })
                            .key("same"),
                        )
                        .scope("inner")
                        .scope("outer"),
                    );
                    ctx.dispatch(
                        Effect::debounce("same", Duration::ZERO, "flat").scope("outer/inner"),
                    );
                    ctx.dispatch(
                        Effect::debounce("same", Duration::ZERO, "nested")
                            .scope(Scope::new("outer").child("inner")),
                    );
                }
            },
            |_| {
                col::<&'static str>()
                    .child(label("nested").scope("inner").scope("outer"))
                    .child(label("flat").scope("outer/inner"))
            },
        );
    app.handle().send("start").unwrap();
    app.step(&[]).unwrap();
    assert_eq!(executor.pending(), 2);
    executor.run_next();
    executor.run_next();
    app.step(&[]).unwrap();
    for msg in ["task", "subscription", "nested", "flat"] {
        assert!(app.state().contains(&msg));
    }
    assert!(app.tasks().iter().all(|t| t.state == TaskState::Completed));
}

#[test]
fn retained_scopes_keep_resources_until_their_description_is_removed() {
    let executor = ManualExecutor::default();
    let token = Arc::new(Mutex::new(None));
    let mut app = App::new((true, false))
        .with_executor(executor.clone())
        .subscriptions({
            let token = token.clone();
            move |_| {
                let token = token.clone();
                Subscription::run("watch", move |ctx| {
                    *token.lock().unwrap() = Some(ctx.cancellation_token());
                    TaskOutcome::Retry
                })
                .scope("editor")
            }
        })
        .build(
            |s, msg| *s = msg,
            |s| {
                let mut root = col();
                if s.0 {
                    root = root.child(
                        collapsible("editor")
                            .expanded(s.1)
                            .retain_state(true)
                            .child(
                                editor::view(&editor::State::default())
                                    .map(|_| (true, false))
                                    .scope("editor"),
                            ),
                    );
                }
                root
            },
        );
    app.step(&[]).unwrap();
    assert_eq!(executor.pending(), 1);
    executor.run_next();
    // The producer returned but its completion has not yet been collected.
    assert!(!token.lock().unwrap().as_ref().unwrap().is_cancelled());
    app.handle().send((false, false)).unwrap();
    app.step(&[]).unwrap();
    // Finished tasks are no longer executing; pending resources still vanish.
    assert_eq!(app.next_update_at(), None);
    app.handle().send((true, false)).unwrap();
    app.step(&[]).unwrap();
    assert_eq!(executor.pending(), 1);
    app.shutdown();
    assert_eq!(app.next_update_at(), None);
    assert!(app.tasks().last().unwrap().state == TaskState::Cancelled);
}

#[test]
fn mapped_request_preserves_retry_status_payload_and_timeout() {
    #[derive(Clone, Debug)]
    enum Message {
        Start,
        Request(RequestEvent<u32, &'static str>),
        Timeout(TaskState),
    }
    let clock = ManualClock::default();
    let executor = ManualExecutor::default();
    let mut app = App::new((RequestState::new(), Vec::new()))
        .with_clock(clock.clone())
        .with_executor(executor.clone())
        .build_with_effects(
            |state, msg, ctx| match msg {
                Message::Start => {
                    let request = Request::new(|ctx| {
                        if ctx.attempt() == 1 {
                            RequestOutcome::failure("retry")
                        } else {
                            RequestOutcome::success(42)
                        }
                    })
                    .key("load")
                    .retry(RetryPolicy::fixed(1, Duration::from_millis(10)));
                    ctx.dispatch(
                        Effect::spawn(request.into_task(|event| event))
                            .map(Message::Request)
                            .scope("editor"),
                    );
                    ctx.dispatch(
                        Effect::spawn(
                            Task::new(|_| TaskOutcome::Complete)
                                .key("timeout")
                                .timeout(Duration::from_millis(5))
                                .on_status(|s| s.state),
                        )
                        .map(Message::Timeout)
                        .scope("editor"),
                    );
                }
                Message::Request(event) => {
                    state.0.apply(event);
                }
                Message::Timeout(state_) => state.1.push(state_),
            },
            |_| label("editor").scope("editor"),
        );
    app.handle().send(Message::Start).unwrap();
    app.step(&[]).unwrap();
    executor.run_next();
    app.step(&[]).unwrap();
    assert_eq!(
        app.state().0.status().unwrap().state,
        TaskState::RetryScheduled
    );
    clock.advance(Duration::from_millis(10));
    app.step(&[]).unwrap();
    executor.run_next(); // cancelled timeout attempt
    executor.run_next(); // request's second attempt
    app.step(&[]).unwrap();
    assert_eq!(app.state().0.value(), Some(&42));
    assert!(app.state().1.contains(&TaskState::TimedOut));
}

#[test]
fn mapping_preserves_event_phases_outcomes_metadata_and_animation_declarations() {
    use render::area::Area;
    use render::chunk::Chunk;
    use std::collections::HashMap;
    use tui::animation::AnimationCtx;
    use tui::widget::EventOutcome;
    struct Probe {
        children: Vec<Box<dyn Widget<u8>>>,
        presence: Presence,
        shared: SharedTransition,
        phases: Arc<Mutex<Vec<EventPhase>>>,
    }
    impl Widget<u8> for Probe {
        fn render(&self, _: &mut Chunk, _: &RenderCtx) {}
        fn constraints(&self) -> Constraints {
            Constraints::fixed(17, 3)
        }
        fn measure(&self, _: SizeProposal, ctx: &MeasureCtx) -> MeasuredSize {
            assert_eq!(ctx.path(), &WidgetPath::root());
            MeasuredSize::new(17, 3)
        }
        fn handle_event(&self, _: &Event, ctx: &mut EventCtx<u8>) {
            self.phases.lock().unwrap().push(ctx.phase());
            assert!(ctx.was_handled());
            assert_eq!(ctx.bounds().unwrap().width(), 17);
            ctx.state_mut::<usize>().clone_from(&42);
            ctx.emit(7);
            ctx.request_focus_self();
            ctx.stop_propagation();
        }
        fn children(&self) -> &[Box<dyn Widget<u8>>] {
            &self.children
        }
        fn key(&self) -> Option<&str> {
            Some("probe")
        }
        fn animate(&self, _: &mut AnimationCtx) -> bool {
            true
        }
        fn presence(&self) -> Option<&Presence> {
            Some(&self.presence)
        }
        fn layout_transition(&self) -> Option<LayoutTransition> {
            Some(LayoutTransition::none())
        }
        fn shared_transition(&self) -> Option<&SharedTransition> {
            Some(&self.shared)
        }
        fn hit_test_mode(&self) -> HitTestMode {
            HitTestMode::None
        }
        fn focus_config(&self) -> FocusConfig {
            FocusConfig::Leaf
        }
    }
    let phases = Arc::new(Mutex::new(Vec::new()));
    let original = Probe {
        children: vec![Box::new(button("child").key("child").on_click(|| 9))],
        presence: Presence::new().exit(Transition::fade_out()),
        shared: SharedTransition::new("shared", LayoutTransition::none()),
        phases: phases.clone(),
    };
    let ty = original.widget_type();
    let widget = original.map(u32::from).map(|n| n.to_string());
    assert_eq!(widget.widget_type(), ty);
    assert_eq!(widget.key(), Some("probe"));
    assert_eq!(widget.children()[0].key(), Some("child"));
    assert!(widget.presence().unwrap().exit.is_some());
    assert!(widget.layout_transition().is_some());
    assert_eq!(widget.shared_transition().unwrap().id, "shared");
    assert_eq!(widget.hit_test_mode(), HitTestMode::None);
    assert!(matches!(widget.focus_config(), FocusConfig::Leaf));
    let mut animations = tui::animation::AnimationStore::new();
    assert!(widget.animate(&mut AnimationCtx::new(
        &mut animations,
        WidgetPath::root(),
        None,
        std::time::Instant::now()
    )));
    let mut store = WidgetStore::new();
    let theme = Theme::dark();
    assert_eq!(
        widget.measure(SizeProposal::UNBOUNDED, &MeasureCtx::new(&store, &theme)),
        MeasuredSize::new(17, 3)
    );
    let geometry = HashMap::from([(WidgetPath::root(), Area::new((0, 0).into(), (17, 3).into()))]);
    for phase in [EventPhase::Capture, EventPhase::Target, EventPhase::Bubble] {
        let mut messages = Vec::new();
        let mut ctx = EventCtx::new(
            &mut store,
            &mut messages,
            WidgetPath::root(),
            WidgetId::root(),
            Some(WidgetPath::root()),
            &geometry,
            phase,
            true,
        );
        widget.handle_event(&key(KeyCode::Enter), &mut ctx);
        assert_eq!(
            ctx.finish(),
            EventOutcome {
                handled: true,
                stop_propagation: true,
                focus_request: Some(FocusRequest::Set(WidgetPath::root()))
            }
        );
        assert_eq!(messages, ["7"]);
    }
    assert_eq!(
        *phases.lock().unwrap(),
        [EventPhase::Capture, EventPhase::Target, EventPhase::Bubble]
    );
    assert_eq!(store.get::<usize>(WidgetId::root()), Some(&42));
}

#[test]
#[should_panic(expected = "Duplicate subscription key")]
fn duplicate_subscription_ids_are_diagnosed() {
    let mut app = App::new(())
        .with_executor(ManualExecutor::default())
        .subscriptions(|_| {
            Subscription::batch([
                Subscription::run("same", |_| TaskOutcome::Complete),
                Subscription::run("same", |_| TaskOutcome::Complete),
            ])
        })
        .build(|_, _: ()| {}, |_| label("test"));
    app.step(&[]).unwrap();
}

#[test]
fn removing_only_a_subscription_rejects_its_already_completed_result() {
    #[derive(Clone, Debug)]
    enum Message {
        Stop,
        Result,
    }
    let executor = ManualExecutor::default();
    let mut app = App::new((true, 0))
        .with_executor(executor.clone())
        .subscriptions(|s| {
            if s.0 {
                Subscription::run("events", |ctx| {
                    ctx.emit(Message::Result);
                    TaskOutcome::Complete
                })
                .scope("editor")
            } else {
                Subscription::none()
            }
        })
        .build(
            |s, msg| match msg {
                Message::Stop => s.0 = false,
                Message::Result => s.1 += 1,
            },
            |_| label("still mounted").scope("editor"),
        );
    app.step(&[]).unwrap();
    app.handle().send(Message::Stop).unwrap();
    executor.run_next();
    app.step(&[]).unwrap();
    assert_eq!(app.state().1, 0);
}

#[test]
fn replacing_a_scope_owner_also_restarts_nested_subscriptions() {
    use render::chunk::Chunk;
    struct Owner<const KIND: u8> {
        children: Vec<Box<dyn Widget<Message>>>,
    }
    #[derive(Clone, Debug)]
    enum Message {
        Replace,
        Root,
        Nested,
    }
    impl<const KIND: u8> Widget<Message> for Owner<KIND> {
        fn render(&self, _: &mut Chunk, _: &RenderCtx) {}
        fn constraints(&self) -> Constraints {
            Constraints::content()
        }
        fn children(&self) -> &[Box<dyn Widget<Message>>] {
            &self.children
        }
        fn key(&self) -> Option<&str> {
            Some("owner")
        }
        fn resource_scope(&self) -> Option<&str> {
            Some("outer")
        }
    }
    let executor = ManualExecutor::default();
    let mut app = App::new((false, 0, 0))
        .with_executor(executor.clone())
        .subscriptions(|_| {
            Subscription::batch([
                Subscription::run("source", |ctx| {
                    ctx.emit(Message::Root);
                    TaskOutcome::Complete
                }),
                Subscription::run("source", |ctx| {
                    ctx.emit(Message::Nested);
                    TaskOutcome::Complete
                })
                .scope("inner")
                .scope("outer"),
            ])
        })
        .build(
            |s, msg| match msg {
                Message::Replace => s.0 = true,
                Message::Root => s.1 += 1,
                Message::Nested => s.2 += 1,
            },
            |s| -> Box<dyn Widget<Message>> {
                let children = vec![label("inner").scope("inner").into_widget()];
                if s.0 {
                    Box::new(Owner::<2> { children })
                } else {
                    Box::new(Owner::<1> { children })
                }
            },
        );
    app.step(&[]).unwrap();
    app.handle().send(Message::Replace).unwrap();
    app.step(&[]).unwrap();
    assert_eq!(executor.pending(), 3);
    assert_eq!(
        app.tasks()
            .iter()
            .filter(|t| t.state == TaskState::Cancelled)
            .count(),
        1
    );
    while executor.run_next() {}
    app.step(&[]).unwrap();
    assert_eq!((app.state().1, app.state().2), (1, 1));
}

#[test]
fn mapped_retained_children_keep_state_without_focus_and_unmount_cancels_exit_resources() {
    let clock = ManualClock::default();
    let executor = ManualExecutor::default();
    let mut app = App::new((true, true))
        .with_clock(clock.clone())
        .with_executor(executor.clone())
        .subscriptions(|_| Subscription::run("watch", |_| TaskOutcome::Complete).scope("editor"))
        .build(
            |s, msg| *s = msg,
            |s| {
                let mut root = col();
                if s.0 {
                    root = root.child(
                        animate(
                            collapsible::<()>("Editor")
                                .expanded(s.1)
                                .retain_state(true)
                                .child(text_input().key("input").value("retained")),
                        )
                        .key("exit")
                        .exit(Transition::fade_out())
                        .scope("editor")
                        .map(|_| (true, true)),
                    );
                }
                root
            },
        );
    app.step(&[]).unwrap();
    let id = WidgetId::root()
        .child("editor")
        .child("exit")
        .child("input");
    assert!(app.widget_state::<TextInputState>(&id).is_some());
    app.handle().send((true, false)).unwrap();
    app.step(&[]).unwrap();
    assert!(app.widget_state::<TextInputState>(&id).is_some());
    assert_ne!(app.focused_key(), Some("input"));
    assert_eq!(executor.pending(), 1);
    app.handle().send((false, false)).unwrap();
    app.step(&[]).unwrap();
    assert!(app.widget_state::<TextInputState>(&id).is_none());
    assert_eq!(app.tasks()[0].state, TaskState::Cancelled);
    assert!(app.next_update_at().is_some());
    clock.advance(Duration::from_secs(2));
    app.step(&[]).unwrap();
    app.step(&[]).unwrap(); // clear the exit renderer's final cleanup frame
    assert_eq!(app.next_update_at(), None);
}
