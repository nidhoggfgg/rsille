use super::*;
use battle::{Move, Phase, ACTION_SECONDS, ENEMY_HP, IMPACT_SECONDS, MAX_PP, PLAYER_HP};
use crossterm::event::{KeyEvent, MouseEvent};

type GameRuntime = Runtime<Game, Msg>;
use rsille::tui::runtime::ManualClock;
use std::time::Duration;

fn running(game: Game, size: (u16, u16)) -> (GameRuntime, ManualClock) {
    let clock = ManualClock::default();
    let mut runtime = app(Game {
        viewport: size.into(),
        ..game
    })
    .with_clock(clock.clone())
    .update(update)
    .view(view)
    .build()
    .size(size.0, size.1);
    runtime.step(&[]).unwrap();
    (runtime, clock)
}
fn press(runtime: &mut GameRuntime, key: KeyCode) {
    runtime
        .step(&[Event::Key(KeyEvent::new(key, KeyModifiers::NONE))])
        .unwrap();
}
fn advance(runtime: &mut GameRuntime, clock: &ManualClock, seconds: f64) {
    for _ in 0..(seconds * 20.0).round() as usize {
        clock.advance(Duration::from_millis(50));
        runtime.step(&[]).unwrap();
    }
}
fn move_path(runtime: &GameRuntime, index: usize) -> WidgetPath {
    let source = runtime
        .focused_path()
        .expect("at least one enabled native button");
    let mut path = WidgetPath::root();
    for segment in source.as_slice() {
        let key = match segment {
            WidgetKey::Named(key) if key.starts_with("skill-glass-") => {
                WidgetKey::from(format!("skill-glass-{index}"))
            }
            WidgetKey::Named(key) if key.starts_with("skill-card-") => {
                WidgetKey::from(format!("skill-card-{index}"))
            }
            WidgetKey::Named(key) if key.starts_with("move-") && key != "move-deck" => {
                WidgetKey::from(format!("move-{index}"))
            }
            other => other.clone(),
        };
        path = path.child(key);
    }
    path
}
fn move_bounds(runtime: &GameRuntime, index: usize) -> rsille::render::area::Area {
    let path = move_path(runtime, index);
    runtime
        .bounds(&path)
        .unwrap_or_else(|| panic!("missing button bounds: {path:?}"))
}
fn text(runtime: &GameRuntime) -> String {
    runtime
        .frame()
        .content()
        .iter()
        .map(|c| c.content.symbol())
        .collect()
}
fn mouse(kind: MouseEventKind, column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}
fn round(battle: &mut Battle, index: usize) {
    assert!(battle.choose(index));
    battle.advance(ACTION_SECONDS * 2.0);
}

#[test]
fn waits_for_input_then_resolves_each_hit_once_and_returns_control() {
    let mut battle = Battle::default();
    battle.advance(100.0);
    assert_eq!(battle.phase, Phase::Choose);
    assert_eq!((battle.player_hp, battle.enemy_hp), (PLAYER_HP, ENEMY_HP));
    assert!(!battle.choose(9));
    assert!(battle.choose(0));
    assert_eq!(battle.pp[0], MAX_PP[0] - 1);
    assert!(!battle.choose(1), "input during an attack must be locked");
    battle.advance(IMPACT_SECONDS - 0.01);
    assert_eq!(battle.enemy_hp, ENEMY_HP);
    battle.advance(0.02);
    assert_eq!(battle.enemy_hp, ENEMY_HP - 48);
    battle.advance(0.2);
    assert_eq!(
        battle.enemy_hp,
        ENEMY_HP - 48,
        "damage must not repeat per frame"
    );
    battle.advance(4.0);
    assert_eq!(battle.player_hp, PLAYER_HP - 28);
    assert_eq!(battle.phase, Phase::Choose);
    assert_eq!(battle.turn, 2);
}

#[test]
fn pp_exhaustion_is_rejected_without_spending_a_turn() {
    let mut battle = Battle::default();
    for _ in 0..3 {
        round(&mut battle, 0);
    }
    assert_eq!(battle.pp[0], 0);
    let hp = battle.enemy_hp;
    let turn = battle.turn;
    assert!(!battle.choose(0));
    assert_eq!(battle.phase, Phase::Choose);
    assert_eq!(battle.turn, turn);
    assert_eq!(battle.enemy_hp, hp);
    assert!(battle.message.contains("No PP"));
    assert!(battle.choose(2));
}

