use super::*;

#[test]
fn every_fighter_attacks_and_reacts_without_input() {
    let mut attacks = [false; 3];
    let mut reactions = [false; 3];
    let mut min_x = [f32::INFINITY; 3];
    let mut max_x = [f32::NEG_INFINITY; 3];
    for frame in 0..720 {
        let beat = scene::Beat::at(frame as f64 / 30.0);
        for (i, pose) in beat.poses.iter().enumerate() {
            attacks[i] |= pose.attack > 0.8;
            reactions[i] |= pose.hit > 0.8;
            min_x[i] = min_x[i].min(pose.x);
            max_x[i] = max_x[i].max(pose.x);
            assert!([pose.x, pose.y, pose.lean].iter().all(|v| v.is_finite()));
        }
    }
    assert!(attacks.into_iter().all(|v| v));
    assert!(reactions.into_iter().all(|v| v));
    assert!((0..3).all(|i| max_x[i] - min_x[i] > 5.0));
}

#[test]
fn frames_animate_repeat_and_survive_resizing() {
    let mut battle = Battle::default();
    let first = export::frame(&mut battle, (128, 32).into()).unwrap();
    battle.time = 1.8;
    let attack = export::frame(&mut battle, (128, 32).into()).unwrap();
    let changed = first
        .content()
        .iter()
        .zip(attack.content())
        .filter(|(a, b)| a.content != b.content)
        .count();
    assert!(changed > 128 * 32 / 5);
    battle.time += scene::PERIOD;
    let repeated = export::frame(&mut battle, (128, 32).into()).unwrap();
    assert_eq!(attack.content(), repeated.content());
    for size in [(1, 1), (80, 24), (32, 60), (200, 20), (128, 32)] {
        for time in [
            0.0,
            6.0,
            10.0,
            14.0,
            18.0,
            22.0,
            scene::PERIOD - 1e-8,
            86400.0,
        ] {
            battle.time = time;
            let frame = export::frame(&mut battle, size.into()).unwrap();
            assert_eq!(frame.size(), Size::from(size));
            assert!(frame.content().iter().all(|cell| !cell.content.is_empty()));
            assert!(battle
                .scene
                .raster
                .pixels
                .iter()
                .flatten()
                .all(|v| v.is_finite()));
        }
    }
}

#[test]
fn loop_returns_to_idle_and_quit_is_available() {
    let end = scene::Beat::at(scene::PERIOD - 0.0001);
    let start = scene::Beat::at(0.0);
    for (a, b) in end.poses.iter().zip(start.poses) {
        assert!((a.x - b.x).abs() < 0.01 && (a.y - b.y).abs() < 0.01);
        assert!(a.attack < 0.01 && a.hit < 0.01);
    }
    for (code, modifiers) in [
        (KeyCode::Char('q'), KeyModifiers::NONE),
        (KeyCode::Esc, KeyModifiers::NONE),
        (KeyCode::Char('c'), KeyModifiers::CONTROL),
    ] {
        let mut battle = Battle::default();
        battle
            .on_events(&[Event::Key(crossterm::event::KeyEvent::new(code, modifiers))])
            .unwrap();
        assert!(battle.should_quit());
    }
}

#[test]
fn motion_has_no_pose_jumps_and_velocity_matches_at_loop_seam() {
    let mut previous = motion::poses(0.0);
    for frame in 1..=2880 {
        let next = motion::poses(frame as f64 / 120.0);
        for (a, b) in previous.iter().zip(next) {
            assert!(
                (a.x - b.x).abs() < 1.0 && (a.y - b.y).abs() < 0.7,
                "root position jumped at frame {frame}"
            );
            assert!((a.lean - b.lean).abs() < 0.06 && (a.squash - b.squash).abs() < 0.04);
            assert!(a.arms.iter().zip(b.arms).all(|(a, b)| (a - b).abs() < 0.12));
            assert!((1.0 - b.squash) > 0.7 && (1.0 + b.squash) > 0.7);
        }
        previous = next;
    }
    let before = motion::poses(scene::PERIOD - 0.001);
    let seam = motion::poses(0.0);
    let after = motion::poses(0.001);
    for i in 0..3 {
        for (a, b, c) in [
            (before[i].x, seam[i].x, after[i].x),
            (before[i].y, seam[i].y, after[i].y),
            (before[i].lean, seam[i].lean, after[i].lean),
        ] {
            assert!(
                ((b - a) - (c - b)).abs() < 0.0002,
                "velocity discontinuity at loop seam"
            );
        }
    }
}
