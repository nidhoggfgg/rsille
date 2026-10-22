use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tui::prelude::*;
use tui::runtime::ManualClock;
use tui::widgets::TextInputState;

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
}

#[derive(Clone, Debug)]
enum Edit {
    Value(String),
    Swap,
    Rename,
    Multiline,
}

#[derive(Default)]
struct Editor {
    value: String,
    swap: bool,
    rename: bool,
    multiline: bool,
}

fn editor_update(state: &mut Editor, message: Edit) {
    match message {
        Edit::Value(value) => state.value = value,
        Edit::Swap => state.swap = !state.swap,
        Edit::Rename => state.rename = !state.rename,
        Edit::Multiline => state.multiline = !state.multiline,
    }
}

fn editor_view(state: &Editor) -> impl Widget<Edit> {
    let editor: Box<dyn Widget<Edit>> = if state.multiline {
        Box::new(
            textarea()
                .key("editor")
                .value(&state.value)
                .on_change(Edit::Value),
        )
    } else {
        Box::new(
            text_input()
                .key(if state.rename { "renamed" } else { "editor" })
                .value(&state.value)
                .on_change(Edit::Value),
        )
    };
    // A keyed leaf stays stable even when it moves beneath an unkeyed wrapper.
    if state.swap {
        col().child(label("heading")).child(col().child(editor))
    } else {
        col().child(editor)
    }
}

#[test]
fn controlled_input_syncs_without_input_and_repairs_utf8_cursor_after_reordering() {
    let mut app = App::new(Editor::default())
        .with_motion_policy(MotionPolicy::disabled())
        .build(editor_update, editor_view)
        .size(30, 12);
    let id = WidgetId::root().child("editor");
    app.step(&[
        key(KeyCode::Char('a')),
        key(KeyCode::Char('b')),
        key(KeyCode::Char('c')),
    ])
    .unwrap();
    assert_eq!(app.state().value, "abc");
    app.step(&[key(KeyCode::Left), key(KeyCode::Left)]).unwrap();
    assert_eq!(
        app.widget_state::<TextInputState>(&id)
            .unwrap()
            .cursor_position,
        1
    );
    app.handle().send(Edit::Swap).unwrap();
    app.step(&[]).unwrap();
    assert_eq!(app.focused_key(), Some("editor"));
    assert_eq!(
        app.widget_state::<TextInputState>(&id)
            .unwrap()
            .cursor_position,
        1
    );
    // Byte 1 is not a boundary in the new value. This used to panic in render
    // before a keyboard event had a chance to synchronize the external reset.
    app.handle().send(Edit::Value("中文".into())).unwrap();
    app.step(&[]).unwrap();
    let input = app.widget_state::<TextInputState>(&id).unwrap();
    assert_eq!(input.value.as_deref(), Some("中文"));
    assert_eq!(input.cursor_position, 0);
    app.step(&[key(KeyCode::Delete), key(KeyCode::Char('x'))])
        .unwrap();
    assert_eq!(app.state().value, "x文");
    app.handle().send(Edit::Value(String::new())).unwrap();
    app.step(&[]).unwrap();
    let input = app.widget_state::<TextInputState>(&id).unwrap();
    assert_eq!(input.value.as_deref(), Some(""));
    assert_eq!(input.cursor_position, 0);
}

#[test]
fn same_key_changes_component_type_and_new_key_starts_fresh_state() {
    let mut app = App::new(Editor::default())
        .build(editor_update, editor_view)
        .size(30, 12);
    let id = WidgetId::root().child("editor");
    app.step(&[key(KeyCode::Char('a')), key(KeyCode::Char('b'))])
        .unwrap();
    app.handle().send(Edit::Multiline).unwrap();
    app.step(&[]).unwrap();
    assert!(app.widget_state::<TextInputState>(&id).is_none());
    assert_eq!(
        app.widget_state::<TextAreaState>(&id)
            .unwrap()
            .cursor_position,
        0
    );
    app.step(&[
        key(KeyCode::End),
        key(KeyCode::Enter),
        key(KeyCode::Char('中')),
    ])
    .unwrap();
    assert_eq!(app.state().value, "ab\n中");
    app.handle().send(Edit::Multiline).unwrap();
    app.step(&[]).unwrap();
    assert!(app.widget_state::<TextAreaState>(&id).is_none());
    assert_eq!(
        app.widget_state::<TextInputState>(&id)
            .unwrap()
            .cursor_position,
        0
    );
    app.step(&[key(KeyCode::End)]).unwrap();
    app.handle().send(Edit::Rename).unwrap();
    app.step(&[]).unwrap();
    assert!(app.widget_state::<TextInputState>(&id).is_none());
    let id = WidgetId::root().child("renamed");
    assert_eq!(
        app.widget_state::<TextInputState>(&id)
            .unwrap()
            .cursor_position,
        0
    );
}

