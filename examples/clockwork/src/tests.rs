use super::*;
use crossterm::event::{KeyEvent, MouseEvent};
use machine::{targets, PARTS, PERIOD};

fn press(demo: &mut Demo, key: KeyCode) {
    demo.on_events(&[Event::Key(KeyEvent::new(key, KeyModifiers::NONE))])
        .unwrap();
}

fn error(machine: &Machine) -> f32 {
    machine
        .parts
        .iter()
        .zip(targets(machine.form, machine.time as f32))
        .map(|(p, t)| (p.pose.center - t.center).length())
        .sum::<f32>()
        / PARTS as f32
}

#[test]
fn inventory_assembles_articulates_and_recovers_from_a_pulse() {
    for form in 0..3 {
        let mut machine = Machine::default();
        machine.select(form);
        assert_eq!(targets(form, 0.0).len(), PARTS);
        machine.advance(7.0);
        assert!(
            error(&machine) < 0.27,
            "form {form} failed to dock: {}",
            error(&machine)
        );
        let original: Vec<_> = machine.parts.iter().map(|p| p.pose.center).collect();
        machine.advance(0.5);
        assert!(machine
            .parts
            .iter()
            .zip(original)
            .any(|(p, a)| (p.pose.center - a).length() > 0.12));
        machine.kick();
        machine.advance(0.4);
        assert!(
            error(&machine) > 0.8,
            "pulse should visibly scatter the machine"
        );
        machine.advance(4.0);
        assert!(
            error(&machine) < 0.3,
            "parts should magnetically reassemble"
        );
        assert_eq!(machine.parts.len(), PARTS);
        assert!(machine
            .parts
            .iter()
            .all(|p| p.pose.center.length().is_finite() && p.pose.center.1 >= 0.11));
    }
}

#[test]
fn fixed_step_and_automatic_lifecycle_are_independent_of_frame_rate() {
    let mut a = Machine::default();
    let mut b = Machine::default();
    a.advance(PERIOD * 3.0 + 7.0);
    for _ in 0..((PERIOD * 3.0 + 7.0) * 30.0) as usize {
        b.advance(1.0 / 30.0);
    }
    assert_eq!(a.form, 0);
    assert_eq!(a.form, b.form);
    assert!((a.age - 7.0).abs() < 1e-7);
    for (a, b) in a.parts.iter().zip(&b.parts) {
        assert_eq!(a.pose.center, b.pose.center);
    }
    a.select(2);
    a.advance(PERIOD * 4.0);
    assert_eq!(a.form, 2, "manual selection must hold the requested form");
}

#[test]
fn draw_is_repeatable_at_both_encodings_and_does_not_advance_physics() {
    for form in 0..3 {
        let mut demo = Demo::default();
        demo.machine.select(form);
        demo.advance(7.0);
        for encoding in [Encoding::HalfBlock, Encoding::Braille] {
            demo.encoding = encoding;
            demo.canvas = Canvas::with_encoding(encoding);
            for size in [(0, 0), (1, 1), (2, 3), (37, 15), (100, 36)] {
                let a = export::frame(&mut demo, size.into()).unwrap();
                let b = export::frame(&mut demo, size.into()).unwrap();
                assert_eq!(export::svg(&a), export::svg(&b));
                assert!(a.content().iter().all(|c| !c.content.is_empty()));
                assert!(demo
                    .raster
                    .pixels
                    .iter()
                    .flatten()
                    .all(|c| c.is_finite() && *c >= 0.0 && *c <= 1.0));
            }
        }
    }
}

#[test]
fn pause_controls_mouse_camera_and_exit() {
    let mut demo = Demo {
        clean: true,
        ..Demo::default()
    };
    demo.advance(6.0);
    let before = export::svg(&export::frame(&mut demo, (80, 24).into()).unwrap());
    press(&mut demo, KeyCode::Char(' '));
    demo.advance(5.0);
    assert_eq!(
        before,
        export::svg(&export::frame(&mut demo, (80, 24).into()).unwrap())
    );
    press(&mut demo, KeyCode::Char(' '));
    press(&mut demo, KeyCode::Char('3'));
    assert_eq!(demo.machine.form, 2);
    press(&mut demo, KeyCode::Char('x'));
    assert!(demo.machine.exploded);
    demo.advance(6.0);
    assert!(
        error(&demo.machine) > 0.5,
        "exploded view must separate the physical parts"
    );
    press(&mut demo, KeyCode::Char('x'));
    demo.advance(2.0);
    assert!(
        error(&demo.machine) < 0.3,
        "closing exploded view must reconnect the mechanism"
    );
    press(&mut demo, KeyCode::Char('a'));
    assert!(demo.machine.autoplay);
    let mouse = |kind, column, row| {
        Event::Mouse(MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        })
    };
    demo.on_events(&[
        mouse(MouseEventKind::Down(MouseButton::Left), 20, 10),
        mouse(MouseEventKind::Drag(MouseButton::Left), 30, 12),
    ])
    .unwrap();
    assert!(!demo.orbit);
    assert!(demo.camera.yaw > Camera::default().yaw);
    let yaw = demo.camera.yaw;
    demo.on_events(&[
        Event::FocusLost,
        mouse(MouseEventKind::Drag(MouseButton::Left), 60, 30),
    ])
    .unwrap();
    assert_eq!(
        demo.camera.yaw, yaw,
        "focus loss must release the camera drag"
    );
    press(&mut demo, KeyCode::Char('k'));
    assert!(demo.machine.pulse > 1.0);
    press(&mut demo, KeyCode::Char('q'));
    assert!(demo.should_quit());
}