#[test]
fn paralysis_prevents_replies_and_cannot_be_spammed() {
    let mut battle = Battle::default();
    round(&mut battle, 3);
    assert!(battle.paralyzed);
    let pp = battle.pp;
    assert!(!battle.choose(3));
    assert_eq!(battle.pp, pp);
    let hp = battle.player_hp;
    round(&mut battle, 1);
    assert_eq!(battle.player_hp, hp, "second enemy reply must be paralyzed");
    round(&mut battle, 1);
    assert!(battle.guard, "third enemy reply should use Withdraw");
}

#[test]
fn withdraw_halves_one_hit_and_iron_tail_breaks_it() {
    let mut guarded = Battle {
        guard: true,
        ..Battle::default()
    };
    round(&mut guarded, 0);
    assert_eq!(guarded.enemy_hp, ENEMY_HP - 24);
    assert!(!guarded.guard);
    let mut breaker = Battle {
        guard: true,
        ..Battle::default()
    };
    round(&mut breaker, 2);
    assert_eq!(breaker.enemy_hp, ENEMY_HP - 34);
    assert!(!breaker.guard);
}

#[test]
fn victory_stops_retaliation_and_defeat_stops_new_moves() {
    let mut winner = Battle {
        enemy_hp: 10,
        ..Battle::default()
    };
    round(&mut winner, 1);
    assert_eq!(winner.enemy_hp, 0);
    assert_eq!(winner.player_hp, PLAYER_HP);
    assert_eq!(winner.phase, Phase::Won);
    let frozen = winner.pp;
    assert!(!winner.choose(0));
    winner.advance(100.0);
    assert_eq!(winner.pp, frozen);
    assert_eq!(winner.phase, Phase::Won);
    let mut loser = Battle::default();
    while loser.phase == Phase::Choose {
        round(&mut loser, 1);
    }
    assert_eq!(loser.phase, Phase::Lost);
    assert_eq!(loser.player_hp, 0);
    assert!(!loser.choose(0));
}

#[test]
fn combat_results_are_independent_of_frame_rate() {
    let mut coarse = Battle::default();
    let mut fine = Battle::default();
    for index in [3, 0, 2, 1] {
        assert!(coarse.choose(index));
        assert!(fine.choose(index));
        coarse.advance(4.0);
        for _ in 0..240 {
            fine.advance(1.0 / 60.0);
        }
        assert_eq!(coarse.phase, fine.phase);
        assert_eq!(coarse.player_hp, fine.player_hp);
        assert_eq!(coarse.enemy_hp, fine.enemy_hp);
        assert_eq!(coarse.pp, fine.pp);
        assert_eq!(coarse.turn, fine.turn);
    }
}

#[test]
fn native_focus_arrows_enter_pause_repeat_and_rematch() {
    let (mut runtime, clock) = running(Game::default(), (160, 50));
    assert_eq!(runtime.focused_key(), Some("move-0"));
    press(&mut runtime, KeyCode::Right);
    assert_eq!(runtime.focused_key(), Some("move-1"));
    press(&mut runtime, KeyCode::Down);
    assert_eq!(runtime.focused_key(), Some("move-3"));
    press(&mut runtime, KeyCode::Enter);
    assert_eq!(
        runtime.state().battle.attack().unwrap().0,
        Move::ThunderWave
    );
    press(&mut runtime, KeyCode::Char('p'));
    assert!(runtime.state().paused);
    assert!(text(&runtime).contains("RESUME"));
    let before = runtime.state().battle;
    press(&mut runtime, KeyCode::Char('1'));
    advance(&mut runtime, &clock, 1.0);
    assert_eq!(runtime.state().battle, before);
    press(&mut runtime, KeyCode::Char('r'));
    assert_eq!(runtime.state().battle, Battle::default());
    assert!(!runtime.state().paused);
    let mut repeat = KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE);
    repeat.kind = KeyEventKind::Repeat;
    runtime.step(&[Event::Key(repeat)]).unwrap();
    assert_eq!(runtime.state().battle.phase, Phase::Choose);
    press(&mut runtime, KeyCode::Char('b'));
    assert_eq!(runtime.state().encoding, Encoding::Braille);
    press(&mut runtime, KeyCode::Char('q'));
    assert!(runtime.exit_reason().is_some());
}