#[test]
fn textarea_external_reset_repairs_cursor_and_scroll_without_keyboard_input() {
    let mut app = App::new(String::new())
        .build(
            |state, value| *state = value,
            |value| textarea().value(value).height(4).on_change(|value| value),
        )
        .size(20, 4);
    app.step(&[
        key(KeyCode::Char('a')),
        key(KeyCode::Enter),
        key(KeyCode::Char('b')),
        key(KeyCode::Enter),
        key(KeyCode::Char('c')),
    ])
    .unwrap();
    assert!(
        app.widget_state::<TextAreaState>(&WidgetId::root())
            .unwrap()
            .scroll_offset
            > 0
    );
    app.step(&[
        key(KeyCode::Home),
        key(KeyCode::Up),
        key(KeyCode::Up),
        key(KeyCode::Right),
    ])
    .unwrap();
    app.handle().send("中文".into()).unwrap();
    app.step(&[]).unwrap();
    let state = app
        .widget_state::<TextAreaState>(&WidgetId::root())
        .unwrap();
    assert_eq!(state.value.as_deref(), Some("中文"));
    assert_eq!(state.cursor_position, 0);
    assert_eq!(state.scroll_offset, 0);
    app.step(&[key(KeyCode::Delete)]).unwrap();
    assert_eq!(app.state(), "文");
}

#[test]
#[should_panic(expected = "paths WidgetPath[0 → \"duplicate\"] and WidgetPath[1 → \"duplicate\"]")]
fn duplicate_keys_across_unkeyed_wrappers_have_path_diagnostics() {
    let mut app = App::<(), ()>::new(()).build(
        |_, _| (),
        |_| {
            col()
                .child(col().child(text_input().key("duplicate")))
                .child(col().child(textarea().key("duplicate")))
        },
    );
    app.step(&[]).unwrap();
}

#[test]
fn keys_in_distinct_named_scopes_do_not_collide() {
    let mut app = App::<(), ()>::new(()).build(
        |_, _| (),
        |_| {
            col()
                .child(col().key("left").child(text_input().key("editor")))
                .child(col().key("right").child(text_input().key("editor")))
        },
    );
    app.step(&[]).unwrap();
    for scope in ["left", "right"] {
        assert!(app
            .widget_state::<TextInputState>(&WidgetId::root().child(scope).child("editor"))
            .is_some());
    }
}

#[derive(Default)]
struct LocalState {
    value: usize,
    drops: Option<Arc<AtomicUsize>>,
}

impl Drop for LocalState {
    fn drop(&mut self) {
        if let Some(drops) = &self.drops {
            drops.fetch_add(1, Ordering::SeqCst);
        }
    }
}

type LifeLog = Arc<Mutex<Vec<(String, Lifecycle)>>>;
type RenderLog = Arc<Mutex<Vec<(String, usize, bool, bool)>>>;

struct Probe {
    name: &'static str,
    drops: Arc<AtomicUsize>,
    lifecycle: LifeLog,
    renders: RenderLog,
}

impl Widget<Screen> for Probe {
    fn lifecycle(&self, ctx: &mut StateCtx) {
        self.lifecycle
            .lock()
            .unwrap()
            .push((self.name.into(), ctx.lifecycle()));
        if ctx.lifecycle() == Lifecycle::Mount {
            ctx.state_mut::<LocalState>().drops = Some(self.drops.clone());
        }
    }
    fn render(&self, chunk: &mut render::chunk::Chunk, ctx: &RenderCtx) {
        ctx.record_bounds(chunk.area());
        let state = ctx
            .state::<LocalState>()
            .expect("exit rendering must retain state");
        self.renders.lock().unwrap().push((
            self.name.into(),
            state.value,
            ctx.is_exit_render(),
            ctx.is_focused(),
        ));
    }
    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<Screen>) {
        if ctx.phase() == EventPhase::Target && matches!(event, Event::Key(_)) {
            ctx.state_mut::<LocalState>().value += 1;
        }
    }
    fn constraints(&self) -> Constraints {
        Constraints::fixed(10, 2)
    }
    fn focus_config(&self) -> FocusConfig {
        FocusConfig::Leaf
    }
    fn key(&self) -> Option<&str> {
        Some(self.name)
    }
}

