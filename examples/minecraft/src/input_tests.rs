use super::*;
use rsille::tui::runtime::ManualClock;
use std::time::Duration;

fn key(code: KeyCode, kind: KeyEventKind) -> Event {
    Event::Key(KeyEvent::new_with_kind(code, KeyModifiers::empty(), kind))
}

fn game(enhanced: bool) -> (Runtime<World, Msg>, ManualClock) {
    let clock = ManualClock::default();
    let world = World {
        player: [28.5, 20.0, 17.5],
        yaw: 0.0,
        pitch: 0.0,
        flying: true,
        ..World::default()
    };
    let mut runtime = app(world)
        .with_clock(clock.clone())
        .with_motion_policy(MotionPolicy::disabled())
        .update(update)
        .view(view)
        .build()
        .size(20, 8);
    runtime.set_input_capabilities(InputCapabilities {
        key_releases: enhanced,
        focus_events: true,
    });
    runtime.step(&[]).unwrap();
    (runtime, clock)
}

fn frame(runtime: &mut Runtime<World, Msg>, clock: &ManualClock) {
    clock.advance(Duration::from_millis(20));
    runtime.step(&[]).unwrap();
}

#[test]
fn held_combinations_are_independent_of_order_and_repeats() {
    use KeyEventKind::{Press, Release, Repeat};
    for second in [KeyCode::Right, KeyCode::Up, KeyCode::Char('d')] {
        let mut expected = None;
        for reverse in [false, true] {
            for repeats in [0, 1, 8] {
                let (mut runtime, clock) = game(true);
                let mut codes = [KeyCode::Char('w'), second];
                if reverse {
                    codes.reverse();
                }
                for code in codes {
                    runtime.step(&[key(code, Press)]).unwrap();
                }
                for _ in 0..5 {
                    for _ in 0..repeats {
                        runtime.step(&[key(second, Repeat)]).unwrap();
                    }
                    frame(&mut runtime, &clock);
                }
                let state = runtime.state();
                let result = (state.player, state.yaw, state.pitch);
                if let Some(expected) = expected {
                    assert_eq!(result, expected);
                } else {
                    expected = Some(result);
                }
                assert!(state.player[2] > 17.5);
                match second {
                    KeyCode::Right => assert!(state.yaw > 0.0),
                    KeyCode::Up => assert!(state.pitch > 0.0),
                    _ => {
                        assert!(state.player[0] > 28.5);
                        let distance = (state.player[0] - 28.5).hypot(state.player[2] - 17.5);
                        assert!((distance - 0.6).abs() < 1e-10);
                    }
                }
                runtime.step(&[key(second, Release)]).unwrap();
                let before = (
                    runtime.state().player,
                    runtime.state().yaw,
                    runtime.state().pitch,
                );
                frame(&mut runtime, &clock);
                assert_ne!(runtime.state().player, before.0);
                assert_eq!(runtime.state().yaw, before.1);
                assert_eq!(runtime.state().pitch, before.2);
                runtime.step(&[key(KeyCode::Char('w'), Release)]).unwrap();
                let stopped = runtime.state().player;
                frame(&mut runtime, &clock);
                assert_eq!(runtime.state().player, stopped);
            }
        }
    }
}

#[test]
fn opposing_keys_cancel_and_partial_release_keeps_other_keys_held() {
    use KeyEventKind::{Press, Release};
    let (mut runtime, clock) = game(true);
    runtime
        .step(&[
            key(KeyCode::Char('w'), Press),
            key(KeyCode::Char('s'), Press),
            key(KeyCode::Char('a'), Press),
            key(KeyCode::Char('d'), Press),
            key(KeyCode::Left, Press),
            key(KeyCode::Right, Press),
            key(KeyCode::Up, Press),
            key(KeyCode::Down, Press),
            key(KeyCode::Char(' '), Press),
            key(KeyCode::Char('c'), Press),
        ])
        .unwrap();
    let before = runtime.state().player;
    frame(&mut runtime, &clock);
    assert_eq!(runtime.state().player, before);
    assert_eq!((runtime.state().yaw, runtime.state().pitch), (0.0, 0.0));
    runtime
        .step(&[
            key(KeyCode::Char('s'), Release),
            key(KeyCode::Char(' '), Release),
        ])
        .unwrap();
    frame(&mut runtime, &clock);
    assert!(runtime.state().player[2] > before[2]);
    assert!(runtime.state().player[1] < before[1]);
}

#[test]
fn held_motion_follows_elapsed_time_at_30_60_and_120_fps() {
    for fps in [30, 60, 120] {
        let (runtime, _) = game(true);
        let mut world = runtime.state().clone();
        for code in [
            KeyCode::Char('w'),
            KeyCode::Char('d'),
            KeyCode::Up,
            KeyCode::Char(' '),
        ] {
            assert!(world
                .input
                .key(KeyEvent::new(code, KeyModifiers::empty()), true));
        }
        for _ in 0..fps / 2 {
            world.advance(1.0 / f64::from(fps));
        }
        assert!(((world.player[0] - 28.5).hypot(world.player[2] - 17.5) - 3.0).abs() < 1e-10);
        assert!((world.player[1] - 23.0).abs() < 1e-10);
        assert!((world.pitch - 1.125).abs() < 1e-10);
    }
}

