use super::*;
use crossterm::event::KeyEvent;
use scene::{Action, Character};

#[test]
fn every_character_has_complete_transparent_clips_at_both_sizes() {
    for character in Character::ALL {
        for display_height in [48, 64] {
            let (atlas, size) = character.atlas(display_height);
            let bytes = size * size * 4;
            assert_eq!(size, display_height);
            assert_eq!(atlas.len(), bytes * 24);
            let colors: std::collections::BTreeSet<_> = atlas
                .chunks_exact(4)
                .filter(|pixel| pixel[3] != 0)
                .map(|pixel| &pixel[..3])
                .collect();
            let max_colors = match character {
                Character::ChatGpt => 25,
                Character::DeepSeek => 16,
                Character::Claude => 17,
            };
            assert!(colors.len() <= max_colors);
            for action in Action::ALL {
                let start = action as usize * 4 * bytes;
                let frames: Vec<_> = atlas[start..start + 4 * bytes]
                    .chunks_exact(bytes)
                    .collect();
                let distinct: std::collections::BTreeSet<_> = frames.iter().copied().collect();
                assert_eq!(
                    distinct.len(),
                    if action == Action::Idle { 2 } else { 4 },
                    "{character:?}/{action:?}/{size} lost animation poses"
                );
                for frame in frames {
                    assert!(frame.chunks_exact(4).any(|pixel| pixel[3] == 255));
                    assert!(frame.chunks_exact(4).any(|pixel| pixel[3] == 0));
                    assert!(frame
                        .chunks_exact(4)
                        .all(|pixel| pixel[3] == 0 || pixel[3] == 255));
                }
            }
        }
    }
    for pair in Character::ALL.windows(2) {
        assert_ne!(pair[0].atlas(64).0, pair[1].atlas(64).0);
    }
}

#[test]
fn companions_blink_and_wave_without_flickering_hair_skirts_or_feet() {
    for character in [Character::DeepSeek, Character::Claude] {
        for height in [24, 32] {
            let mut nursery = Nursery {
                character,
                ..Nursery::default()
            };
            let baseline = export::rgba(&export::frame(&mut nursery, (80, height).into()).unwrap());
            for (action, times) in [
                (Action::Idle, [0.9, 1.5, 1.65]),
                (Action::Wave, [0.3, 0.5, 0.7]),
            ] {
                nursery.action = Some(action);
                for time in times {
                    nursery.time = time;
                    let actual =
                        export::rgba(&export::frame(&mut nursery, (80, height).into()).unwrap());
                    for y in 0..usize::from(height) * 4 {
                        // Head crown and lower quarter are stationary, including
                        // alpha. Check the encoded output, not just atlas bytes.
                        if y < usize::from(height) || y >= usize::from(height) * 3 {
                            assert_eq!(
                                &actual[y * 160..(y + 1) * 160],
                                &baseline[y * 160..(y + 1) * 160],
                                "{character:?}/{action:?}/{height}: flicker at row {y}"
                            );
                        }
                    }
                    if action == Action::Wave || time == 1.5 {
                        assert_ne!(actual, baseline, "cleanup erased the gesture");
                    }
                }
            }
        }
    }
}

#[test]
fn character_selection_restarts_clip_preserves_pause_and_clears_old_sprite() {
    use rsille::render::buffer::Buffer;
    let size = Size::from((80, 32));
    let mut buffer = Buffer::new(size);
    let mut nursery = Nursery {
        paused: true,
        action: Some(Action::Wave),
        ..Nursery::default()
    };
    let press = |code| Event::Key(KeyEvent::new(code, KeyModifiers::NONE));
    for (key, character) in [
        (KeyCode::Tab, Character::DeepSeek),
        (KeyCode::Tab, Character::Claude),
        (KeyCode::Tab, Character::ChatGpt),
        (KeyCode::Char('c'), Character::Claude),
        (KeyCode::Char('d'), Character::DeepSeek),
        (KeyCode::Char('g'), Character::ChatGpt),
    ] {
        nursery.time = 1.0;
        nursery.last = Some(Instant::now());
        nursery.on_events(&[press(key)]).unwrap();
        assert_eq!(nursery.character, character);
        assert_eq!(nursery.action, Some(Action::Wave));
        assert_eq!(nursery.time, 0.0);
        assert!(nursery.last.is_none());
        assert!(nursery.paused);
        assert!(!nursery.should_quit());
        buffer.clear();
        nursery
            .draw(Chunk::new(&mut buffer, size.into()).unwrap())
            .unwrap();
        let fresh = export::frame(
            &mut Nursery {
                character,
                action: Some(Action::Wave),
                ..Nursery::default()
            },
            size,
        )
        .unwrap();
        assert_eq!(export::rgba(&buffer), export::rgba(&fresh));
    }
}

