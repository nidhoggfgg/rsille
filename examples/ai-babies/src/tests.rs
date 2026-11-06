use super::*;
use crossterm::event::KeyEvent;

#[test]
fn embedded_sprites_survive_terminal_encoding_and_resize() {
    let mut nursery = Nursery::default();
    for focus in [None, Some(0), Some(1), Some(2)] {
        nursery.focus = focus;
        for size in [(1, 1), (12, 6), (80, 24), (128, 32), (200, 40), (32, 60)] {
            for time in [0.0, 2.78, 4.4, 7.99, 86400.0] {
                nursery.time = time;
                let frame = export::frame(&mut nursery, size.into()).unwrap();
                assert_eq!(frame.size(), Size::from(size));
                assert!(frame.content().iter().all(|cell| !cell.content.is_empty()));
            }
        }
    }
}

#[test]
fn each_character_blinks_hops_and_loops() {
    for id in 0..3 {
        let mut nursery = Nursery {
            focus: Some(id),
            ..Nursery::default()
        };
        nursery.time = -(id as f64) * 2.0;
        let idle = export::frame(&mut nursery, (128, 32).into()).unwrap();
        nursery.time += 2.78;
        let blink = export::frame(&mut nursery, (128, 32).into()).unwrap();
        assert_ne!(idle.content(), blink.content());
        nursery.time += scene::PERIOD;
        let repeat = export::frame(&mut nursery, (128, 32).into()).unwrap();
        assert_eq!(blink.content(), repeat.content());
        nursery.time = 4.4 - id as f64 * 2.0;
        let hop = export::frame(&mut nursery, (128, 32).into()).unwrap();
        assert_ne!(idle.content(), hop.content());
    }
}

#[test]
fn idle_motion_preserves_sprite_pixels_throughout_the_loop() {
    // Remove translation and labels, retaining every colored sprite pixel.
    // Both open-eye and blinking frames must keep a constant shape and color.
    fn normalized_sprite(w: usize, h: usize, time: f64, id: usize) -> Vec<(usize, usize, [u8; 3])> {
        let pixels = scene::render(w, h, time, Some(id));
        let mut points: Vec<_> = pixels[..w * (h - 8)]
            .iter()
            .enumerate()
            .filter(|(_, rgb)| **rgb != scene::BACKGROUND)
            .map(|(i, &rgb)| (i % w, i / w, rgb))
            .collect();
        let left = points.iter().map(|p| p.0).min().unwrap();
        let top = points.iter().map(|p| p.1).min().unwrap();
        for point in &mut points {
            point.0 -= left;
            point.1 -= top;
        }
        points
    }

    for (w, h) in [(80, 48), (128, 64), (200, 80), (32, 120)] {
        for id in 0..3 {
            let offset = id as f64 * 2.0;
            let open = normalized_sprite(w, h, -offset, id);
            let closed = normalized_sprite(w, h, 2.78 - offset, id);
            for frame in 0..192 {
                let phase = frame as f64 / 24.0;
                let blinking = (2.7..2.88).contains(&phase) || (6.1..6.24).contains(&phase);
                let expected = if blinking { &closed } else { &open };
                assert!(
                    &normalized_sprite(w, h, phase - offset, id) == expected,
                    "sprite {id} deformed at frame {frame} in {w}x{h}"
                );
            }
        }
    }
}

#[test]
fn pause_focus_and_exit_controls_work() {
    let mut nursery = Nursery::default();
    let press = |code| Event::Key(KeyEvent::new(code, KeyModifiers::NONE));
    nursery.on_events(&[press(KeyCode::Char(' '))]).unwrap();
    assert!(nursery.paused);
    let time = nursery.time;
    nursery.update().unwrap();
    nursery.update().unwrap();
    assert_eq!(time, nursery.time);
    for (key, focus) in [('1', Some(0)), ('2', Some(1)), ('3', Some(2)), ('0', None)] {
        nursery.on_events(&[press(KeyCode::Char(key))]).unwrap();
        assert_eq!(nursery.focus, focus);
    }
    nursery.on_events(&[press(KeyCode::Char(' '))]).unwrap();
    assert!(!nursery.paused);
    for event in [
        press(KeyCode::Char('q')),
        press(KeyCode::Esc),
        Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
    ] {
        nursery.quit = false;
        nursery.on_events(&[event]).unwrap();
        assert!(nursery.should_quit());
    }
}
