use render::chunk::Chunk;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{cell::Cell, rc::Rc, sync::Arc, time::Duration};
use tui::{prelude::*, runtime::ManualClock};

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
}
fn mouse(kind: MouseEventKind, x: u16, y: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: KeyModifiers::empty(),
    })
}
fn click(x: u16, y: u16) -> Event {
    mouse(MouseEventKind::Down(MouseButton::Left), x, y)
}

struct Logs {
    len: usize,
    start: usize,
    reversed: bool,
    reads: Arc<AtomicUsize>,
}
impl DataSource for Logs {
    type Item = String;
    fn len(&self) -> usize {
        self.len
    }
    fn key(&self, index: usize) -> String {
        (self.start
            + if self.reversed {
                self.len - 1 - index
            } else {
                index
            })
        .to_string()
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        let n = key.parse::<usize>().ok()?.checked_sub(self.start)?;
        (n < self.len).then(|| if self.reversed { self.len - 1 - n } else { n })
    }
    fn get(&self, index: usize) -> RowData<String> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        match index % 3 {
            0 => RowData::Ready(format!("log {}", self.key(index))),
            1 => RowData::Loading,
            _ => RowData::Failed("offline; retry".into()),
        }
    }
}
struct LogsState {
    source: Arc<Logs>,
    window: VirtualListState,
}
enum LogsMsg {
    Window(VirtualListState),
    Source(Arc<Logs>),
}
#[test]
fn virtual_list_ignores_releases_but_preserves_repeats_and_raw_input() {
    #[derive(Default)]
    struct State {
        window: VirtualListState,
        changes: usize,
        releases: usize,
    }
    enum Msg {
        Window(VirtualListState),
        Released,
    }
    let source = Arc::new(Logs {
        len: 100,
        start: 0,
        reversed: false,
        reads: Arc::new(AtomicUsize::new(0)),
    });
    let mut app = App::new(State::default())
        .on_input(|input| {
            matches!(input, InputEvent::Terminal(Event::Key(key)) if key.kind == KeyEventKind::Release)
                .then_some(Msg::Released)
        })
        .update(|state, msg, _| match msg {
            Msg::Window(window) => {
                state.window = window;
                state.changes += 1;
            }
            Msg::Released => state.releases += 1,
        })
        .view(move |state| {
            VirtualList::new(
                source.clone(),
                &state.window,
                VirtualListConfig {
                    height: 3,
                    ..Default::default()
                },
                |_, row, _| label(format!("{row:?}")),
            )
            .on_change(Msg::Window)
        })
        .build()
        .size(20, 3);
    app.step(&[key(KeyCode::Down)]).unwrap();
    assert_eq!(app.state().window.selected.as_deref(), Some("0"));
    app.step(&[Event::Key(KeyEvent::new_with_kind(
        KeyCode::Down,
        KeyModifiers::empty(),
        KeyEventKind::Release,
    ))])
    .unwrap();
    assert_eq!(app.state().releases, 1);
    assert_eq!(app.state().changes, 1);
    assert_eq!(app.state().window.selected.as_deref(), Some("0"));
    app.step(&[Event::Key(KeyEvent::new_with_kind(
        KeyCode::Down,
        KeyModifiers::empty(),
        KeyEventKind::Repeat,
    ))])
    .unwrap();
    assert_eq!(app.state().changes, 2);
    assert_eq!(app.state().window.selected.as_deref(), Some("1"));
}

