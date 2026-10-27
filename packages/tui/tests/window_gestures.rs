use std::time::Duration;
use tui::prelude::*;
use tui::runtime::ManualClock;
use tui::widgets::TextInputState;
fn mouse(kind: MouseEventKind, x: u16, y: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: KeyModifiers::empty(),
    })
}
fn down(x: u16, y: u16) -> Event {
    mouse(MouseEventKind::Down(MouseButton::Left), x, y)
}
fn drag(x: u16, y: u16) -> Event {
    mouse(MouseEventKind::Drag(MouseButton::Left), x, y)
}
fn up(x: u16, y: u16) -> Event {
    mouse(MouseEventKind::Up(MouseButton::Left), x, y)
}
fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
}
#[derive(Debug, Clone)]
enum Msg {
    Gesture(DragEvent),
    Disabled(bool),
    Hidden(bool),
    Swap,
    Replace,
    Modal(bool),
    Rect(usize, WindowRect),
    Visibility(usize, WindowVisibility),
    Active(Option<String>),
    Text(String),
    Reset(WindowRect),
    Refuse,
    Close,
    Click,
}
#[derive(Default)]
struct State {
    events: Vec<DragEvent>,
    disabled: bool,
    hidden: bool,
    swap: bool,
    replace: bool,
    modal: bool,
    rects: [WindowRect; 2],
    visibility: [WindowVisibility; 2],
    active: Option<String>,
    text: String,
    refuse: bool,
    closed: bool,
    clicks: usize,
}
fn update(s: &mut State, m: Msg) {
    match m {
        Msg::Gesture(e) => s.events.push(e),
        Msg::Disabled(v) => s.disabled = v,
        Msg::Hidden(v) => s.hidden = v,
        Msg::Swap => s.swap = !s.swap,
        Msg::Replace => s.replace = !s.replace,
        Msg::Modal(v) => s.modal = v,
        Msg::Rect(i, r) => {
            if !s.refuse {
                s.rects[i] = r;
            }
        }
        Msg::Visibility(i, v) => s.visibility[i] = v,
        Msg::Active(v) => s.active = v,
        Msg::Text(v) => s.text = v,
        Msg::Reset(r) => s.rects[0] = r,
        Msg::Refuse => s.refuse = true,
        Msg::Close => s.closed = true,
        Msg::Click => s.clicks += 1,
    }
}
fn send(rt: &mut Runtime<State, Msg>, m: Msg) {
    rt.handle().send(m).unwrap();
    rt.step(&[]).unwrap();
}
fn focus(rt: &mut Runtime<State, Msg>, target: &str) {
    for _ in 0..30 {
        if rt.focused_key() == Some(target) {
            return;
        }
        rt.step(&[key(KeyCode::Tab)]).unwrap();
    }
    panic!("cannot focus {target}, current {:?}", rt.focused_key());
}
fn drag_app() -> Runtime<State, Msg> {
    App::new(State::default())
        .disable_quit_key()
        .with_motion_policy(MotionPolicy::disabled())
        .build(update, |s| {
            let handle: Box<dyn Widget<Msg>> = if s.replace {
                Box::new(label("replacement").key("drag"))
            } else {
                Box::new(
                    drag_region(label("ordinary content"))
                        .key("drag")
                        .disabled(s.disabled)
                        .on_drag(Msg::Gesture)
                        .scope("scope")
                        .map(|m| m),
                )
            };
            let base = collapsible("Retained")
                .expanded(!s.hidden)
                .retain_state(true)
                .child(handle);
            let mut root = portal(base);
            if s.modal {
                root = root.layer(
                    PortalLayer::new(button("Modal"))
                        .modal(true)
                        .key("modal")
                        .size(12, 3),
                );
            }
            root
        })
        .size(40, 12)
}
#[test]
fn ordinary_drag_fixed_basis_threshold_capture_and_terminal_events() {
    let mut rt = drag_app();
    rt.step(&[]).unwrap();
    rt.step(&[down(8, 1), drag(7, 0), drag(5, 0), up(5, 0), drag(20, 8)])
        .unwrap();
    let e = &rt.state().events;
    assert!(matches!(e.first(), Some(DragEvent::Begin(_))));
    assert!(matches!(e.last(),Some(DragEvent::End(u)) if u.total_delta==CellPoint::new(-3,-1)));
    assert_eq!(
        e.iter()
            .filter(|e| matches!(e, DragEvent::Begin(_)))
            .count(),
        1
    );
    assert_eq!(
        e.iter().filter(|e| matches!(e, DragEvent::End(_))).count(),
        1
    );
    assert!(!e.iter().any(|e| matches!(e, DragEvent::Cancel(_))));
}
#[test]
fn drag_cancel_reasons_and_pending_cancellation_are_distinct_from_end() {
    for (message, reason) in [
        (Msg::Disabled(true), DragCancelReason::Disabled),
        (Msg::Hidden(true), DragCancelReason::Hidden),
        (Msg::Modal(true), DragCancelReason::ModalChanged),
    ] {
        let mut rt = drag_app();
        rt.step(&[down(8, 1), drag(9, 1)]).unwrap();
        send(&mut rt, message);
        assert!(
            matches!(rt.state().events.last(),Some(DragEvent::Cancel(r)) if *r==reason),
            "{:?}",
            rt.state().events
        );
        let count = rt.state().events.len();
        rt.step(&[drag(11, 1), up(11, 1)]).unwrap();
        assert_eq!(count, rt.state().events.len());
    }
    for event in [key(KeyCode::Esc), Event::FocusLost, Event::Resize(30, 10)] {
        let mut rt = drag_app();
        rt.step(&[down(8, 1), drag(9, 1)]).unwrap();
        rt.step(&[event]).unwrap();
        assert!(matches!(
            rt.state().events.last(),
            Some(DragEvent::Cancel(_))
        ));
    }
    let mut rt = drag_app();
    rt.step(&[down(8, 1), Event::FocusLost, up(8, 1)]).unwrap();
    assert!(rt.state().events.is_empty());
}
#[test]
fn replacement_does_not_inherit_or_deliver_to_old_gesture_and_buttons_keep_clicks() {
    let mut rt = drag_app();
    rt.step(&[down(8, 1), drag(9, 1)]).unwrap();
    send(&mut rt, Msg::Replace);
    send(&mut rt, Msg::Replace);
    let count = rt.state().events.len();
    rt.step(&[drag(15, 1), up(15, 1)]).unwrap();
    assert_eq!(count, rt.state().events.len());
    let mut rt = App::new(State::default())
        .build(update, |_| {
            drag_region(button("click").on_click(|| Msg::Click)).on_drag(Msg::Gesture)
        })
        .size(20, 3);
    rt.step(&[down(3, 1), drag(5, 1), up(5, 1)]).unwrap();
    assert_eq!(rt.state().clicks, 1);
    assert!(rt.state().events.is_empty());
}
fn windows(
    state: State,
    clock: ManualClock,
    motion: bool,
    controlled: bool,
) -> Runtime<State, Msg> {
    App::new(state)
        .disable_quit_key()
        .with_clock(clock)
        .with_motion_policy(if motion {
            MotionPolicy::default()
        } else {
            MotionPolicy::disabled()
        })
        .build(update, move |s| {
            let mut host = window_host(button("Workspace").key("base").on_click(|| Msg::Click))
                .key("host")
                .active_window(s.active.clone())
                .on_active_window_change(Msg::Active);
            let mut ids = vec![0, 1];
            if s.swap {
                ids.reverse();
            }
            for i in ids {
                if i == 0 && s.closed {
                    continue;
                }
                let content: Box<dyn Widget<Msg>> = if i == 0 {
                    Box::new(
                        text_input()
                            .key("editor")
                            .value(&s.text)
                            .on_change(Msg::Text),
                    )
                } else {
                    Box::new(
                        button("Second content")
                            .key("second")
                            .on_click(|| Msg::Click),
                    )
                };
                let mut window = floating_window(content)
                    .key(format!("w{i}"))
                    .title(format!("Window {i}"))
                    .default_rect(WindowRect::new(3 + 20 * i as i32, 2 + 4 * i as i32, 24, 10))
                    .visibility(s.visibility[i])
                    .on_visibility_change(move |v| Msg::Visibility(i, v))
                    .on_rect_change(move |r| Msg::Rect(i, r))
                    .hideable(true);
                if controlled {
                    window = window.rect(s.rects[i]);
                }
                if i == 0 {
                    window = window.on_close(|| Msg::Close);
                }
                host = host.window(window);
            }
            let mut root = portal(host.map(|m| m).scope("desktop"));
            if s.modal {
                root = root.layer(
                    PortalLayer::new(button("Modal"))
                        .modal(true)
                        .size(14, 3)
                        .key("modal"),
                );
            }
            root
        })
        .size(64, 24)
}
fn initial() -> State {
    State {
        rects: [
            WindowRect::new(3, 2, 24, 10),
            WindowRect::new(23, 6, 24, 10),
        ],
        ..State::default()
    }
}
fn title(_rt: &Runtime<State, Msg>, i: usize) -> tui::widget::WidgetPath {
    WidgetPath::root()
        .child("desktop")
        .child("host")
        .child(format!("w{i}"))
        .child("window-title")
}
fn rect_state(rt: &Runtime<State, Msg>, i: usize) -> &FloatingWindowState {
    rt.widget_state(
        &WidgetId::root()
            .child("desktop")
            .child("host")
            .child(format!("w{i}")),
    )
    .unwrap()
}
#[test]
fn windows_capture_move_resize_escape_and_external_override() {
    let mut rt = windows(initial(), ManualClock::default(), false, true);
    rt.step(&[]).unwrap();
    let t = rt.bounds(&title(&rt, 0)).unwrap();
    let (x, y) = (t.x() + 3, t.y());
    rt.step(&[down(x, y), drag(x + 7, y + 2)]).unwrap();
    assert_eq!(rt.state().rects[0], WindowRect::new(10, 4, 24, 10));
    assert_eq!(rt.bounds(&title(&rt, 0)).unwrap().x(), t.x() + 7);
    rt.step(&[key(KeyCode::Esc)]).unwrap();
    assert_eq!(rt.state().rects[0], WindowRect::new(3, 2, 24, 10));
    rt.step(&[drag(40, 20), up(40, 20)]).unwrap();
    assert_eq!(rt.state().rects[0].x, 3);
    rt.step(&[down(x, y), drag(x + 2, y)]).unwrap();
    send(&mut rt, Msg::Reset(WindowRect::new(10, 1, 24, 10)));
    rt.step(&[drag(40, 20), up(40, 20)]).unwrap();
    assert_eq!(rt.state().rects[0], WindowRect::new(10, 1, 24, 10));
    focus(&mut rt, "window-resize");
    let grip = rt.bounds(rt.focused_path().unwrap()).unwrap();
    rt.step(&[
        down(grip.x() + 3, grip.y()),
        drag(grip.x() + 6, grip.y() + 2),
        up(grip.x() + 6, grip.y() + 2),
    ])
    .unwrap();
    assert_eq!(rt.state().rects[0].width, 27);
}
#[test]
fn window_content_click_keeps_cursor_and_retained_state_through_reorder_hide_and_fold() {
    let mut s = initial();
    s.text = "中文abcd".into();
    let mut rt = windows(s, ManualClock::default(), false, true);
    rt.step(&[]).unwrap();
    focus(&mut rt, "editor");
    rt.step(&[key(KeyCode::End), key(KeyCode::Left)]).unwrap();
    let editor_id = WidgetId::root()
        .child("desktop")
        .child("host")
        .child("w0")
        .child("window-content")
        .child("editor");
    let cursor = rt
        .widget_state::<TextInputState>(&editor_id)
        .unwrap()
        .cursor_position;
    send(&mut rt, Msg::Swap);
    assert_eq!(
        rt.widget_state::<TextInputState>(&editor_id)
            .unwrap()
            .cursor_position,
        cursor
    );
    send(&mut rt, Msg::Visibility(0, WindowVisibility::Collapsed));
    assert_ne!(rt.focused_key(), Some("editor"));
    assert!(rt.widget_state::<TextInputState>(&editor_id).is_some());
    send(&mut rt, Msg::Visibility(0, WindowVisibility::Hidden));
    send(&mut rt, Msg::Visibility(0, WindowVisibility::Expanded));
    focus(&mut rt, "editor");
    assert_eq!(
        rt.widget_state::<TextInputState>(&editor_id)
            .unwrap()
            .cursor_position,
        cursor
    );
    let bounds = rt.bounds(rt.focused_path().unwrap()).unwrap();
    let before = rt.state().rects[0];
    rt.step(&[down(bounds.x() + 2, bounds.y() + 1), drag(1, 1), up(1, 1)])
        .unwrap();
    assert_eq!(
        rt.state().rects[0],
        before,
        "editing doesn't move the window"
    );
    assert_eq!(rt.state().active.as_deref(), Some("w0"));
    send(&mut rt, Msg::Close);
    assert!(rt.widget_state::<TextInputState>(&editor_id).is_none());
}
#[test]
fn uncontrolled_windows_keep_geometry_and_tiny_hosts_stay_reachable() {
    let mut rt = windows(initial(), ManualClock::default(), false, false);
    rt.step(&[]).unwrap();
    let t = rt.bounds(&title(&rt, 0)).unwrap();
    rt.step(&[
        down(t.x() + 3, t.y()),
        drag(t.x() + 8, t.y() + 2),
        up(t.x() + 8, t.y() + 2),
    ])
    .unwrap();
    assert_eq!(rect_state(&rt, 0).rect.x, 8);
    send(&mut rt, Msg::Swap);
    assert_eq!(rect_state(&rt, 0).rect.x, 8);
    for (w, h) in [(8, 3), (1, 1), (64, 24)] {
        rt.step(&[Event::Resize(w, h)]).unwrap();
        let area = rt.bounds(&title(&rt, 0));
        if let Some(a) = area {
            assert!(u32::from(a.x()) + u32::from(a.width()) <= u32::from(w));
        }
    }
}
#[test]
fn animation_handoff_uses_display_and_modal_cancels_active_drag() {
    let clock = ManualClock::default();
    let mut rt = windows(initial(), clock.clone(), true, true);
    rt.step(&[]).unwrap();
    send(&mut rt, Msg::Reset(WindowRect::new(30, 2, 24, 10)));
    clock.advance(Duration::from_millis(80));
    rt.step(&[]).unwrap();
    let t = rt.bounds(&title(&rt, 0)).unwrap();
    assert!(t.x() < 31);
    rt.step(&[down(t.x() + 3, t.y()), drag(t.x() + 5, t.y() + 1)])
        .unwrap();
    assert_eq!(rt.state().rects[0].x, i32::from(t.x() - 1) + 2);
    assert_eq!(rt.bounds(&title(&rt, 0)).unwrap().x(), t.x() + 2);
    send(&mut rt, Msg::Modal(true));
    let before = rt.state().rects[0];
    rt.step(&[drag(1, 1), up(1, 1)]).unwrap();
    assert_eq!(rt.state().rects[0], before);
    send(&mut rt, Msg::Modal(false));
    rt.step(&[drag(50, 20)]).unwrap();
    assert_eq!(rt.state().rects[0], before);
}
#[test]
fn rejected_controlled_window_move_restores_parent_value() {
    let mut rt = windows(initial(), ManualClock::default(), false, true);
    rt.step(&[]).unwrap();
    send(&mut rt, Msg::Refuse);
    let t = rt.bounds(&title(&rt, 0)).unwrap();
    rt.step(&[down(t.x() + 2, t.y()), drag(t.x() + 8, t.y())])
        .unwrap();
    assert_eq!(rect_state(&rt, 0).rect, rt.state().rects[0]);
    assert_eq!(rt.bounds(&title(&rt, 0)).unwrap().x(), 4);
}