#[derive(Clone, Debug)]
struct Screen {
    visible: bool,
    second: bool,
    retain: bool,
    root_key: &'static str,
}

impl Default for Screen {
    fn default() -> Self {
        Self {
            visible: true,
            second: true,
            retain: false,
            root_key: "page",
        }
    }
}

#[derive(Clone, Default)]
struct Observed {
    drops: Arc<AtomicUsize>,
    lifecycle: LifeLog,
    renders: RenderLog,
}

impl Observed {
    fn probe(&self, name: &'static str) -> Probe {
        Probe {
            name,
            drops: self.drops.clone(),
            lifecycle: self.lifecycle.clone(),
            renders: self.renders.clone(),
        }
    }
    fn count(&self, name: &str, lifecycle: Lifecycle) -> usize {
        self.lifecycle
            .lock()
            .unwrap()
            .iter()
            .filter(|(key, phase)| key == name && *phase == lifecycle)
            .count()
    }
    fn drops(&self) -> usize {
        self.drops.load(Ordering::SeqCst)
    }
}

fn exit(duration: u64) -> Transition {
    Transition::new(
        TransitionEffect::Collapse,
        AnimationSpec::new(Duration::from_millis(duration), Easing::Linear),
    )
}

#[test]
fn exit_keeps_state_but_stops_input_and_releases_each_subtree_at_its_deadline() {
    let observed = Observed::default();
    let view_observed = observed.clone();
    let clock = ManualClock::default();
    let mut app = App::new(Screen::default())
        .with_clock(clock.clone())
        .build(
            |state, next| *state = next,
            move |state| {
                let mut view = col();
                if state.visible {
                    view = view.child(
                        animate(view_observed.probe("first"))
                            .key("first-exit")
                            .exit(exit(100)),
                    );
                }
                if state.second {
                    view = view.child(
                        animate(view_observed.probe("second"))
                            .key("second-exit")
                            .exit(exit(200)),
                    );
                }
                view
            },
        )
        .size(30, 10);
    app.step(&[key(KeyCode::Char('x'))]).unwrap();
    app.handle()
        .send(Screen {
            visible: false,
            second: false,
            ..Screen::default()
        })
        .unwrap();
    app.step(&[]).unwrap();
    assert_eq!(observed.count("first", Lifecycle::Unmount), 1);
    assert_eq!(observed.count("second", Lifecycle::Unmount), 1);
    assert_eq!(observed.drops(), 0);
    assert!(app.focused_path().is_none());
    assert!(app
        .widget_state::<LocalState>(&WidgetId::root().child("first-exit").child("first"))
        .is_none());
    app.step(&[key(KeyCode::Char('y'))]).unwrap();
    assert!(observed
        .renders
        .lock()
        .unwrap()
        .iter()
        .any(|(name, value, exiting, focused)| name == "first"
            && *value == 1
            && *exiting
            && !*focused));
    clock.advance(Duration::from_millis(120));
    app.step(&[]).unwrap();
    assert_eq!(observed.drops(), 1);
    clock.advance(Duration::from_millis(100));
    app.step(&[]).unwrap();
    assert_eq!(observed.drops(), 2);
    clock.advance(Duration::from_millis(20));
    assert_eq!(app.step(&[]).unwrap().next_update_at, None);
}