#[test]
fn character_cli_selection_applies_to_playback_and_exports() {
    let empty: Vec<String> = vec![];
    assert_eq!(
        animation_args(&empty).unwrap(),
        (Character::ChatGpt, INLINE_HEIGHT, empty.as_slice())
    );
    for character in Character::ALL {
        for tail in [
            vec![],
            vec!["--headless"],
            vec!["--snapshot", "frame.ppm"],
            vec!["--frames", "frames"],
            vec!["--action", "wave", "frames"],
        ] {
            let mut args = vec!["--character".to_string(), character.name().to_string()];
            args.extend(tail.iter().map(|s| s.to_string()));
            let (selected, height, remaining) = animation_args(&args).unwrap();
            assert_eq!(height, INLINE_HEIGHT);
            assert_eq!(selected, character);
            assert_eq!(remaining, &args[2..]);
        }
    }
    assert!(animation_args(&["--character".into()]).is_err());
    assert!(animation_args(&["--character".into(), "unknown".into()]).is_err());
}

#[test]
fn animation_height_accepts_either_option_order_and_rejects_invalid_rows() {
    for args in [
        vec!["--height", "24", "--character", "claude", "--frames", "out"],
        vec!["--character", "claude", "--height", "24", "--frames", "out"],
    ] {
        let args: Vec<String> = args.into_iter().map(String::from).collect();
        let (character, height, remaining) = animation_args(&args).unwrap();
        assert_eq!(character, Character::Claude);
        assert_eq!(height, 24);
        assert_eq!(remaining, &["--frames", "out"]);
    }
    for value in ["0", "129", "-1", "abc", "65536"] {
        assert!(animation_args(&["--height".into(), value.into()]).is_err());
    }
    assert!(animation_args(&["--height".into()]).is_err());
    for height in [1, 24, 32, 128] {
        assert_eq!(
            animation_args(&["--height".into(), height.to_string()])
                .unwrap()
                .1,
            height
        );
    }
}

#[test]
fn clips_use_distinct_embedded_drawings_and_loop_at_their_own_duration() {
    for action in Action::ALL {
        let mut seen = std::collections::BTreeSet::new();
        for tick in 0..(action.duration() * 100.0) as usize {
            let time = tick as f64 / 100.0;
            let frame = scene::current_frame(time, Some(action));
            assert!((action as usize * 4..action as usize * 4 + 4).contains(&frame));
            assert_eq!(
                frame,
                scene::current_frame(time + action.duration(), Some(action))
            );
            seen.insert(frame);
        }
        assert_eq!(seen.len(), 4);
        let frames: Vec<_> = seen
            .into_iter()
            .map(|index| {
                &scene::SPRITES[index * scene::FRAME_BYTES..(index + 1) * scene::FRAME_BYTES]
            })
            .collect();
        let distinct: std::collections::BTreeSet<_> = frames.iter().copied().collect();
        assert_eq!(distinct.len(), if action == Action::Idle { 2 } else { 4 });
        for frame in frames {
            assert!(frame.chunks_exact(4).any(|px| px[3] == 0));
            assert!(frame.chunks_exact(4).any(|px| px[3] == 255));
        }
    }
}