#[test]
fn native_buttons_activate_once_and_disabled_moves_are_skipped_by_tab() {
    let game = Game {
        battle: Battle {
            pp: [0, 20, 8, 5],
            paralyzed: true,
            ..Battle::default()
        },
        ..Game::default()
    };
    let (mut runtime, _) = running(game, (160, 50));
    assert_eq!(runtime.focused_key(), Some("move-1"));
    press(&mut runtime, KeyCode::Tab);
    assert_eq!(runtime.focused_key(), Some("move-2"));
    press(&mut runtime, KeyCode::Tab);
    assert_eq!(runtime.focused_key(), Some("move-1"));
    let disabled = move_bounds(&runtime, 0);
    runtime
        .step(&[mouse(
            MouseEventKind::Down(MouseButton::Left),
            disabled.x() + 2,
            disabled.y() + disabled.height() / 2,
        )])
        .unwrap();
    assert_eq!(runtime.state().battle.phase, Phase::Choose);
    let enabled = move_bounds(&runtime, 2);
    runtime
        .step(&[
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                enabled.x() + 2,
                enabled.y() + enabled.height() / 2,
            ),
            mouse(
                MouseEventKind::Up(MouseButton::Left),
                enabled.x() + 2,
                enabled.y() + enabled.height() / 2,
            ),
        ])
        .unwrap();
    assert_eq!(runtime.state().battle.attack().unwrap().0, Move::IronTail);
    assert_eq!(runtime.state().battle.pp[2], 7);
    press(&mut runtime, KeyCode::Enter);
    press(&mut runtime, KeyCode::Char('2'));
    assert_eq!(runtime.state().battle.pp, [0, 20, 7, 5]);
}

#[test]
fn native_layout_hit_bounds_hover_and_resize_match_the_rendered_cards() {
    let (mut runtime, _) = running(Game::default(), (160, 50));
    for size in [(160, 50), (80, 24), (120, 38)] {
        runtime.step(&[Event::Resize(size.0, size.1)]).unwrap();
        assert_eq!(runtime.state().viewport, Size::from(size));
        for index in 0..4 {
            let r = move_bounds(&runtime, index);
            let path = move_path(&runtime, index);
            let before = runtime.state().battle;
            runtime
                .step(&[mouse(
                    MouseEventKind::Moved,
                    r.x() + 1,
                    r.y() + r.height() / 2,
                )])
                .unwrap();
            assert_eq!(runtime.hovered_path(), Some(&path));
            assert_eq!(runtime.state().battle, before);
            // The gap belongs to the grid, not either button.
            runtime
                .step(&[mouse(
                    MouseEventKind::Down(MouseButton::Left),
                    r.x() + r.width(),
                    r.y() + r.height() / 2,
                )])
                .unwrap();
            assert_eq!(runtime.state().battle, before);
        }
    }
    runtime.step(&[Event::FocusLost]).unwrap();
    assert!(runtime.hovered_path().is_none());
    let current = runtime
        .focused_key()
        .unwrap()
        .strip_prefix("move-")
        .unwrap()
        .parse::<usize>()
        .unwrap();
    press(&mut runtime, KeyCode::Right);
    assert_eq!(
        runtime.focused_key(),
        Some(format!("move-{}", current ^ 1).as_str())
    );
    runtime.step(&[Event::Resize(80, 24)]).unwrap();
    let current = runtime
        .focused_key()
        .unwrap()
        .strip_prefix("move-")
        .unwrap()
        .parse::<usize>()
        .unwrap();
    press(&mut runtime, KeyCode::Down);
    assert_eq!(
        runtime.focused_key(),
        Some(format!("move-{}", current ^ 2).as_str())
    );
    press(&mut runtime, KeyCode::Char('r'));
    let r = move_bounds(&runtime, 3);
    runtime
        .step(&[mouse(
            MouseEventKind::Down(MouseButton::Left),
            r.x() + 2,
            r.y() + r.height() / 2,
        )])
        .unwrap();
    assert_eq!(
        runtime.state().battle.attack().unwrap().0,
        Move::ThunderWave
    );
}

