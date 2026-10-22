use std::{cell::Cell, rc::Rc, time::Duration};

use render::{area::Area, chunk::Chunk};
use tui::{animation::AnimationCtx, prelude::*};

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

// A real custom container: the framework handles all child identity, clipping,
// animation and input routing. No container-specific focus/runtime code.
struct Inset {
    children: Vec<Box<dyn Widget<()>>>,
    layouts: Rc<Cell<usize>>,
}
impl Widget<()> for Inset {
    fn layout(&self, area: Area, _: &MeasureCtx) -> Layout {
        self.layouts.set(self.layouts.get() + 1);
        Layout::new([ChildLayout::new(0, area.shrink_saturating(1, 1, 2, 2))])
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        ctx.render_children(chunk, self);
    }
    fn constraints(&self) -> Constraints {
        Constraints::content()
    }
    fn children(&self) -> &[Box<dyn Widget<()>>] {
        &self.children
    }
}

#[test]
fn paint_and_cursor_movement_reuse_layout_with_mapped_scoped_custom_containers() {
    let layouts = Rc::new(Cell::new(0));
    let count = layouts.clone();
    let mut app = App::new(0)
        .with_motion_policy(MotionPolicy::disabled())
        .build(
            |n, ()| *n += 1,
            move |_| {
                Inset {
                    children: vec![Box::new(text_input().key("edit").value("中文abc"))],
                    layouts: count.clone(),
                }
                .map(|()| ())
                .scope("editor")
            },
        )
        .size(24, 5);
    app.step(&[]).unwrap();
    assert_eq!(layouts.get(), 1);
    let computed = app.layout_stats().computations;
    let path = WidgetPath::root().child(0).child("edit");
    assert_eq!(
        app.bounds(&path).unwrap(),
        Area::new((2, 1).into(), (20, 3).into())
    );
    assert_eq!(app.focused_key(), Some("edit"));
    app.step(&[key(KeyCode::Right)]).unwrap();
    app.request_paint();
    app.step(&[]).unwrap();
    assert_eq!(layouts.get(), 1);
    assert_eq!(app.layout_stats().computations, computed);
    app.step(&[click(6, 2)]).unwrap();
    let state = app
        .widget_state::<tui::widgets::text_input::TextInputState>(&WidgetId::root().child("edit"))
        .unwrap();
    assert_eq!(state.cursor_position, "中".len());
    app.step(&[Event::Resize(32, 7)]).unwrap();
    assert_eq!(layouts.get(), 2);
    assert_eq!(app.bounds(&path).unwrap().width(), 28);
    app.handle().send(()).unwrap();
    app.step(&[]).unwrap();
    assert_eq!(layouts.get(), 3);
}

#[derive(Default)]
struct LocalSize(u16);
struct StatefulWidth;
impl Widget<()> for StatefulWidth {
    fn lifecycle(&self, ctx: &mut StateCtx) {
        ctx.state_mut::<LocalSize>();
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        ctx.record_bounds(chunk.area());
        let _ = chunk.set_string(0, 0, "size", render::style::Style::default());
    }
    fn measure(&self, _: SizeProposal, ctx: &MeasureCtx) -> MeasuredSize {
        MeasuredSize::new(4 + ctx.state::<LocalSize>().unwrap().0, 1)
    }
    fn constraints(&self) -> Constraints {
        Constraints::content()
    }
    fn focus_config(&self) -> FocusConfig {
        FocusConfig::Leaf
    }
    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<()>) {
        if ctx.phase() == EventPhase::Target && matches!(event, Event::Key(_)) {
            ctx.state_mut::<LocalSize>().0 += 3;
            ctx.request_layout();
            ctx.set_handled();
        }
    }
}