#[test]
fn hundred_thousand_rows_build_only_window_and_preserve_keys_on_data_changes() {
    let reads = Arc::new(AtomicUsize::new(0));
    let builds = Rc::new(Cell::new(0));
    let b = builds.clone();
    let source = Arc::new(Logs {
        len: 100_000,
        start: 0,
        reversed: false,
        reads: reads.clone(),
    });
    let mut app = App::new(LogsState {
        source,
        window: VirtualListState::default(),
    })
    .build(
        |state, msg| match msg {
            LogsMsg::Window(window) => state.window = window,
            LogsMsg::Source(source) => {
                state.source = source;
                state.window.reconcile(state.source.as_ref(), 8);
            }
        },
        move |state| {
            VirtualList::new(
                state.source.clone(),
                &state.window,
                VirtualListConfig {
                    height: 8,
                    row_height: 1,
                    overscan: 2,
                },
                |_, row, selected| {
                    b.set(b.get() + 1);
                    label(format!("{} {row:?}", if selected { ">" } else { " " }))
                },
            )
            .key("logs")
            .on_change(LogsMsg::Window)
        },
    )
    .size(40, 8);
    app.step(&[]).unwrap();
    assert_eq!(builds.get(), 10);
    assert_eq!(reads.load(Ordering::Relaxed), 10);
    assert_eq!(app.focused_key(), Some("logs"));
    // Loading and failed rows remain keyboard-navigable.
    app.step(&[key(KeyCode::Down), key(KeyCode::Down), key(KeyCode::Down)])
        .unwrap();
    assert_eq!(app.state().window.selected.as_deref(), Some("2"));
    app.step(&[key(KeyCode::End)]).unwrap();
    assert_eq!(app.state().window.selected.as_deref(), Some("99999"));
    assert_eq!(app.state().window.offset, 99_992);
    let anchor = app.state().window.anchor.clone();
    let before = builds.get();
    // Appending does not jump a viewport anchored to an existing item.
    app.handle()
        .send(LogsMsg::Source(Arc::new(Logs {
            len: 100_010,
            start: 0,
            reversed: false,
            reads: reads.clone(),
        })))
        .unwrap();
    app.step(&[]).unwrap();
    assert_eq!(app.state().window.anchor, anchor);
    assert!(builds.get() - before <= 12);
    // Sorting follows the selected key and the first visible item's key.
    app.handle()
        .send(LogsMsg::Source(Arc::new(Logs {
            len: 100_010,
            start: 0,
            reversed: true,
            reads: reads.clone(),
        })))
        .unwrap();
    app.step(&[]).unwrap();
    assert_eq!(app.state().window.selected.as_deref(), Some("99999"));
    assert_eq!(app.state().window.anchor, anchor);
    assert_eq!(app.state().window.offset, 17);
    // Filtering away both keys clears selection and clamps the previous index.
    app.handle()
        .send(LogsMsg::Source(Arc::new(Logs {
            len: 20,
            start: 0,
            reversed: false,
            reads: reads.clone(),
        })))
        .unwrap();
    app.step(&[]).unwrap();
    assert_eq!(app.state().window.selected, None);
    assert_eq!(app.state().window.offset, 12);
    app.step(&[click(1, 2)]).unwrap();
    assert_eq!(app.state().window.selected.as_deref(), Some("14"));
}

struct Row {
    mounts: Rc<Cell<usize>>,
    unmounts: Rc<Cell<usize>>,
}
impl Widget<VirtualListState> for Row {
    fn constraints(&self) -> Constraints {
        Constraints::fixed(10, 1)
    }
    fn lifecycle(&self, ctx: &mut StateCtx) {
        match ctx.lifecycle() {
            Lifecycle::Mount => self.mounts.set(self.mounts.get() + 1),
            Lifecycle::Unmount => self.unmounts.set(self.unmounts.get() + 1),
            _ => {}
        }
    }
    fn render(&self, _: &mut Chunk, _: &RenderCtx) {}
}
#[test]
fn overscan_retention_is_bounded_and_rows_unmount_when_leaving_window() {
    let mounts = Rc::new(Cell::new(0));
    let unmounts = Rc::new(Cell::new(0));
    let (m, u) = (mounts.clone(), unmounts.clone());
    let source = Arc::new(Logs {
        len: 100_000,
        start: 0,
        reversed: false,
        reads: Arc::new(AtomicUsize::new(0)),
    });
    let mut app = App::new(VirtualListState::default())
        .build(
            |state, next| *state = next,
            move |state| {
                VirtualList::new(
                    source.clone(),
                    state,
                    VirtualListConfig {
                        height: 4,
                        row_height: 1,
                        overscan: 1,
                    },
                    |_, _, _| Row {
                        mounts: m.clone(),
                        unmounts: u.clone(),
                    },
                )
                .on_change(|next| next)
            },
        )
        .size(10, 4);
    app.step(&[]).unwrap();
    assert_eq!(mounts.get(), 5);
    app.step(&[mouse(MouseEventKind::ScrollDown, 1, 1)])
        .unwrap();
    assert_eq!(mounts.get() - unmounts.get(), 6);
    assert_eq!(unmounts.get(), 2);
    app.step(&[key(KeyCode::End)]).unwrap();
    assert!(mounts.get() - unmounts.get() <= 6);
    drop(app);
    assert_eq!(mounts.get(), unmounts.get());
}