#[test]
fn transitions_clear_motion_and_require_release_before_rearming() {
    use KeyEventKind::{Press, Release, Repeat};
    for transition in [
        KeyCode::Char('p'),
        KeyCode::Char('?'),
        KeyCode::Home,
        KeyCode::Char('f'),
    ] {
        let (mut runtime, clock) = game(true);
        runtime
            .step(&[key(KeyCode::Char('w'), Press), key(KeyCode::Right, Press)])
            .unwrap();
        frame(&mut runtime, &clock);
        runtime
            .step(&[key(transition, Press), key(transition, Release)])
            .unwrap();
        if runtime.state().paused || runtime.state().help {
            runtime
                .step(&[key(KeyCode::Esc, Press), key(KeyCode::Esc, Release)])
                .unwrap();
        }
        let before = (runtime.state().player, runtime.state().yaw);
        runtime
            .step(&[
                key(KeyCode::Char('w'), Repeat),
                key(KeyCode::Right, Repeat),
                // Windows reports native autorepeat as additional Press events.
                key(KeyCode::Char('w'), Press),
                key(KeyCode::Right, Press),
            ])
            .unwrap();
        frame(&mut runtime, &clock);
        assert_eq!(runtime.state().player[0], before.0[0]);
        assert_eq!(runtime.state().player[2], before.0[2]);
        assert_eq!(runtime.state().yaw, before.1);
        runtime
            .step(&[
                key(KeyCode::Char('w'), Release),
                key(KeyCode::Char('w'), Press),
            ])
            .unwrap();
        frame(&mut runtime, &clock);
        assert_ne!(
            [runtime.state().player[0], runtime.state().player[2]],
            [before.0[0], before.0[2]]
        );
    }
}

#[test]
fn focus_and_modal_input_cannot_reintroduce_motion() {
    use KeyEventKind::{Press, Release, Repeat};
    let (mut runtime, clock) = game(true);
    runtime
        .step(&[key(KeyCode::Char('w'), Press), Event::FocusLost])
        .unwrap();
    let before = runtime.state().player;
    runtime
        .step(&[
            key(KeyCode::Char('w'), Release),
            key(KeyCode::Char('d'), Press),
        ])
        .unwrap();
    frame(&mut runtime, &clock);
    assert_eq!(runtime.state().player, before);
    runtime
        .step(&[Event::FocusGained, key(KeyCode::Char('d'), Repeat)])
        .unwrap();
    frame(&mut runtime, &clock);
    assert_eq!(runtime.state().player, before);
    runtime
        .step(&[
            key(KeyCode::Char('?'), Press),
            key(KeyCode::Char('?'), Release),
        ])
        .unwrap();
    assert!(runtime.state().help);
    // Space is observed first, then consumed by the modal's resume button.
    runtime.step(&[key(KeyCode::Char(' '), Press)]).unwrap();
    assert!(!runtime.state().help);
    runtime.step(&[key(KeyCode::Char(' '), Repeat)]).unwrap();
    frame(&mut runtime, &clock);
    assert_eq!(runtime.state().player, before);
    runtime.step(&[key(KeyCode::Char(' '), Release)]).unwrap();
    frame(&mut runtime, &clock);
    assert_eq!(runtime.state().player, before);
}

#[test]
fn modifiers_do_not_leave_stuck_keys_or_accidentally_move() {
    let (mut runtime, clock) = game(true);
    runtime
        .step(&[key(KeyCode::Char('w'), KeyEventKind::Press)])
        .unwrap();
    frame(&mut runtime, &clock);
    runtime
        .step(&[Event::Key(KeyEvent::new_with_kind(
            KeyCode::Char('W'),
            KeyModifiers::SHIFT,
            KeyEventKind::Release,
        ))])
        .unwrap();
    let stopped = runtime.state().player;
    frame(&mut runtime, &clock);
    assert_eq!(runtime.state().player, stopped);
    runtime
        .step(&[Event::Key(KeyEvent::new(
            KeyCode::Char('w'),
            KeyModifiers::CONTROL,
        ))])
        .unwrap();
    frame(&mut runtime, &clock);
    assert_eq!(runtime.state().player, stopped);
}

#[test]
fn toggles_fire_once_and_mouse_look_cancels_held_turns() {
    use KeyEventKind::{Press, Release, Repeat};
    let (mut runtime, clock) = game(true);
    runtime
        .step(&[
            key(KeyCode::Char('n'), Press),
            key(KeyCode::Char('n'), Repeat),
            key(KeyCode::Char('n'), Press),
            key(KeyCode::Char('n'), Release),
        ])
        .unwrap();
    assert!(runtime.state().night);
    runtime
        .step(&[key(KeyCode::Char('w'), Press), key(KeyCode::Right, Press)])
        .unwrap();
    runtime.handle().send(Msg::Look(0.2, 0.1)).unwrap();
    runtime.step(&[]).unwrap();
    frame(&mut runtime, &clock);
    assert_eq!(runtime.state().yaw, 0.2);
    assert_eq!(runtime.state().pitch, 0.1);
    assert_ne!(runtime.state().player[2], 17.5);
}

#[test]
fn legacy_fallback_keeps_finite_smoothed_motion() {
    let (mut runtime, clock) = game(false);
    runtime
        .step(&[key(KeyCode::Char('w'), KeyEventKind::Press)])
        .unwrap();
    for _ in 0..50 {
        frame(&mut runtime, &clock);
    }
    assert!((runtime.state().player[2] - 17.5 - 0.36).abs() < 1e-7);
    assert!(runtime.state().notice.contains("Legacy"));
}