#[test]
fn explicit_widget_layout_invalidation_remeasures_without_rebuilding_view() {
    let views = Rc::new(Cell::new(0));
    let count = views.clone();
    let mut app = App::new(())
        .build(
            |_, ()| {},
            move |_| {
                count.set(count.get() + 1);
                row().child(StatefulWidth).child(button("next").key("next"))
            },
        )
        .size(24, 3);
    app.step(&[]).unwrap();
    let path = WidgetPath::root().child(0);
    let width = app.bounds(&path).unwrap().width();
    let computations = app.layout_stats().computations;
    app.step(&[key(KeyCode::Char('x'))]).unwrap();
    assert_eq!(views.get(), 1);
    assert!(app.layout_stats().computations > computations);
    assert_eq!(app.bounds(&path).unwrap().width(), width + 3);
    let next = app.bounds(&WidgetPath::root().child("next")).unwrap();
    assert_eq!(next.x(), width + 3);
}

struct AnimationRequests {
    mode: Rc<Cell<u8>>,
    measurements: Rc<Cell<u32>>,
}
impl Widget<()> for AnimationRequests {
    fn constraints(&self) -> Constraints {
        Constraints::fixed(4, 1)
    }
    fn measure(&self, _: SizeProposal, _: &MeasureCtx) -> MeasuredSize {
        self.measurements.set(self.measurements.get() + 1);
        MeasuredSize::new(4, 1)
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        let _ = chunk.set_string(0, 0, "tick", ctx.theme().styles.text.to_render_style());
    }
    fn animate(&self, ctx: &mut AnimationCtx) -> bool {
        match self.mode.get() {
            1 => {
                ctx.request_next_frame();
            }
            2 => {
                ctx.request_paint();
            }
            3 => {
                ctx.request_layout();
            }
            4 => return true, // Backwards-compatible paint + another update.
            _ => {}
        }
        false
    }
}

#[test]
fn animation_scheduling_paint_and_layout_requests_are_independent() {
    let clock = ManualClock::default();
    let mode = Rc::new(Cell::new(0));
    let measurements = Rc::new(Cell::new(0));
    let (m, measured) = (mode.clone(), measurements.clone());
    let mut app = App::new(())
        .with_clock(clock.clone())
        .with_theme_from(|_| Theme::dark())
        .build(
            |_, ()| {},
            move |_| {
                col().child(AnimationRequests {
                    mode: m.clone(),
                    measurements: measured.clone(),
                })
            },
        )
        .size(20, 4);
    app.step(&[]).unwrap();
    let count = measurements.get();
    let computations = app.layout_stats().computations;
    mode.set(1);
    let result = app.step(&[]).unwrap();
    assert!(!result.rendered);
    assert!(result.next_update_at.is_some());
    mode.set(2);
    clock.advance(Duration::from_millis(20));
    let result = app.step(&[]).unwrap();
    assert!(result.rendered);
    assert_eq!(result.next_update_at, None);
    assert_eq!(measurements.get(), count);
    assert_eq!(app.layout_stats().computations, computations);
    mode.set(3);
    assert!(app.step(&[]).unwrap().rendered);
    assert!(measurements.get() > count);
    mode.set(4);
    let result = app.step(&[]).unwrap();
    assert!(result.rendered && result.next_update_at.is_some());
    mode.set(0);
    assert_eq!(app.step(&[]).unwrap().next_update_at, None);
}

#[test]
fn scrolled_child_clicks_use_visible_screen_geometry_and_full_local_coordinates() {
    let mut app = App::new(Vec::new())
        .with_motion_policy(MotionPolicy::disabled())
        .build(
            |rows, row| rows.push(row),
            |_| {
                scroll_view(
                    col()
                        .child(label("header").height(1))
                        .child(button("first").key("first").on_click(|| 1))
                        .child(button("second").key("second").on_click(|| 2))
                        .child(label("footer\nfooter")),
                )
                .vertical()
                .scrollbars(ScrollbarVisibility::Hidden)
                .scroll_step(2)
            },
        )
        .size(20, 3);
    app.step(&[]).unwrap();
    app.step(&[mouse(MouseEventKind::ScrollDown, 1, 1)])
        .unwrap();
    let path = WidgetPath::root().child(0).child("second");
    let area = app.bounds(&path).unwrap();
    assert!(area.height() > 0 && area.y() < 3);
    app.step(&[click(area.x() + 1, area.y())]).unwrap();
    assert_eq!(app.state(), &[2]);
    let geometry = app.layout_geometry(&path).unwrap();
    assert_eq!(geometry.coordinate_offset.1, -2);
    assert!(geometry.hit.unwrap().height() <= 3);
}