#[derive(Clone, Copy)]
enum ModalMsg {
    Open,
    Close,
    Background,
    Global,
}
#[test]
fn nested_modals_trap_focus_isolate_background_and_restore_each_level() {
    let mut router = CommandRouter::new();
    router
        .register("global", "Global", || ModalMsg::Global)
        .hotkey(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::empty()));
    let mut app = App::new((0u8, 0u32))
        .with_command_router(router)
        .build(
            |state, msg| match msg {
                ModalMsg::Open => state.0 += 1,
                ModalMsg::Close => state.0 -= 1,
                ModalMsg::Background | ModalMsg::Global => state.1 += 1,
            },
            |state| {
                let mut root = portal(
                    row()
                        .child(button("open").key("open").on_click(|| ModalMsg::Open))
                        .child(
                            button("background")
                                .key("background")
                                .on_click(|| ModalMsg::Background),
                        ),
                );
                if state.0 >= 1 {
                    root = root.layer(
                        PortalLayer::new(button("first").key("first").on_click(|| ModalMsg::Open))
                            .key("one")
                            .modal(true)
                            .size(12, 3)
                            .on_dismiss(|| ModalMsg::Close),
                    );
                }
                if state.0 >= 2 {
                    root = root.layer(
                        PortalLayer::new(button("second").key("second"))
                            .key("two")
                            .modal(true)
                            .size(8, 1)
                            .on_dismiss(|| ModalMsg::Close),
                    );
                }
                root
            },
        )
        .size(40, 10);
    app.step(&[]).unwrap();
    assert_eq!(app.focused_key(), Some("open"));
    app.step(&[key(KeyCode::Enter)]).unwrap();
    assert_eq!(app.focused_key(), Some("first"));
    app.step(&[key(KeyCode::Tab), key(KeyCode::Char('g'))])
        .unwrap();
    assert_eq!(app.focused_key(), Some("first"));
    assert_eq!(app.state().1, 0);
    app.step(&[key(KeyCode::Enter)]).unwrap();
    assert_eq!(app.focused_key(), Some("second"));
    app.step(&[key(KeyCode::Esc)]).unwrap();
    assert_eq!(app.state().0, 1);
    assert_eq!(app.focused_key(), Some("first"));
    // An outside click dismisses the modal without reaching a background button.
    app.step(&[click(1, 0)]).unwrap();
    assert_eq!(app.state().0, 0);
    assert_eq!(app.state().1, 0);
    assert_eq!(app.focused_key(), Some("open"));
    app.step(&[key(KeyCode::Char('g'))]).unwrap();
    assert_eq!(app.state().1, 1);
}

#[test]
fn portal_uses_current_scrolled_anchor_and_escapes_its_container_clip() {
    let mut app = App::new(0u32)
        .build(
            |n, ()| *n += 1,
            |_| {
                portal(
                    row()
                        .child(
                            scroll_view(
                                col()
                                    .child(label("header\nheader\nheader"))
                                    .child(button("anchor").key("anchor")),
                            )
                            .vertical()
                            .content_height(12)
                            .scroll_step(2)
                            .scrollbars(ScrollbarVisibility::Hidden),
                        )
                        .child(label("side")),
                )
                .layer(
                    PortalLayer::new(button("popup").key("action").on_click(|| ()))
                        .key("popup")
                        .anchor(WidgetId::root().child("anchor"))
                        .size(18, 2),
                )
            },
        )
        .size(40, 6);
    app.step(&[]).unwrap();
    let popup = WidgetPath::root().child("popup").child("action");
    let before = app.bounds(&popup).unwrap();
    app.step(&[mouse(MouseEventKind::ScrollDown, 1, 1)])
        .unwrap();
    let anchor = app
        .bounds(
            &WidgetPath::root()
                .child(0)
                .child(0)
                .child(0)
                .child("anchor"),
        )
        .unwrap();
    let after = app.bounds(&popup).unwrap();
    assert!(after.y() < before.y());
    assert_eq!(after.y(), anchor.y() + anchor.height());
    app.step(&[click(after.x(), after.y())]).unwrap();
    assert_eq!(*app.state(), 1);
    app.step(&[Event::Resize(10, 3)]).unwrap();
    let area = app.bounds(&popup).unwrap();
    assert!(area.x() + area.width() <= 10 && area.y() + area.height() <= 3);
}