#[test]
fn native_drag_capture_does_not_escape_into_controls_and_cancels_on_focus_loss() {
    let (mut runtime, _) = running(Game::default(), (160, 50));
    let button = move_bounds(&runtime, 0);
    runtime
        .step(&[
            mouse(MouseEventKind::Down(MouseButton::Left), 70, 20),
            mouse(MouseEventKind::Drag(MouseButton::Left), 80, 23),
        ])
        .unwrap();
    assert!(runtime.state().camera.yaw > Camera::default().yaw);
    let yaw = runtime.state().camera.yaw;
    runtime
        .step(&[
            Event::FocusLost,
            mouse(MouseEventKind::Drag(MouseButton::Left), 90, 25),
            mouse(MouseEventKind::Drag(MouseButton::Left), 100, 26),
        ])
        .unwrap();
    assert_eq!(runtime.state().camera.yaw, yaw);
    assert_eq!(runtime.state().battle.phase, Phase::Choose);
    // A captured scene drag remains a scene drag when its pointer crosses a card.
    runtime
        .step(&[
            mouse(MouseEventKind::Down(MouseButton::Left), 70, 20),
            mouse(
                MouseEventKind::Drag(MouseButton::Left),
                button.x() + 2,
                button.y() + button.height() / 2,
            ),
            mouse(
                MouseEventKind::Up(MouseButton::Left),
                button.x() + 2,
                button.y() + button.height() / 2,
            ),
        ])
        .unwrap();
    assert_eq!(runtime.state().battle.phase, Phase::Choose);
}

#[test]
fn actual_tui_frames_resize_repeat_and_render_all_moves_without_mutating_combat() {
    for encoding in [Encoding::HalfBlock, Encoding::Braille] {
        for size in [(0, 0), (1, 1), (2, 3), (40, 12), (80, 24)] {
            for index in 0..4 {
                let mut game = Game {
                    encoding,
                    ..Game::default()
                };
                game.battle.choose(index);
                game.battle.advance(0.9);
                let before = game.battle;
                let a = export::frame(&game, size.into()).unwrap();
                let b = export::frame(&game, size.into()).unwrap();
                assert_eq!(export::svg(&a), export::svg(&b));
                assert_eq!(game.battle, before);
                assert!(a.content().iter().all(|c| !c.content.is_empty()));
                assert!(game
                    .raster
                    .borrow()
                    .pixels
                    .iter()
                    .flatten()
                    .all(|v| v.is_finite() && (0.0..=1.0).contains(v)));
            }
        }
    }
    let (runtime, _) = running(Game::default(), (80, 24));
    assert!(!text(&runtime).contains("What will"));
    for expected in ["120/120", "160/160", "THUNDERBOLT", "IRON TAIL"] {
        assert!(text(&runtime).contains(expected), "missing {expected}");
    }
}

#[test]
fn battle_hud_flanks_the_scene_and_hp_meters_track_damage() {
    let (mut runtime, clock) = running(Game::default(), (160, 50));
    let vitals = |name: &str| WidgetPath::root().child("battle-scene").child(name);
    let player = runtime.bounds(&vitals("player-vitals")).unwrap();
    let foe = runtime.bounds(&vitals("foe-vitals")).unwrap();
    assert!(player.x() < 40 && player.y() > 25);
    assert!(foe.x() > 100 && foe.y() < 10);
    let first_move = move_bounds(&runtime, 0);
    assert!(player.x() + player.width() < first_move.x());
    assert!(player.y() >= first_move.y());
    let region_text = |runtime: &GameRuntime, area: rsille::render::area::Area| -> String {
        (area.y()..area.y() + area.height())
            .flat_map(|y| (area.x()..area.x() + area.width()).map(move |x| (x, y)))
            .map(|pos| runtime.frame().get(pos.into()).unwrap().symbol())
            .collect()
    };
    assert!(region_text(&runtime, player).contains("PIKACHU"));
    assert!(region_text(&runtime, player).contains("120/120"));
    assert!(region_text(&runtime, foe).contains("BLASTOISE"));
    assert!(region_text(&runtime, foe).contains("160/160"));
    assert!(!region_text(&runtime, player).contains('╱'));
    assert!(!region_text(&runtime, foe).contains('╱'));
    let full_blocks = region_text(&runtime, foe).matches('█').count();
    assert!(full_blocks > 0, "native HP meters must be visible");
    press(&mut runtime, KeyCode::Char('1'));
    advance(&mut runtime, &clock, 0.8);
    assert_eq!(runtime.state().battle.enemy_hp, 112);
    assert!(region_text(&runtime, foe).contains("112/160"));
    assert!(region_text(&runtime, foe).matches('█').count() < full_blocks);
    advance(&mut runtime, &clock, 3.2);
    assert_eq!(runtime.state().battle.player_hp, 92);
    assert!(region_text(&runtime, player).contains("092/120"));
    assert_eq!(runtime.state().battle.phase, Phase::Choose);
}