#[test]
fn remount_during_exit_gets_independent_state_and_timeline() {
    let observed = Observed::default();
    let view_observed = observed.clone();
    let clock = ManualClock::default();
    let mut app = App::new(Screen::default()).with_clock(clock.clone()).build(
        |state, next| *state = next,
        move |state| {
            let mut view = col();
            if state.visible {
                view = view.child(
                    animate(view_observed.probe("editor"))
                        .key("exit")
                        .exit(exit(100)),
                );
            }
            view
        },
    );
    app.step(&[key(KeyCode::Char('x'))]).unwrap();
    app.handle()
        .send(Screen {
            visible: false,
            ..Screen::default()
        })
        .unwrap();
    app.step(&[]).unwrap();
    clock.advance(Duration::from_millis(40));
    app.handle().send(Screen::default()).unwrap();
    app.step(&[]).unwrap();
    let id = WidgetId::root().child("exit").child("editor");
    assert_eq!(app.widget_state::<LocalState>(&id).unwrap().value, 0);
    assert_eq!(observed.count("editor", Lifecycle::Mount), 2);
    app.step(&[key(KeyCode::Char('y'))]).unwrap();
    assert_eq!(app.widget_state::<LocalState>(&id).unwrap().value, 1);
    // Removing the second incarnation starts a separate timeline at t=40.
    app.handle()
        .send(Screen {
            visible: false,
            ..Screen::default()
        })
        .unwrap();
    app.step(&[]).unwrap();
    clock.advance(Duration::from_millis(70));
    app.step(&[]).unwrap();
    assert_eq!(observed.drops(), 1);
    clock.advance(Duration::from_millis(40));
    app.step(&[]).unwrap();
    assert_eq!(observed.drops(), 2);
}

#[test]
fn collapsed_children_require_explicit_retention_and_drop_with_the_owner() {
    for retain in [false, true] {
        let observed = Observed::default();
        let view_observed = observed.clone();
        let mut app = App::new(Screen {
            retain,
            ..Screen::default()
        })
        .build(
            |state, next| *state = next,
            move |state| {
                collapsible("page")
                    .key(state.root_key)
                    .expanded(state.visible)
                    .retain_state(state.retain)
                    .child(view_observed.probe("editor"))
            },
        );
        app.step(&[key(KeyCode::Tab), key(KeyCode::Char('x'))])
            .unwrap();
        let id = WidgetId::root().child("editor");
        assert_eq!(app.widget_state::<LocalState>(&id).unwrap().value, 1);
        app.handle()
            .send(Screen {
                visible: false,
                retain,
                ..Screen::default()
            })
            .unwrap();
        app.step(&[]).unwrap();
        assert_eq!(
            observed.count("editor", Lifecycle::Unmount),
            usize::from(!retain)
        );
        assert_eq!(observed.drops(), usize::from(!retain));
        let renders = observed.renders.lock().unwrap().len();
        app.step(&[key(KeyCode::Char('x'))]).unwrap();
        assert_eq!(observed.renders.lock().unwrap().len(), renders);
        if retain {
            assert_eq!(app.widget_state::<LocalState>(&id).unwrap().value, 1);
        } else {
            assert!(app.widget_state::<LocalState>(&id).is_none());
        }
        app.handle()
            .send(Screen {
                retain,
                ..Screen::default()
            })
            .unwrap();
        app.step(&[]).unwrap();
        assert_eq!(
            app.widget_state::<LocalState>(&id).unwrap().value,
            usize::from(retain)
        );
        // Changing the root key ends the lifetime of its entire retained scope.
        app.handle()
            .send(Screen {
                retain,
                root_key: "replacement",
                ..Screen::default()
            })
            .unwrap();
        app.step(&[]).unwrap();
        assert_eq!(app.widget_state::<LocalState>(&id).unwrap().value, 0);
        let unmounts = observed.count("editor", Lifecycle::Unmount);
        app.shutdown();
        assert_eq!(observed.count("editor", Lifecycle::Unmount), unmounts + 1);
        assert!(app.widget_state::<LocalState>(&id).is_none());
        drop(app);
        assert_eq!(observed.count("editor", Lifecycle::Unmount), unmounts + 1);
    }
}

#[test]
fn drop_unmounts_live_components_once() {
    let observed = Observed::default();
    let view_observed = observed.clone();
    let mut app =
        App::new(Screen::default()).build(|_, _: Screen| (), move |_| view_observed.probe("root"));
    app.step(&[]).unwrap();
    assert_eq!(observed.count("root", Lifecycle::Mount), 1);
    drop(app);
    assert_eq!(observed.count("root", Lifecycle::Unmount), 1);
    assert_eq!(observed.drops(), 1);
}

// A different component can use exactly the same state type. Matching must
// compare component TypeIds, rather than waiting for a state downcast to fail.
struct OtherProbe(Probe);