#[test]
fn moving_drag_handle_and_keyed_reorder_keep_the_original_baseline() {
    let mut rt = App::new(State::default())
        .with_motion_policy(MotionPolicy::disabled())
        .build(
            |s: &mut State, m| {
                if let Msg::Gesture(DragEvent::Update(u)) = &m {
                    s.rects[0].x = u.total_delta.x;
                    s.rects[0].y = u.total_delta.y;
                }
                update(s, m);
            },
            |s| {
                let handle = OverlayLayer::new(
                    drag_region(label("move me"))
                        .key("moving")
                        .on_drag(Msg::Gesture),
                )
                .floating(OverlayAnchor::TopLeft)
                .offset((3 + s.rects[0].x) as i16, (3 + s.rects[0].y) as i16)
                .size(12, 2);
                let other = OverlayLayer::new(label("other").key("other"))
                    .floating(OverlayAnchor::BottomRight)
                    .size(10, 1);
                if s.swap {
                    overlay(label("base")).layer(other).layer(handle)
                } else {
                    overlay(label("base")).layer(handle).layer(other)
                }
            },
        )
        .size(50, 20);
    rt.step(&[down(8, 3), drag(11, 5)]).unwrap();
    assert_eq!((rt.state().rects[0].x, rt.state().rects[0].y), (3, 2));
    send(&mut rt, Msg::Swap);
    rt.step(&[drag(14, 7), up(14, 7)]).unwrap();
    assert_eq!((rt.state().rects[0].x, rt.state().rects[0].y), (6, 4));
    assert_eq!(
        rt.state()
            .events
            .iter()
            .filter(|e| matches!(e, DragEvent::Begin(_)))
            .count(),
        1
    );
    assert!(!rt
        .state()
        .events
        .iter()
        .any(|e| matches!(e, DragEvent::Cancel(_))));
}

