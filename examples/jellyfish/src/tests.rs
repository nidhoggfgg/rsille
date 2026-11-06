use crossterm::event::{KeyEvent, KeyEventState};

use super::*;

#[test]
fn animation_changes_and_rendering_is_deterministic() {
    let mut aquarium = Aquarium::default();
    let initial = export::frame(&mut aquarium, (80, 24).into()).unwrap();
    aquarium.time = 3.0;
    let later = export::frame(&mut aquarium, (80, 24).into()).unwrap();
    let changed = initial
        .content()
        .iter()
        .zip(later.content())
        .filter(|(a, b)| a.content != b.content)
        .count();
    assert!(
        changed > 80 * 24 / 4,
        "animation must visibly move without input"
    );
    let repeat = export::frame(&mut aquarium, (80, 24).into()).unwrap();
    assert_eq!(later.content(), repeat.content());
    aquarium.time = 0.0;
    assert_eq!(
        initial.content(),
        export::frame(&mut aquarium, (80, 24).into())
            .unwrap()
            .content()
    );
}

#[test]
fn resize_and_long_playback_produce_finite_complete_frames() {
    let mut aquarium = Aquarium::default();
    for size in [(1, 1), (80, 24), (24, 60), (200, 20), (144, 44), (1, 1)] {
        for time in [0.0, 31.5, 3600.0, 86400.0] {
            aquarium.time = time;
            let frame = export::frame(&mut aquarium, size.into()).unwrap();
            assert_eq!(frame.size(), Size::from(size));
            assert!(frame
                .content()
                .iter()
                .all(|cell| matches!(cell.content.as_char(), Some('▀' | '▄' | '█' | ' '))));
            assert!(aquarium
                .sea
                .pixels
                .iter()
                .flatten()
                .all(|v| v.is_finite() && *v >= 0.0));
        }
    }
}

#[test]
fn only_exit_keys_affect_playback() {
    let mut aquarium = Aquarium::default();
    for code in [KeyCode::Char(' '), KeyCode::Left, KeyCode::Char('1')] {
        aquarium
            .on_events(&[Event::Key(KeyEvent::new(code, KeyModifiers::NONE))])
            .unwrap();
    }
    assert!(!aquarium.should_quit());
    assert_eq!(aquarium.time, 0.0);
    for (code, modifiers) in [
        (KeyCode::Esc, KeyModifiers::NONE),
        (KeyCode::Char('q'), KeyModifiers::NONE),
        (KeyCode::Char('c'), KeyModifiers::CONTROL),
    ] {
        let mut aquarium = Aquarium::default();
        aquarium
            .on_events(&[Event::Key(KeyEvent {
                code,
                modifiers,
                kind: KeyEventKind::Release,
                state: KeyEventState::NONE,
            })])
            .unwrap();
        assert!(!aquarium.should_quit());
        aquarium
            .on_events(&[Event::Key(KeyEvent::new(code, modifiers))])
            .unwrap();
        assert!(aquarium.should_quit());
    }
}