#[test]
fn commands_share_palette_availability_scope_priority_rebinding_and_sequence_timeout() {
    let mut router = CommandRouter::new();
    let now = std::time::Instant::now();
    let k = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty());
    router.register("global", "Global", || 1).hotkey(k('s'));
    router
        .register("editor", "Save", || 2)
        .scope(WidgetId::root().child("editor"))
        .hotkey(k('s'));
    router
        .register("jump", "Jump", || 3)
        .sequence([k('g'), k('g')]);
    let editor = WidgetId::root().child("editor").child("input");
    assert!(matches!(
        router.handle_key(&k('s'), Some(&editor), now),
        CommandMatch::Message(2)
    ));
    assert!(router.set_enabled("editor", false));
    assert!(
        router
            .palette_items(Some(&editor))
            .iter()
            .find(|item| item.id == "editor")
            .unwrap()
            .disabled
    );
    assert!(matches!(
        router.handle_key(&k('s'), Some(&editor), now),
        CommandMatch::Message(1)
    ));
    assert!(router.rebind("global", vec![vec![k('x')]]));
    assert_eq!(router.dispatch_hotkey(&k('x')), Some(1));
    assert!(matches!(
        router.handle_key(&k('s'), None, now),
        CommandMatch::Ignored
    ));
    assert!(matches!(
        router.handle_key(&k('x'), None, now),
        CommandMatch::Message(1)
    ));
    assert!(matches!(
        router.handle_key(&k('g'), None, now),
        CommandMatch::Pending
    ));
    assert!(matches!(
        router.handle_key(&k('g'), None, now + Duration::from_secs(2)),
        CommandMatch::Pending
    ));
    assert!(matches!(
        router.handle_key(&k('g'), None, now + Duration::from_millis(2100)),
        CommandMatch::Message(3)
    ));
    assert!(matches!(
        router.handle_key(&k('g'), None, now),
        CommandMatch::Pending
    ));
    // A focus change cancels a partially typed chord.
    assert!(matches!(
        router.handle_key(&k('g'), Some(&editor), now),
        CommandMatch::Pending
    ));
}

#[test]
fn runtime_commands_follow_state_and_use_injected_time_for_sequences() {
    let clock = ManualClock::default();
    let mut app = App::new(0)
        .with_clock(clock.clone())
        .with_commands_from(|n| {
            let mut router = CommandRouter::new();
            router
                .register("increment", "Increment", || ())
                .enabled(*n < 1)
                .sequence([KeyEvent::new(KeyCode::Char('g'), KeyModifiers::empty()); 2]);
            router
        })
        .build(|n, ()| *n += 1, |_| label("commands"))
        .size(20, 2);
    app.step(&[]).unwrap();
    app.step(&[key(KeyCode::Char('g'))]).unwrap();
    clock.advance(Duration::from_secs(2));
    app.step(&[key(KeyCode::Char('g'))]).unwrap();
    assert_eq!(*app.state(), 0);
    app.step(&[key(KeyCode::Char('g'))]).unwrap();
    assert_eq!(*app.state(), 1);
    app.step(&[key(KeyCode::Char('g')), key(KeyCode::Char('g'))])
        .unwrap();
    assert_eq!(*app.state(), 1);
}