#[test]
fn nested_handles_have_one_owner_and_explicit_cancel_is_immediate() {
    struct CancelHandle;
    impl Widget<Msg> for CancelHandle {
        fn constraints(&self) -> Constraints {
            Constraints::min(1, 1)
        }
        fn focus_config(&self) -> FocusConfig {
            FocusConfig::Leaf
        }
        fn render(&self, chunk: &mut render::chunk::Chunk, _: &RenderCtx) {
            let _ = chunk.set_string(0, 0, "Drag, then press c", render::style::Style::default());
        }
        fn handle_event(&self, event: &Event, ctx: &mut EventCtx<Msg>) {
            if ctx.phase() == EventPhase::Bubble
                && matches!(event,Event::Key(k) if k.code==KeyCode::Char('c'))
            {
                ctx.cancel_pointer();
                ctx.set_handled();
            }
        }
    }
    let mut rt = App::new(State::default())
        .build(update, |_| {
            drag_region(
                drag_region(label("inner"))
                    .key("inner")
                    .on_drag(Msg::Gesture),
            )
            .key("outer")
            .on_drag(Msg::Gesture)
        })
        .size(24, 4);
    rt.step(&[down(5, 1), drag(8, 1), up(8, 1)]).unwrap();
    assert_eq!(
        rt.state()
            .events
            .iter()
            .filter(|e| matches!(e, DragEvent::Begin(_)))
            .count(),
        1
    );
    // A custom capture owner may cancel itself independently of drag recognition.
    struct Owner {
        child: Vec<Box<dyn Widget<Msg>>>,
    }
    impl Widget<Msg> for Owner {
        fn constraints(&self) -> Constraints {
            Constraints::min(1, 1)
        }
        fn children(&self) -> &[Box<dyn Widget<Msg>>] {
            &self.child
        }
        fn render(&self, c: &mut render::chunk::Chunk, ctx: &RenderCtx) {
            ctx.render_children(c, self);
        }
        fn handle_event(&self, e: &Event, ctx: &mut EventCtx<Msg>) {
            if ctx.phase() == EventPhase::Capture
                && matches!(e,Event::Key(k) if k.code==KeyCode::Char('c'))
            {
                ctx.cancel_pointer();
                ctx.set_handled();
            }
        }
    }
    let mut rt = App::new(State::default())
        .build(update, |_| Owner {
            child: vec![Box::new(
                drag_region(CancelHandle)
                    .key("handle")
                    .on_drag(Msg::Gesture),
            )],
        })
        .size(24, 4);
    rt.step(&[down(5, 1), drag(8, 1), key(KeyCode::Char('c'))])
        .unwrap();
    assert!(matches!(
        rt.state().events.last(),
        Some(DragEvent::Cancel(DragCancelReason::Explicit))
    ));
}