#[test]
fn glass_overlays_follow_the_live_fullscreen_scene_without_blurring_text() {
    let (mut runtime, _) = running(Game::default(), (160, 50));
    let r = move_bounds(&runtime, 0);
    let frame = runtime.state().raster.borrow();
    assert_eq!(
        (frame.width, frame.height),
        (320, 200),
        "the scene must extend behind the skill dock"
    );
    drop(frame);
    press(&mut runtime, KeyCode::Char('p'));
    let backdrop_colors = |runtime: &GameRuntime| {
        (r.x()..r.x() + r.width())
            .map(|x| {
                runtime
                    .frame()
                    .get((x, r.y()).into())
                    .unwrap()
                    .style
                    .colors
                    .unwrap()
                    .background
            })
            .collect::<Vec<_>>()
    };
    let before = backdrop_colors(&runtime);
    assert!(
        before.windows(2).any(|pair| pair[0] != pair[1]),
        "glass must show spatially varying scenery: {before:?}; bounds {r:?}"
    );
    press(&mut runtime, KeyCode::Char('d'));
    let after = backdrop_colors(&runtime);
    assert_ne!(
        before, after,
        "camera orbit must update the backdrop under the stationary UI"
    );
    assert_eq!(runtime.state().battle, Battle::default());
    assert!(
        text(&runtime).contains("THUNDERBOLT"),
        "foreground text must remain sharp"
    );
    let corner = runtime.frame().get((r.x() - 8, r.y() - 1).into()).unwrap();
    assert!(
        matches!(corner.as_char(), Some('▀' | '▄' | '█')),
        "the cut-away corner must expose the actual scene, without a rectangular glass fill"
    );
}

#[test]
fn skill_diagonal_edges_connect_and_keep_the_untinted_scene_background() {
    for encoding in [Encoding::HalfBlock, Encoding::Braille] {
        for size in [(160, 50), (80, 24)] {
            let (runtime, _) = running(
                Game {
                    encoding,
                    ..Game::default()
                },
                size,
            );
            let mut areas = Vec::new();
            for index in 0..4 {
                let mut path = move_path(&runtime, index);
                for _ in 0..4 {
                    path.pop();
                }
                areas.push(runtime.bounds(&path).unwrap());
            }
            // Render the same scene without the skill dock. Edge backgrounds
            // use the representative color of encoded cells, not original pixels.
            let mut scene = app(runtime.state().clone())
                .with_clock(ManualClock::default())
                .update(update)
                .view(|game| battle_ui::scene_widget(game, battle_ui::wide(game.viewport)))
                .build()
                .size(size.0, size.1);
            scene.step(&[]).unwrap();
            for area in areas {
                let mut previous: Option<[u16; 2]> = None;
                for y in area.y()..area.y() + area.height() {
                    let slashes: Vec<_> = (area.x()..area.x() + area.width())
                        .filter(|&x| {
                            runtime.frame().get((x, y).into()).unwrap().as_char() == Some('╱')
                        })
                        .collect();
                    let edges = [*slashes.first().unwrap(), *slashes.last().unwrap()];
                    if let Some(previous) = previous {
                        assert_eq!(
                            previous,
                            edges.map(|x| x + 1),
                            "each diagonal must join the next cell corner"
                        );
                    }
                    for x in edges {
                        let raw = scene.frame().get((x, y).into()).unwrap();
                        let colors = raw.style.colors.unwrap();
                        let coverage = match raw.as_char().unwrap() {
                            '█' => 1.0,
                            '▀' | '▄' => 0.5,
                            ch @ '\u{2800}'..='\u{28ff}' => {
                                (ch as u32 - 0x2800).count_ones() as f64 / 8.0
                            }
                            _ => 0.0,
                        };
                        let rgb = |color| match color {
                            Some(crossterm::style::Color::Rgb { r, g, b }) => [r, g, b],
                            Some(crossterm::style::Color::Black) | None => [0; 3],
                            other => panic!("unexpected scene color: {other:?}"),
                        };
                        let fg = rgb(colors.foreground);
                        let bg = rgb(colors.background);
                        let [r, g, b] = std::array::from_fn(|c| {
                            (fg[c] as f64 * coverage + bg[c] as f64 * (1.0 - coverage)).round()
                                as u8
                        });
                        let cell = runtime.frame().get((x, y).into()).unwrap();
                        assert_eq!(
                            cell.style.colors.unwrap().background,
                            Some(crossterm::style::Color::Rgb { r, g, b }),
                            "edge cells must not paint rectangular panel tint over the scene"
                        );
                    }
                    previous = Some(edges);
                }
            }
        }
    }
}