#[test]
fn playlist_boundaries_and_authored_holds_are_deterministic() {
    assert_eq!(Action::Idle.frame(0.899), 0);
    assert_eq!(Action::Idle.frame(0.9), 1);
    assert_eq!(Action::Idle.frame(1.5), 2);
    assert_eq!(Action::Idle.frame(1.65), 3);
    let mut time = 0.0;
    for action in Action::ALL {
        assert_eq!(scene::current_frame(time, None), action as usize * 4);
        time += action.duration();
    }
    assert_eq!(time, scene::PERIOD);
    for tick in 0..1702 {
        let time = tick as f64 / 100.0;
        assert_eq!(
            scene::current_frame(time, None),
            scene::current_frame(time + scene::PERIOD, None)
        );
    }
}

#[test]
fn all_actions_survive_terminal_encoding_and_resize() {
    for character in Character::ALL {
        let mut nursery = Nursery {
            character,
            ..Nursery::default()
        };
        for action in std::iter::once(None).chain(Action::ALL.map(Some)) {
            nursery.action = action;
            for size in [(1, 1), (12, 6), (80, 24), (80, 32), (200, 40), (32, 60)] {
                for time in [0.0, 0.95, 1.55, 2.78, 4.4, 7.99, 86400.0] {
                    nursery.time = time;
                    let frame = export::frame(&mut nursery, size.into()).unwrap();
                    assert_eq!(frame.size(), Size::from(size));
                    assert!(frame.content().iter().all(|cell| !cell.content.is_empty()));
                    assert!(!export::snapshot(&frame).is_empty());
                }
            }
        }
    }
}

#[test]
fn encoded_actions_change_visibly_at_normal_terminal_size() {
    for character in Character::ALL {
        for action in Action::ALL {
            let mut nursery = Nursery {
                character,
                action: Some(action),
                ..Nursery::default()
            };
            let initial = export::frame(&mut nursery, (80, 24).into()).unwrap();
            let changed = (1..20).any(|tick| {
                nursery.time = tick as f64 * action.duration() / 20.0;
                let next = export::frame(&mut nursery, (80, 24).into()).unwrap();
                initial.content() != next.content()
            });
            assert!(
                changed,
                "{} lost motion after terminal encoding",
                action.name()
            );
        }
    }
}