#[test]
fn title_actions_are_ordinary_controls_and_no_resize_hides_the_grip() {
    let mut rt = App::new(State::default())
        .with_motion_policy(MotionPolicy::disabled())
        .build(update, |_| {
            window_host(label("base")).key("host").window(
                floating_window(label("body"))
                    .key("w")
                    .title("Custom")
                    .default_rect(WindowRect::new(2, 2, 30, 10))
                    .resizable(false)
                    .title_actions(8, button("Action").key("action").on_click(|| Msg::Click)),
            )
        })
        .size(50, 20);
    rt.step(&[]).unwrap();
    focus(&mut rt, "action");
    let a = rt.bounds(rt.focused_path().unwrap()).unwrap();
    rt.step(&[down(a.x() + 1, a.y()), drag(0, 0), up(0, 0)])
        .unwrap();
    assert_eq!(rt.state().clicks, 1);
    let state: &FloatingWindowState = rt.widget_state(&WidgetId::root().child("w")).unwrap();
    assert_eq!(state.rect, WindowRect::new(2, 2, 30, 10));
    for _ in 0..8 {
        rt.step(&[key(KeyCode::Tab)]).unwrap();
        assert_ne!(rt.focused_key(), Some("window-resize"));
    }
}

#[test]
fn hidden_window_keeps_scoped_work_but_close_invalidates_old_results() {
    use tui::runtime::ManualExecutor;
    let executor = ManualExecutor::default();
    let mut rt = App::new(State::default())
        .with_executor(executor.clone())
        .with_motion_policy(MotionPolicy::disabled())
        .subscriptions(|s| {
            if s.closed {
                Subscription::none()
            } else {
                Subscription::run("source", |ctx| {
                    ctx.emit(Msg::Click);
                    TaskOutcome::Complete
                })
                .scope("window-work")
            }
        })
        .build(
            |s, m| {
                if matches!(m, Msg::Replace) {
                    s.closed = false;
                } else {
                    update(s, m);
                }
            },
            |s| {
                let host = window_host(label("base"));
                if s.closed {
                    host
                } else {
                    host.window(
                        floating_window(label("work").scope("window-work"))
                            .key("w")
                            .visibility(if s.hidden {
                                WindowVisibility::Hidden
                            } else {
                                WindowVisibility::Expanded
                            }),
                    )
                }
            },
        )
        .size(40, 15);
    rt.step(&[]).unwrap();
    let task = rt.tasks()[0].id;
    send(&mut rt, Msg::Hidden(true));
    assert_eq!(rt.tasks()[0].id, task);
    assert_eq!(executor.pending(), 1);
    assert!(executor.run_next());
    rt.step(&[]).unwrap();
    assert_eq!(rt.state().clicks, 1, "retained business work continues");
    send(&mut rt, Msg::Close);
    send(&mut rt, Msg::Replace);
    assert_eq!(executor.pending(), 1);
    rt.handle().send(Msg::Close).unwrap();
    assert!(executor.run_next());
    rt.handle().send(Msg::Replace).unwrap();
    rt.step(&[]).unwrap();
    assert_eq!(
        rt.state().clicks,
        1,
        "a remount cannot accept its previous generation's result"
    );
    assert_eq!(executor.pending(), 1);
    assert!(executor.run_next());
    rt.step(&[]).unwrap();
    assert_eq!(rt.state().clicks, 2);
}

