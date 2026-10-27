//! Real runtime tests: hover follows displayed hit geometry, independently of focus.
use std::time::Duration;
use tui::prelude::*;
use tui::runtime::ManualClock;

fn mouse(kind: MouseEventKind, x: u16, y: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: KeyModifiers::empty(),
    })
}
fn moved(x: u16, y: u16) -> Event {
    mouse(MouseEventKind::Moved, x, y)
}
fn path(key: &str) -> WidgetPath {
    WidgetPath::root().child(key)
}
fn fg<S, M: Send + 'static>(rt: &Runtime<S, M>, x: u16, y: u16) -> Option<crossterm::style::Color> {
    rt.frame().content()[usize::from(y) * usize::from(rt.frame().size().width) + usize::from(x)]
        .content
        .style
        .colors
        .and_then(|c| c.foreground)
}
fn green() -> Style {
    Style::default().fg(Color::Rgb(30, 220, 120))
}

#[test]
fn panel_hover_eases_over_children_and_leaves_without_moving_focus_or_emitting_actions() {
    let clock = ManualClock::default();
    let theme = Theme::dark();
    let base = theme.styles.border.fg_color.map(Into::into);
    let mut rt = App::new(0)
        .with_clock(clock.clone())
        .with_theme(theme)
        .update(|count, (), _| *count += 1)
        .view(|_| {
            grid()
                .columns("20 20")
                .rows("7")
                .child(
                    panel()
                        .key("card")
                        .padding(Padding::uniform(0))
                        .hover_border_style(green())
                        .child(label("Child text")),
                )
                .child(button("Hold focus").key("anchor").on_click(|| ()))
        })
        .build()
        .size(40, 10);
    rt.step(&[]).unwrap();
    assert_eq!(fg(&rt, 0, 0), base);
    assert_eq!(rt.focused_key(), Some("anchor"));
    rt.step(&[moved(3, 1)]).unwrap();
    assert_eq!(rt.hovered_path(), Some(&path("card").child(0)));
    assert_eq!(rt.focused_key(), Some("anchor"));
    clock.advance(Duration::from_millis(60));
    rt.step(&[]).unwrap();
    let middle = fg(&rt, 0, 0);
    assert_ne!(middle, base);
    assert_ne!(middle, Some(Color::Rgb(30, 220, 120).into()));
    clock.advance(Duration::from_millis(60));
    rt.step(&[]).unwrap();
    assert_eq!(fg(&rt, 0, 0), Some(Color::Rgb(30, 220, 120).into()));
    rt.step(&[moved(39, 9)]).unwrap();
    clock.advance(Duration::from_millis(120));
    rt.step(&[]).unwrap();
    assert_eq!(fg(&rt, 0, 0), base);
    assert_eq!(*rt.state(), 0);
    clock.advance(Duration::from_millis(20));
    rt.step(&[]).unwrap();
    assert_eq!(
        rt.next_update_at(),
        None,
        "Settled hover must not keep rendering"
    );
}

#[test]
fn buttons_use_hover_roles_and_disabled_buttons_keep_their_style() {
    let mut theme = Theme::dark();
    theme.styles.primary_action_hover = Style::default().bg(Color::Rgb(17, 31, 67));
    let mut rt = App::new(0)
        .with_theme(theme)
        .with_motion_policy(MotionPolicy::disabled())
        .update(|count, (), _| *count += 1)
        .view(|_| {
            grid()
                .columns("20 20")
                .rows("3 3")
                .child(button("Anchor").key("anchor"))
                .child(button("Hover").key("button").on_click(|| ()))
                .child(button("Disabled").key("disabled").disabled(true))
                .child(button("Ghost").key("ghost").variant(ButtonVariant::Ghost))
        })
        .build()
        .size(40, 10);
    rt.step(&[]).unwrap();
    let button = rt.bounds(&path("button")).unwrap();
    rt.step(&[moved(button.x(), button.y())]).unwrap();
    let cell = &rt.frame().content()[usize::from(button.y()) * 40 + usize::from(button.x())];
    assert_eq!(
        cell.content.style.colors.unwrap().background,
        Some(Color::Rgb(17, 31, 67).into())
    );
    assert_eq!(rt.focused_key(), Some("anchor"));
    assert_eq!(*rt.state(), 0);
    let disabled = rt.bounds(&path("disabled")).unwrap();
    let before =
        rt.frame().content()[usize::from(disabled.y()) * 40 + usize::from(disabled.x())].clone();
    rt.step(&[moved(disabled.x(), disabled.y())]).unwrap();
    assert_eq!(
        before,
        rt.frame().content()[usize::from(disabled.y()) * 40 + usize::from(disabled.x())]
    );
    let ghost = rt.bounds(&path("ghost")).unwrap();
    rt.step(&[moved(ghost.x(), ghost.y())]).unwrap();
    assert_eq!(
        fg(&rt, ghost.x(), ghost.y()),
        Theme::dark().styles.border_focused.fg_color.map(Into::into)
    );
}

#[test]
fn pointer_capture_does_not_capture_hover() {
    let mut rt = App::new(())
        .with_motion_policy(MotionPolicy::disabled())
        .update(|_, (), _| {})
        .view(|_| {
            grid()
                .columns("20 20")
                .rows("3")
                .child(button("A").key("a").variant(ButtonVariant::Ghost))
                .child(button("B").key("b").variant(ButtonVariant::Ghost))
        })
        .build()
        .size(40, 8);
    rt.step(&[]).unwrap();
    let a = rt.bounds(&path("a")).unwrap();
    let b = rt.bounds(&path("b")).unwrap();
    rt.step(&[mouse(MouseEventKind::Down(MouseButton::Left), a.x(), a.y())])
        .unwrap();
    rt.step(&[mouse(MouseEventKind::Drag(MouseButton::Left), b.x(), b.y())])
        .unwrap();
    assert_eq!(rt.focused_key(), Some("a"));
    assert_eq!(rt.hovered_path(), Some(&path("b")));
    rt.shutdown();
    assert!(rt.hovered_path().is_none());
}