#[test]
fn canvas_world_transforms_zoom_clipping_and_pointer_hit_use_the_same_coordinates() {
    for encoding in [Encoding::Braille, Encoding::HalfBlock] {
        let viewport = CanvasViewport::new(-10.0, -5.0, 20.0, 10.0);
        let context = CanvasContext::new(20, 10)
            .with_encoding(encoding)
            .with_viewport(viewport);
        let point = CanvasPoint::new(2.0, 1.0);
        let roundtrip = context
            .sample_to_world(context.world_to_sample(point).unwrap())
            .unwrap();
        assert!((roundtrip.x - point.x).abs() < 1e-10 && (roundtrip.y - point.y).abs() < 1e-10);
        let zoomed = CanvasContext::new(20, 10)
            .with_encoding(encoding)
            .with_viewport(viewport.zoom_at(2.0, point));
        assert_eq!(
            context.world_to_sample(point),
            zoomed.world_to_sample(point)
        );
        assert_eq!(viewport.pan(3.0, 4.0).x(), -7.0);
        let mut app = App::new(Vec::new())
            .build(
                |points, p| points.push(p),
                move |_| {
                    canvas(move |surface, ctx| {
                        ctx.world(surface)
                            .line(CanvasPoint::new(-1e6, 0.0), CanvasPoint::new(1e6, 0.0))
                            .rect(CanvasViewport::new(-2.0, -2.0, 4.0, 4.0))
                            .circle(CanvasPoint::new(0.0, 0.0), 2.0)
                            .point(CanvasPoint::new(f64::NAN, 0.0));
                    })
                    .encoding(encoding)
                    .viewport(viewport)
                    .on_pointer(|p| p)
                    .fixed(20, 10)
                },
            )
            .size(20, 10);
        app.step(&[]).unwrap();
        assert!(app
            .frame()
            .content()
            .iter()
            .any(|cell| cell.content.as_char().is_some_and(|c| c != ' ')));
        app.step(&[click(10, 4)]).unwrap();
        let expected = context.cell_to_world(10, 4).unwrap();
        assert_eq!(app.state()[0].position, expected);
        assert!(viewport.contains(expected));
        assert!(CanvasContext::new(0, 0).world_to_sample(point).is_none());
    }
}

#[test]
fn virtual_list_handles_empty_sources_and_short_allocations_without_losing_selection() {
    let reads = Arc::new(AtomicUsize::new(0));
    let source = Arc::new(Logs {
        len: 100,
        start: 0,
        reversed: false,
        reads: reads.clone(),
    });
    let mut app = App::new(VirtualListState::default())
        .build(
            |state, next| *state = next,
            move |state| {
                VirtualList::new(
                    source.clone(),
                    state,
                    VirtualListConfig {
                        height: 8,
                        row_height: 2,
                        overscan: 1,
                    },
                    |_, row, _| label(format!("{row:?}")),
                )
                .key("logs")
                .on_change(|state| state)
            },
        )
        .size(20, 4);
    app.step(&[]).unwrap();
    app.step(&[key(KeyCode::End)]).unwrap();
    assert_eq!(app.state().selected.as_deref(), Some("99"));
    assert_eq!(app.state().offset, 98);
    let selected = WidgetPath::root().child("99");
    let area = app.bounds(&selected).unwrap();
    assert_eq!(area.y(), 2);
    assert_eq!(area.height(), 2);
    let mut state = app.state().clone();
    let empty = Arc::new(Logs {
        len: 0,
        start: 0,
        reversed: false,
        reads,
    });
    state.reconcile(empty.as_ref(), 4);
    assert_eq!(state, VirtualListState::default());
    let mut app = App::new(state)
        .build(
            |state, next| *state = next,
            move |state| {
                VirtualList::new(
                    empty.clone(),
                    state,
                    VirtualListConfig::default(),
                    |_, _, _| label("never"),
                )
                .on_change(|state| state)
            },
        )
        .size(20, 4);
    app.step(&[
        key(KeyCode::Down),
        key(KeyCode::End),
        mouse(MouseEventKind::ScrollDown, 1, 1),
    ])
    .unwrap();
    assert_eq!(app.state(), &VirtualListState::default());
}