#[test]
fn scrolling_ancestor_cancels_the_old_coordinate_basis() {
    let mut rt = App::new(State::default())
        .with_motion_policy(MotionPolicy::disabled())
        .build(update, |_| {
            scroll_view(
                col()
                    .child(
                        drag_region(label("scrolling handle"))
                            .key("handle")
                            .on_drag(Msg::Gesture),
                    )
                    .child(spacer().height(30)),
            )
            .content_height(40)
            .key("scroll")
        })
        .size(30, 8);
    rt.step(&[down(4, 0), drag(5, 0)]).unwrap();
    rt.step(&[mouse(MouseEventKind::ScrollDown, 5, 0)]).unwrap();
    rt.step(&[]).unwrap();
    assert!(matches!(
        rt.state().events.last(),
        Some(DragEvent::Cancel(DragCancelReason::CoordinateSpaceChanged))
    ));
    let count = rt.state().events.len();
    rt.step(&[drag(8, 0), up(8, 0)]).unwrap();
    assert_eq!(count, rt.state().events.len());
}

#[test]
fn title_controls_share_header_style_and_keep_their_click_targets() {
    let content_style = Style::default().bg(Color::Blue).fg(Color::White);
    let header_style = Style::default().bg(Color::Cyan).fg(Color::Black);
    let focused_style = Style::default().bg(Color::Green).fg(Color::Yellow).bold();
    for width_policy in [AmbiguousWidth::Narrow, AmbiguousWidth::Wide] {
        for (active, focused) in [(false, false), (true, false), (false, true)] {
            for visibility in [WindowVisibility::Expanded, WindowVisibility::Collapsed] {
                let mut theme = Theme::dark();
                theme.text_metrics.ambiguous_width = width_policy;
                theme.styles.surface_header = header_style;
                theme.styles.interactive_focused = focused_style;
                let mut rt = App::new(State {
                    active: active.then(|| "window".into()),
                    visibility: [visibility, WindowVisibility::Expanded],
                    ..State::default()
                })
                .with_theme(theme)
                .with_motion_policy(MotionPolicy::disabled())
                .build(update, move |s| {
                    window_host(button("Workspace").key("base"))
                        .active_window(s.active.clone())
                        .window(
                            floating_window(label("Content"))
                                .key("window")
                                .title("Title")
                                .default_rect(WindowRect::new(2, 2, 30, 10))
                                .visibility(s.visibility[0])
                                .on_visibility_change(|v| Msg::Visibility(0, v))
                                .on_close(|| Msg::Close)
                                .hideable(true)
                                .style(content_style),
                        )
                })
                .size(50, 20);
                rt.step(&[]).unwrap();
                if focused {
                    focus(&mut rt, "window-title");
                }
                let title_path = WidgetPath::root().child("window").child("window-title");
                let bounds = rt.bounds(&title_path).unwrap();
                let expected_style = if active || focused {
                    focused_style
                } else {
                    header_style
                }
                .to_render_style();
                let metrics = rt.frame().text_metrics();
                let glyph = |symbol, fallback| {
                    if metrics.width(symbol) == 1 {
                        symbol
                    } else {
                        fallback
                    }
                };
                let collapse = if visibility == WindowVisibility::Collapsed {
                    "+"
                } else {
                    glyph("−", "-")
                };
                let hide = glyph("↓", "v");
                let close = glyph("×", "x");
                let cell = |x| &rt.frame().content()[usize::from(bounds.y()) * 50 + usize::from(x)];
                let actual_header_style = cell(bounds.x() + 1).content.style;
                assert_eq!(actual_header_style.colors, expected_style.colors);
                if expected_style.attr.is_some() {
                    assert_eq!(actual_header_style.attr, expected_style.attr);
                }
                for x in bounds.x()..bounds.x() + bounds.width() {
                    assert_eq!(
                        cell(x).content.style,
                        actual_header_style,
                        "title cell {x} has a mismatched background or attributes"
                    );
                }
                for (distance, symbol) in [(7, collapse), (5, hide), (3, close)] {
                    let x = bounds.x() + bounds.width() - distance;
                    assert_eq!(cell(x).content.symbol(), symbol);
                    assert!(!cell(x).is_occupied, "each control occupies one cell");
                }
                let x = bounds.x() + bounds.width() - 7;
                rt.step(&[down(x, bounds.y()), up(x, bounds.y())]).unwrap();
                assert_ne!(rt.state().visibility[0], visibility);
                send(&mut rt, Msg::Visibility(0, WindowVisibility::Expanded));
                let x = bounds.x() + bounds.width() - 5;
                rt.step(&[down(x, bounds.y()), up(x, bounds.y())]).unwrap();
                assert_eq!(rt.state().visibility[0], WindowVisibility::Hidden);
                send(&mut rt, Msg::Visibility(0, WindowVisibility::Expanded));
                let x = bounds.x() + bounds.width() - 3;
                rt.step(&[down(x, bounds.y()), up(x, bounds.y())]).unwrap();
                assert!(rt.state().closed);
            }
        }
    }
}