#[test]
fn horizontal_scroll_preserves_text_input_click_column() {
    let mut app = App::new(())
        .with_motion_policy(MotionPolicy::disabled())
        .build(
            |_, ()| {},
            |_| {
                scroll_view(text_input().key("edit").value("0123456789abcdefghij"))
                    .both()
                    .content_width(24)
                    .content_height(3)
                    .scroll_step(5)
                    .scrollbars(ScrollbarVisibility::Hidden)
            },
        )
        .size(10, 3);
    app.step(&[]).unwrap();
    app.step(&[mouse(MouseEventKind::ScrollRight, 2, 0)])
        .unwrap();
    app.step(&[click(4, 1)]).unwrap();
    let state = app
        .widget_state::<tui::widgets::text_input::TextInputState>(&WidgetId::root().child("edit"))
        .unwrap();
    assert_eq!(state.cursor_position, 7);
}

#[test]
fn resize_rewraps_text_and_moves_following_click_target() {
    let mut app = App::new(0)
        .with_motion_policy(MotionPolicy::disabled())
        .build(
            |n, ()| *n += 1,
            |_| {
                col()
                    .child(label("abcdefghijklmno").wrap_char())
                    .child(button("click").key("click").on_click(|| ()))
            },
        )
        .size(8, 8);
    app.step(&[]).unwrap();
    let path = WidgetPath::root().child("click");
    let before = app.bounds(&path).unwrap();
    app.step(&[Event::Resize(16, 8)]).unwrap();
    let after = app.bounds(&path).unwrap();
    assert!(after.y() < before.y());
    app.step(&[click(after.x(), after.y())]).unwrap();
    assert_eq!(*app.state(), 1);
}

#[test]
fn split_resize_invalidates_allocations_and_hit_targets_together() {
    let mut app = App::new(0)
        .with_motion_policy(MotionPolicy::disabled())
        .build(
            |n, ()| *n += 1,
            |_| split(label("left"), button("right").key("right").on_click(|| ())),
        )
        .size(30, 5);
    app.step(&[]).unwrap();
    let path = WidgetPath::root().child("right");
    let before = app.bounds(&path).unwrap();
    app.step(&[key(KeyCode::Right)]).unwrap();
    let after = app.bounds(&path).unwrap();
    assert!(after.x() > before.x());
    app.step(&[click(after.x(), after.y())]).unwrap();
    assert_eq!(*app.state(), 1);
}

#[test]
fn layout_animation_reports_target_display_clip_and_clicks_at_display_position() {
    let clock = ManualClock::default();
    let mut app = App::new((0u16, 0u32))
        .with_clock(clock.clone())
        .build(
            |state, msg: Option<u16>| match msg {
                Some(x) => state.0 = x,
                None => state.1 += 1,
            },
            |state| {
                row().child(label("").fixed(state.0, 1)).child(
                    animate(button("move").key("button").on_click(|| None))
                        .key("moving")
                        .layout_transition(
                            LayoutTransition::size_and_position(AnimationSpec::new(
                                Duration::from_millis(100),
                                Easing::Linear,
                            ))
                            .hit_test(HitTestMode::Display),
                        ),
                )
            },
        )
        .size(30, 3);
    app.step(&[]).unwrap();
    app.handle().send(Some(10)).unwrap();
    app.step(&[]).unwrap();
    clock.advance(Duration::from_millis(50));
    app.step(&[]).unwrap();
    let geometry = app
        .layout_geometry(&WidgetPath::root().child("moving"))
        .unwrap();
    assert_eq!(geometry.target.x(), 10);
    assert_eq!(geometry.display.x(), 5);
    assert_eq!(geometry.hit.unwrap().x(), 5);
    assert_eq!(geometry.clip.x(), 5);
    app.step(&[click(5, 0)]).unwrap();
    assert_eq!(app.state().1, 1);
}