#[test]
fn passive_mapped_portal_preserves_background_clicks_and_modal_protocol_for_empty_content() {
    let mut app = App::new(0)
        .build(
            |n, ()| *n += 1,
            |_| {
                portal(button("base").key("base").on_click(|| ()))
                    .layer(PortalLayer::new(label("tooltip")).size(8, 1))
                    .map(|()| ())
            },
        )
        .size(30, 6);
    app.step(&[]).unwrap();
    app.step(&[click(0, 0)]).unwrap();
    assert_eq!(*app.state(), 1);
    let mut app = App::new(0).on_key(KeyCode::Char('g'), || ()).build(
        |n, ()| *n += 1,
        |_| {
            portal(button("base").key("base").on_click(|| ()))
                .layer(
                    PortalLayer::new(label("modal"))
                        .key("modal")
                        .modal(true)
                        .size(8, 1),
                )
                .map(|()| ())
        },
    );
    app = app.size(30, 6);
    app.step(&[]).unwrap();
    assert_eq!(app.focused_key(), None);
    app.step(&[
        key(KeyCode::Tab),
        key(KeyCode::Char('g')),
        key(KeyCode::Esc),
        click(0, 0),
    ])
    .unwrap();
    assert_eq!(*app.state(), 0);
    assert!(!app.step(&[]).unwrap().should_quit);
}

#[test]
fn modal_commands_allow_only_explicit_scopes_within_the_active_modal() {
    let mut commands = CommandRouter::new();
    let k = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty());
    commands.register("outside", "Outside", || 1).hotkey(k);
    commands
        .register("inside", "Inside", || 10)
        .scope(WidgetId::root().child("modal"))
        .hotkey(k);
    let mut app = App::new(0)
        .with_command_router(commands)
        .build(
            |n, msg| *n += msg,
            |_| {
                portal(button("base")).layer(
                    PortalLayer::new(button("child").key("child"))
                        .key("modal")
                        .modal(true),
                )
            },
        )
        .size(20, 6);
    app.step(&[]).unwrap();
    app.step(&[Event::Key(k)]).unwrap();
    assert_eq!(*app.state(), 10);
    let release = KeyEvent::new_with_kind(
        KeyCode::Char('s'),
        KeyModifiers::empty(),
        tui::event::KeyEventKind::Release,
    );
    app.step(&[Event::Key(release)]).unwrap();
    assert_eq!(*app.state(), 10);
}

#[test]
fn portal_flips_above_an_anchor_near_the_bottom_and_tracks_resize_in_the_same_frame() {
    let mut app = App::new(())
        .build(
            |_, ()| {},
            |_| {
                portal(
                    col()
                        .child(label("spacer").height(5))
                        .child(button("anchor").key("anchor")),
                )
                .layer(
                    PortalLayer::new(label("popup").key("popup"))
                        .key("layer")
                        .anchor(WidgetId::root().child("anchor"))
                        .size(10, 3),
                )
            },
        )
        .size(20, 7);
    app.step(&[]).unwrap();
    let area = app
        .bounds(&WidgetPath::root().child("layer").child("popup"))
        .unwrap();
    assert_eq!(area.y(), 2);
    app.step(&[Event::Resize(8, 4)]).unwrap();
    let area = app
        .bounds(&WidgetPath::root().child("layer").child("popup"))
        .unwrap();
    assert!(area.x() + area.width() <= 8 && area.y() + area.height() <= 4);
}

#[test]
fn canvas_pointer_translation_matches_world_coordinates_inside_scrolled_content() {
    let viewport = CanvasViewport::new(-10.0, -10.0, 20.0, 20.0);
    let mut app = App::new(Vec::new())
        .build(
            |points, p| points.push(p),
            move |_| {
                scroll_view(
                    col().child(label("header\nheader")).child(
                        canvas(|_, _| {})
                            .fixed(10, 6)
                            .viewport(viewport)
                            .key("plot")
                            .on_pointer(|p| p),
                    ),
                )
                .vertical()
                .scroll_step(3)
                .scrollbars(ScrollbarVisibility::Hidden)
            },
        )
        .size(10, 4);
    app.step(&[]).unwrap();
    app.step(&[mouse(MouseEventKind::ScrollDown, 1, 0)])
        .unwrap();
    let path = WidgetPath::root().child(0).child("plot");
    let geometry = app.layout_geometry(&path).unwrap();
    app.step(&[click(3, 1)]).unwrap();
    let local_y = (1i32 - geometry.coordinate_offset.1 - i32::from(geometry.display.y())) as u16;
    let expected = CanvasContext::new(10, 6)
        .with_viewport(viewport)
        .cell_to_world(3, local_y)
        .unwrap();
    assert_eq!(app.state().last().unwrap().position, expected);
}