impl Widget<Screen> for OtherProbe {
    fn lifecycle(&self, ctx: &mut StateCtx) {
        self.0.lifecycle(ctx);
    }
    fn render(&self, chunk: &mut render::chunk::Chunk, ctx: &RenderCtx) {
        self.0.render(chunk, ctx);
    }
    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<Screen>) {
        self.0.handle_event(event, ctx);
    }
    fn constraints(&self) -> Constraints {
        self.0.constraints()
    }
    fn focus_config(&self) -> FocusConfig {
        self.0.focus_config()
    }
    fn key(&self) -> Option<&str> {
        self.0.key()
    }
}

#[test]
fn type_changes_reset_state_even_when_state_types_match_and_the_view_is_boxed() {
    let observed = Observed::default();
    let view_observed = observed.clone();
    let mut app = App::new(Screen::default()).build(
        |state, next| *state = next,
        move |state| {
            let child: Box<dyn Widget<Screen>> = if state.visible {
                Box::new(view_observed.probe("editor"))
            } else {
                Box::new(OtherProbe(view_observed.probe("editor")))
            };
            col().child(child)
        },
    );
    app.step(&[key(KeyCode::Char('x'))]).unwrap();
    let id = WidgetId::root().child("editor");
    assert_eq!(app.widget_state::<LocalState>(&id).unwrap().value, 1);
    app.handle()
        .send(Screen {
            visible: false,
            ..Screen::default()
        })
        .unwrap();
    app.step(&[]).unwrap();
    assert_eq!(app.widget_state::<LocalState>(&id).unwrap().value, 0);
    assert_eq!(observed.drops(), 1);
    assert_eq!(observed.count("editor", Lifecycle::Unmount), 1);
    assert_eq!(observed.count("editor", Lifecycle::Mount), 2);
}

#[test]
fn moving_a_live_keyed_child_out_of_an_exiting_wrapper_preserves_its_state() {
    let observed = Observed::default();
    let view_observed = observed.clone();
    let mut app = App::new(Screen::default()).build(
        |state, next| *state = next,
        move |state| {
            if state.visible {
                col().child(animate(view_observed.probe("editor")).exit(exit(100)))
            } else {
                col().child(col().child(view_observed.probe("editor")))
            }
        },
    );
    app.step(&[key(KeyCode::Char('x'))]).unwrap();
    app.handle()
        .send(Screen {
            visible: false,
            ..Screen::default()
        })
        .unwrap();
    app.step(&[]).unwrap();
    assert_eq!(
        app.widget_state::<LocalState>(&WidgetId::root().child("editor"))
            .unwrap()
            .value,
        1
    );
    assert_eq!(observed.count("editor", Lifecycle::Mount), 1);
    assert_eq!(observed.count("editor", Lifecycle::Unmount), 0);
    assert_eq!(observed.drops(), 0);
    assert!(observed
        .renders
        .lock()
        .unwrap()
        .iter()
        .all(|(_, _, exiting, _)| !*exiting));
}

#[test]
#[should_panic(expected = "including retained children")]
fn retained_hidden_children_participate_in_duplicate_key_validation() {
    let mut app = App::<(), ()>::new(()).build(
        |_, _| (),
        |_| {
            col().child(text_input().key("duplicate")).child(
                collapsible("hidden")
                    .retain_state(true)
                    .child(textarea().key("duplicate")),
            )
        },
    );
    app.step(&[]).unwrap();
}

#[test]
fn deterministic_exits_use_animation_time_even_after_the_scheduler_clock_jumps() {
    let observed = Observed::default();
    let view_observed = observed.clone();
    let clock = ManualClock::default();
    let mut app = App::new(Screen::default())
        .with_clock(clock.clone())
        .with_motion_policy(MotionPolicy::deterministic(Duration::from_millis(10)))
        .build(
            |state, next| *state = next,
            move |state| {
                let mut view = col();
                if state.visible {
                    view = view.child(animate(view_observed.probe("editor")).exit(exit(100)));
                }
                view
            },
        );
    app.step(&[]).unwrap();
    clock.advance(Duration::from_secs(10));
    app.handle()
        .send(Screen {
            visible: false,
            ..Screen::default()
        })
        .unwrap();
    app.step(&[]).unwrap();
    assert_eq!(observed.drops(), 0);
    for _ in 0..10 {
        app.step(&[]).unwrap();
    }
    assert_eq!(observed.drops(), 1);
    assert_eq!(app.next_update_at(), None);
}