struct ColorAnimation {
    target: Rc<Cell<bool>>,
    measured: Rc<Cell<u32>>,
}
impl Widget<()> for ColorAnimation {
    fn constraints(&self) -> Constraints {
        Constraints::fixed(4, 1)
    }
    fn measure(&self, _: SizeProposal, _: &MeasureCtx) -> MeasuredSize {
        self.measured.set(self.measured.get() + 1);
        MeasuredSize::new(4, 1)
    }
    fn animate(&self, ctx: &mut AnimationCtx) -> bool {
        ctx.track_style(
            "color",
            Style::default().fg(if self.target.get() {
                Color::Rgb(0, 0, 255)
            } else {
                Color::Rgb(255, 0, 0)
            }),
            AnimationSpec::new(Duration::from_millis(100), Easing::Linear),
        )
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        let _ = chunk.set_string(
            0,
            0,
            "text",
            ctx.animation_style("color")
                .unwrap_or_else(Style::default)
                .to_render_style(),
        );
    }
}

#[test]
fn actual_color_animation_keeps_measurement_and_layout_counts_constant() {
    let clock = ManualClock::default();
    let target = Rc::new(Cell::new(false));
    let measured = Rc::new(Cell::new(0));
    let (t, m) = (target.clone(), measured.clone());
    let mut app = App::new(())
        .with_clock(clock.clone())
        .build(
            |_, ()| {},
            move |_| {
                col().child(ColorAnimation {
                    target: t.clone(),
                    measured: m.clone(),
                })
            },
        )
        .size(20, 4);
    app.step(&[]).unwrap();
    let first = app.frame().content()[0].content.style;
    let count = measured.get();
    let computations = app.layout_stats().computations;
    target.set(true);
    app.step(&[]).unwrap();
    clock.advance(Duration::from_millis(50));
    assert!(app.step(&[]).unwrap().rendered);
    assert_ne!(app.frame().content()[0].content.style, first);
    assert_eq!(measured.get(), count);
    assert_eq!(app.layout_stats().computations, computations);
}

struct Overflowing {
    children: Vec<Box<dyn Widget<()>>>,
}
impl Widget<()> for Overflowing {
    fn constraints(&self) -> Constraints {
        Constraints::fixed(4, 1)
    }
    fn layout(&self, area: Area, _: &MeasureCtx) -> Layout {
        Layout::new([ChildLayout::new(0, Area::new(area.pos(), (20, 1).into()))])
    }
    fn children(&self) -> &[Box<dyn Widget<()>>] {
        &self.children
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        ctx.render_children(chunk, self);
    }
}

#[test]
fn custom_overflow_is_clipped_for_paint_bounds_and_pointer_hits() {
    let mut app = App::new(0)
        .with_motion_policy(MotionPolicy::disabled())
        .build(
            |n, ()| *n += 1,
            |_| {
                col().child(Overflowing {
                    children: vec![Box::new(button("hit").key("hit").on_click(|| ()))],
                })
            },
        )
        .size(20, 3);
    app.step(&[]).unwrap();
    let path = WidgetPath::root().child(0).child("hit");
    let geometry = app.layout_geometry(&path).unwrap();
    assert_eq!(geometry.target.width(), 20);
    assert_eq!(geometry.display.width(), 20);
    assert_eq!(geometry.clip.width(), 4);
    assert_eq!(geometry.hit.unwrap().width(), 4);
    assert_eq!(app.bounds(&path).unwrap().width(), 4);
    app.step(&[click(8, 0)]).unwrap();
    assert_eq!(*app.state(), 0);
    app.step(&[
        mouse(MouseEventKind::Up(MouseButton::Left), 8, 0),
        click(1, 0),
    ])
    .unwrap();
    assert_eq!(*app.state(), 1);
}