#[test]
fn keyboard_focus_roundtrip_cancels_a_pending_command_sequence() {
    let mut commands = CommandRouter::new();
    commands
        .register("go", "Go", || ())
        .sequence([KeyEvent::new(KeyCode::Char('g'), KeyModifiers::empty()); 2]);
    let mut app = App::new(0)
        .with_command_router(commands)
        .build(
            |n, ()| *n += 1,
            |_| {
                row()
                    .child(button("one").key("one"))
                    .child(button("two").key("two"))
            },
        )
        .size(20, 3);
    app.step(&[
        key(KeyCode::Char('g')),
        key(KeyCode::Tab),
        Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT)),
        key(KeyCode::Char('g')),
    ])
    .unwrap();
    assert_eq!(*app.state(), 0);
    app.step(&[key(KeyCode::Char('g'))]).unwrap();
    assert_eq!(*app.state(), 1);
}

#[test]
fn a_nested_modal_routes_escape_to_its_own_boundary_before_ancestor_dismissal() {
    let mut app = App::new(2u8)
        .build(
            |depth, next| *depth = next,
            |depth| {
                let root = portal(button("background").key("background"));
                if *depth == 0 {
                    return root;
                }
                let mut content = portal(button("outer action").key("outer-action"));
                if *depth == 2 {
                    content = content.layer(
                        PortalLayer::new(button("inner action").key("inner-action"))
                            .key("inner")
                            .modal(true)
                            .size(8, 1)
                            .on_dismiss(|| 1),
                    );
                }
                root.layer(
                    PortalLayer::new(content)
                        .key("outer")
                        .modal(true)
                        .size(16, 5)
                        .on_dismiss(|| 0),
                )
            },
        )
        .size(30, 10);
    app.step(&[]).unwrap();
    assert_eq!(app.focused_key(), Some("inner-action"));
    for kind in [KeyEventKind::Release, KeyEventKind::Repeat] {
        app.step(&[Event::Key(KeyEvent::new_with_kind(
            KeyCode::Esc,
            KeyModifiers::empty(),
            kind,
        ))])
        .unwrap();
        assert_eq!(*app.state(), 2);
        assert_eq!(app.focused_key(), Some("inner-action"));
    }
    app.step(&[key(KeyCode::Esc)]).unwrap();
    assert_eq!(*app.state(), 1);
    assert_eq!(app.focused_key(), Some("outer-action"));
    for kind in [KeyEventKind::Release, KeyEventKind::Repeat] {
        app.step(&[Event::Key(KeyEvent::new_with_kind(
            KeyCode::Esc,
            KeyModifiers::empty(),
            kind,
        ))])
        .unwrap();
        assert_eq!(*app.state(), 1);
        assert_eq!(app.focused_key(), Some("outer-action"));
    }
    app.step(&[key(KeyCode::Esc)]).unwrap();
    assert_eq!(*app.state(), 0);
    assert_eq!(app.focused_key(), Some("background"));
}

#[test]
fn portal_defers_escape_to_child_handlers_and_reuses_measurement_on_paint_only_updates() {
    struct Measured {
        count: Rc<Cell<usize>>,
    }
    impl Widget<()> for Measured {
        fn constraints(&self) -> Constraints {
            Constraints::fixed(5, 1)
        }
        fn measure(&self, _: SizeProposal, _: &MeasureCtx) -> MeasuredSize {
            self.count.set(self.count.get() + 1);
            MeasuredSize::new(5, 1)
        }
        fn render(&self, _: &mut Chunk, _: &RenderCtx) {}
    }
    let count = Rc::new(Cell::new(0));
    let c = count.clone();
    let mut app = App::new(())
        .build(
            |_, ()| {},
            move |_| portal(label("base")).layer(PortalLayer::new(Measured { count: c.clone() })),
        )
        .size(20, 5);
    app.step(&[]).unwrap();
    let measured = count.get();
    app.request_paint();
    app.step(&[]).unwrap();
    assert_eq!(count.get(), measured);
    let mut app = App::new(0)
        .build(
            |n, ()| *n += 1,
            |_| {
                portal(label("base"))
                    .layer(PortalLayer::new(command_palette().on_close(|| ())).modal(true))
            },
        )
        .size(30, 10);
    app.step(&[key(KeyCode::Esc)]).unwrap();
    assert_eq!(*app.state(), 1);
}