#[test]
fn overlapping_layers_and_modals_only_hover_the_visible_target() {
    let mut rt = App::new(false)
        .with_motion_policy(MotionPolicy::disabled())
        .on_key(KeyCode::F(1), || ())
        .update(|modal, (), _| *modal = !*modal)
        .view(|modal| {
            let base = panel()
                .key("lower")
                .padding(Padding::uniform(0))
                .hover_border_style(green())
                .child(label("Lower"));
            let mut root = portal(
                overlay(base).layer(
                    OverlayLayer::new(
                        panel()
                            .key("upper")
                            .hover_border_style(green())
                            .child(label("Upper")),
                    )
                    .floating(OverlayAnchor::TopLeft)
                    .offset(5, 2)
                    .size(20, 7),
                ),
            );
            if *modal {
                root = root.layer(
                    PortalLayer::new(dialog().key("modal").child(button("Close")))
                        .key("modal-layer")
                        .modal(true)
                        .size(20, 7),
                );
            }
            root
        })
        .build()
        .size(40, 12);
    rt.step(&[]).unwrap();
    let base = fg(&rt, 0, 0);
    rt.step(&[moved(7, 4)]).unwrap();
    assert!(rt
        .hovered_path()
        .unwrap()
        .as_slice()
        .contains(&WidgetKey::Named("upper".into())));
    assert_eq!(fg(&rt, 0, 0), base, "Covered siblings must not highlight");
    rt.step(&[moved(0, 0)]).unwrap();
    assert_eq!(fg(&rt, 0, 0), Some(Color::Rgb(30, 220, 120).into()));
    rt.step(&[Event::Key(KeyEvent::new(
        KeyCode::F(1),
        KeyModifiers::empty(),
    ))])
    .unwrap();
    rt.step(&[]).unwrap();
    assert!(
        rt.hovered_path()
            .is_none_or(|path| path.starts_with(&self::path("modal-layer"))),
        "A modal blocks hover on its background: {:?}",
        rt.hovered_path()
    );
    assert_eq!(fg(&rt, 0, 0), base);
    rt.step(&[moved(12, 4)]).unwrap();
    assert!(rt.hovered_path().unwrap().starts_with(&path("modal-layer")));
}

#[test]
fn stationary_pointer_retargets_after_scroll_and_ignores_offscreen_content() {
    let mut rt = App::new(())
        .with_motion_policy(MotionPolicy::disabled())
        .update(|_, (), _| {})
        .view(|_| {
            scroll_view(
                grid()
                    .columns("1fr")
                    .rows("6 6")
                    .child(
                        panel()
                            .key("a")
                            .hover_border_style(green())
                            .child(label("A")),
                    )
                    .child(
                        panel()
                            .key("b")
                            .hover_border_style(green())
                            .child(label("B")),
                    ),
            )
            .key("viewport")
            .vertical()
            .content_height(12)
            .scroll_step(6)
            .scrollbars(ScrollbarVisibility::Hidden)
        })
        .build()
        .size(30, 6);
    rt.step(&[]).unwrap();
    rt.step(&[moved(2, 2)]).unwrap();
    let has_key = |rt: &Runtime<(), ()>, name: &str| {
        rt.hovered_path()
            .unwrap()
            .as_slice()
            .iter()
            .any(|k| *k == WidgetKey::Named(name.into()))
    };
    assert!(has_key(&rt, "a"));
    rt.step(&[mouse(MouseEventKind::ScrollDown, 2, 2)]).unwrap();
    rt.step(&[]).unwrap();
    assert!(
        has_key(&rt, "b"),
        "Hover must follow the newly visible content: {:?}, {:?}",
        rt.hovered_path(),
        rt.widget_state::<ScrollState>(&WidgetId::root())
    );
    rt.step(&[moved(2, 8)]).unwrap();
    assert!(rt.hovered_path().is_none());
}

#[test]
fn inputs_highlight_without_focus_and_hover_clears_on_focus_loss_and_resize() {
    let mut rt = App::new(())
        .with_motion_policy(MotionPolicy::disabled())
        .update(|_, (), _| {})
        .view(|_| {
            grid()
                .columns("30 30")
                .rows("3 5")
                .child(button("Anchor").key("anchor"))
                .child(text_input().key("input"))
                .child(textarea().key("area").height(5))
                .child(text_input().key("disabled").disabled(true))
        })
        .build()
        .size(60, 12);
    rt.step(&[]).unwrap();
    for id in ["input", "area"] {
        let bounds = rt.bounds(&path(id)).unwrap();
        let base = fg(&rt, bounds.x(), bounds.y());
        rt.step(&[moved(bounds.x(), bounds.y())]).unwrap();
        assert_eq!(rt.focused_key(), Some("anchor"));
        assert_ne!(fg(&rt, bounds.x(), bounds.y()), base);
        rt.step(&[Event::FocusLost]).unwrap();
        assert!(rt.hovered_path().is_none());
        assert_eq!(fg(&rt, bounds.x(), bounds.y()), base);
    }
    rt.step(&[moved(31, 0)]).unwrap();
    assert!(rt.hovered_path().is_some());
    rt.step(&[Event::Resize(50, 10)]).unwrap();
    assert!(rt.hovered_path().is_none());
}