#[test]
fn pause_action_restart_and_exit_controls_work() {
    let mut nursery = Nursery::default();
    let press = |code| Event::Key(KeyEvent::new(code, KeyModifiers::NONE));
    nursery.on_events(&[press(KeyCode::Char(' '))]).unwrap();
    assert!(nursery.paused);
    let time = nursery.time;
    nursery.update().unwrap();
    nursery.update().unwrap();
    assert_eq!(time, nursery.time);
    for (i, action) in Action::ALL.into_iter().enumerate() {
        nursery.time = 10.0;
        nursery
            .on_events(&[press(KeyCode::Char(char::from(b'1' + i as u8)))])
            .unwrap();
        assert_eq!(nursery.action, Some(action));
        assert_eq!(nursery.time, 0.0);
        assert!(nursery.paused);
    }
    nursery.on_events(&[press(KeyCode::Char('0'))]).unwrap();
    assert_eq!(nursery.action, None);
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

#[test]
fn idle_and_wave_keep_hair_and_lower_body_identical() {
    for (atlas, size) in [Character::ChatGpt.atlas(48), Character::ChatGpt.atlas(64)] {
        let bytes = size * size * 4;
        let base = &atlas[..bytes];
        for frame in 1..8 {
            let pixels = &atlas[frame * bytes..(frame + 1) * bytes];
            for y in 0..size {
                if y < size / 4 || y > size * 4 / 5 {
                    assert_eq!(
                        &base[y * size * 4..(y + 1) * size * 4],
                        &pixels[y * size * 4..(y + 1) * size * 4],
                        "stationary region changed in frame {frame}, row {y}, size {size}"
                    );
                }
            }
        }
        for frame in [1, 3, 4] {
            assert_eq!(base, &atlas[frame * bytes..(frame + 1) * bytes]);
        }
        let colors: std::collections::BTreeSet<_> = atlas
            .chunks_exact(4)
            .filter(|px| px[3] != 0)
            .map(|px| &px[..3])
            .collect();
        assert!(colors.len() <= 25);
        assert!(atlas.chunks_exact(4).all(|px| px[3] == 0 || px[3] == 255));
    }
}

#[test]
fn facial_skin_highlights_stay_warm_and_eye_glints_stay_white() {
    for (size, skin_points, glint) in [
        (64, [(32, 28), (33, 30), (25, 34), (32, 35)], (24, 30)),
        (48, [(24, 21), (24, 25), (24, 26), (25, 26)], (18, 22)),
    ] {
        let (atlas, width) = Character::ChatGpt.atlas(size);
        let pixel = |frame: usize, x: usize, y: usize| {
            let offset = (frame * width * width + y * width + x) * 4;
            &atlas[offset..offset + 4]
        };
        for frame in [0, 1, 3, 4, 5, 7] {
            for (x, y) in skin_points {
                let rgba = pixel(frame, x, y);
                assert_eq!(rgba[3], 255);
                assert!(
                    rgba[0] >= 239 && rgba[0] - rgba[2] >= 20,
                    "skin turned white at frame {frame}, {x},{y}, size {size}: {rgba:?}"
                );
            }
            let rgba = pixel(frame, glint.0, glint.1);
            assert!(
                rgba[..3].iter().all(|&channel| channel >= 245),
                "eye glint was recolored at frame {frame}, size {size}"
            );
        }
    }
}

#[test]
fn terminal_encoding_preserves_every_sample_color() {
    for character in Character::ALL {
        for (w, h) in [(80u16, 32u16), (80, 24), (51, 23), (32, 60)] {
            for action in Action::ALL {
                let mut nursery = Nursery {
                    character,
                    action: Some(action),
                    time: 0.75,
                    ..Nursery::default()
                };
                let expected = scene::render(
                    w as usize,
                    h as usize * 2,
                    nursery.time,
                    nursery.action,
                    nursery.character,
                );
                let buffer = export::frame(&mut nursery, (w, h).into()).unwrap();
                let actual = export::rgba(&buffer);
                for y in 0..usize::from(h) * 4 {
                    for x in 0..usize::from(w) * 2 {
                        let offset = y * usize::from(w) * 2 + x;
                        let expected = expected[(y / 2) * usize::from(w) + x / 2]
                            .map_or([0, 0, 0, 0], |[r, g, b]| [r, g, b, 255]);
                        assert_eq!(actual[offset], expected);
                    }
                }
            }
        }
    }
}

#[test]
fn transparent_cells_and_edges_inherit_terminal_background_without_trails() {
    for character in Character::ALL {
        use rsille::render::buffer::Buffer;
        let size: Size = (80, 32).into();
        let mut buffer = Buffer::new(size);
        let mut nursery = Nursery {
            character,
            ..Nursery::default()
        };
        let mut saw_partial = false;
        let mut saw_cleared = false;
        let mut previous = vec![false; 80 * 32];
        for time in [0.0, 3.1, 6.65, 13.0, 0.0] {
            buffer.clear();
            nursery.time = time;
            nursery
                .draw(Chunk::new(&mut buffer, size.into()).unwrap())
                .unwrap();
            let pixels = scene::render(80, 64, time, None, nursery.character);
            for y in 0..32 {
                for x in 0..80 {
                    let cell = buffer.get((x as u16, y as u16).into()).unwrap();
                    let top = pixels[(y * 2) * 80 + x];
                    let bottom = pixels[(y * 2 + 1) * 80 + x];
                    if top.is_none() || bottom.is_none() {
                        assert_eq!(cell.style.colors.and_then(|c| c.background), None);
                    }
                    if top.is_none() && bottom.is_none() {
                        assert_eq!(cell.as_char(), Some(' '));
                        assert_eq!(cell.style.colors.and_then(|c| c.foreground), None);
                        saw_cleared |= previous[y * 80 + x];
                    } else if top.is_none() || bottom.is_none() {
                        saw_partial = true;
                    }
                    previous[y * 80 + x] = top.is_some() || bottom.is_some();
                }
            }
        }
        assert!(
            saw_partial,
            "must exercise a partially transparent character cell"
        );
        assert!(
            saw_cleared,
            "must exercise pixels vacated by the previous pose"
        );
    }
}
