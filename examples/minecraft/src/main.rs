//! A tiny first-person Minecraft-style sandbox using rsille's application and renderer.
use rsille::tui::prelude::*;
use world::World;

mod input;
#[cfg(test)]
mod input_tests;
mod scene;
mod verify;
mod world;

#[derive(Clone, Copy)]
enum Msg {
    Key(KeyEvent),
    Capabilities(InputCapabilities),
    Focus(bool),
    Frame(FrameInfo),
    Walk(f64, f64),
    Look(f64, f64),
    Turn(f64, f64),
    Jump,
    Down,
    Mine,
    Place,
    Select(usize),
    Cycle(i32),
    Fly,
    Night,
    Home,
    Pause,
    Help,
    Escape,
    Quit,
}

fn app(world: World) -> App<World, Msg> {
    let mut theme = Theme::dark();
    let base = Style::default().fg(scene::TEXT).bg(scene::BG);
    theme.styles.surface = base;
    theme.styles.surface_modal = base;
    theme.styles.text = base;
    theme.styles.border = base.fg(Color::Rgb(151, 166, 113));
    App::new(world)
        .with_theme(theme)
        .disable_quit_key()
        .keyboard_mode(KeyboardMode::Auto)
        .focus_events(true)
        .on_frame(Msg::Frame)
        .on_input(input_message)
}

fn input_message(input: InputEvent) -> Option<Msg> {
    match input {
        InputEvent::Capabilities(capabilities) => Some(Msg::Capabilities(capabilities)),
        InputEvent::Terminal(Event::Key(key)) => Some(Msg::Key(key)),
        InputEvent::Terminal(Event::FocusLost) => Some(Msg::Focus(false)),
        InputEvent::Terminal(Event::FocusGained) => Some(Msg::Focus(true)),
        _ => None,
    }
}

fn key_message(world: &mut World, key: KeyEvent) -> Option<Msg> {
    let active = world.active();
    let activate = world.input.key(key, active);
    if key.kind == KeyEventKind::Press
        && key.code == KeyCode::Char('c')
        && key.modifiers.contains(KeyModifiers::CONTROL)
    {
        return Some(Msg::Quit);
    }
    if !activate {
        return None;
    }
    let code = input::normalize(key.code);
    if world.input.enhanced && input::is_motion(code) {
        return (code == KeyCode::Char(' ') && !world.flying).then_some(Msg::Jump);
    }
    Some(match code {
        KeyCode::Char('w') => Msg::Walk(1.0, 0.0),
        KeyCode::Char('s') => Msg::Walk(-1.0, 0.0),
        KeyCode::Char('a') => Msg::Walk(0.0, -1.0),
        KeyCode::Char('d') => Msg::Walk(0.0, 1.0),
        KeyCode::Left => Msg::Turn(-0.09, 0.0),
        KeyCode::Right => Msg::Turn(0.09, 0.0),
        KeyCode::Up => Msg::Turn(0.0, 0.075),
        KeyCode::Down => Msg::Turn(0.0, -0.075),
        KeyCode::Char(' ') => Msg::Jump,
        KeyCode::Char('c') => Msg::Down,
        KeyCode::Char('e') => Msg::Mine,
        KeyCode::Char('r') => Msg::Place,
        KeyCode::Char('f') => Msg::Fly,
        KeyCode::Char('n') => Msg::Night,
        KeyCode::Home => Msg::Home,
        KeyCode::Char('p') => Msg::Pause,
        KeyCode::Esc => Msg::Escape,
        KeyCode::Char('?') => Msg::Help,
        KeyCode::Char('q') => Msg::Quit,
        KeyCode::Char(c @ '1'..='8') => Msg::Select((c as u8 - b'1') as usize),
        _ => return None,
    })
}

fn update(world: &mut World, msg: Msg, ctx: &mut UpdateCtx<Msg>) {
    match msg {
        Msg::Capabilities(capabilities) => {
            world.stop_input();
            world.input.enhanced = capabilities.key_releases;
            world.say(if capabilities.key_releases {
                "Held-key input on / [?] controls"
            } else {
                "Legacy input: multi-key hold unavailable / [?] controls"
            });
        }
        Msg::Focus(focused) => {
            world.stop_input();
            world.input.focused = focused;
        }
        Msg::Key(key) => {
            if let Some(action) = key_message(world, key) {
                update(world, action, ctx);
            }
        }
        Msg::Quit => ctx.exit(),
        Msg::Help => {
            world.stop_input();
            world.help = !world.help;
        }
        Msg::Escape if world.help => {
            world.stop_input();
            world.help = false;
        }
        Msg::Escape | Msg::Pause if !world.help => {
            world.stop_input();
            world.paused = !world.paused;
        }
        _ if !world.active() => {}
        Msg::Frame(frame) => world.advance(frame.delta.as_secs_f64()),
        Msg::Walk(forward, side) => world.walk(forward, side),
        Msg::Look(yaw, pitch) => world.look(yaw, pitch),
        Msg::Turn(yaw, pitch) => world.turn(yaw, pitch),
        Msg::Jump => world.jump(),
        Msg::Down => world.vertical(-0.5),
        Msg::Mine => world.mine(),
        Msg::Place => world.place(),
        Msg::Select(slot) => world.selected = slot,
        Msg::Cycle(delta) => {
            world.selected = (world.selected as i32 + delta).rem_euclid(8) as usize
        }
        Msg::Fly => {
            world.stop_input();
            world.flying = !world.flying;
            world.velocity = 0.0;
            world.say(if world.flying {
                "Flight on / Space up / C down"
            } else {
                "Back on foot."
            });
        }
        Msg::Night => world.night = !world.night,
        Msg::Home => {
            world.stop_input();
            let spawn = World::default();
            world.player = spawn.player;
            world.yaw = spawn.yaw;
            world.pitch = spawn.pitch;
            world.flying = true;
            world.velocity = 0.0;
            // Find safe headroom even if the player built over the spawn point.
            while !world.free(world.player) && world.player[1] < 29.0 {
                world.player[1] += 1.0;
            }
            world.say("Returned to spawn / flight on");
        }
        Msg::Pause | Msg::Escape => {}
    }
}

fn view(world: &World) -> impl Widget<Msg> {
    let base = overlay(scene::Game {
        world: world.clone(),
    });
    if world.help || world.paused {
        let title = if world.help {
            " FIELD GUIDE / MINECRAFT "
        } else {
            " GAME PAUSED "
        };
        base.layer(OverlayLayer::new(dialog().key("guide").title(title).gap(1)
            .child(label("A tiny creative sandbox, made of terminal cells.\nGrass, trees, a lake, a cabin, and room to build."))
            .child(label("W A S D        Walk (hold)\nArrow keys     Look left / right / up / down\nSpace          Jump / fly up\nF / C          Toggle flight / fly down\nE / R          Mine / place at the crosshair\n1 - 8 / Wheel  Select an unlimited block\nLeft / Right mouse button: mine / place\nMiddle mouse drag: look around\nN              Day / night\nHome           Return to spawn, keep your builds\nP / Esc        Pause / resume\n?              Open / close this guide\nQ / Ctrl+C     Quit"))
            .child(label(format!("Reach: 6 blocks. Walk up one-block steps.\nWorld edits last until you quit.\n{}", if world.input.enhanced { "Input: independent held keys" } else { "Input: legacy repeat; multi-key hold unavailable" })).fg(Color::Rgb(169, 183, 142)))
            .child(button("Back to the world").key("resume").on_click(|| Msg::Escape)))
            .floating(OverlayAnchor::Center).size(58, 27)).trap_focus()
    } else {
        base
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => {
            app(World::default()).update(update).view(view).run()?;
        }
        [arg, path] if arg == "--input-log" => {
            use std::{cell::RefCell, io::Write, rc::Rc};
            let log = RefCell::new(std::fs::File::create(path)?);
            let failure = Rc::new(RefCell::new(None));
            let input_failure = failure.clone();
            app(World::default())
                .on_input(move |input| {
                    if let Err(error) = writeln!(log.borrow_mut(), "{input:?}") {
                        *input_failure.borrow_mut() = Some(error);
                        return Some(Msg::Quit);
                    }
                    input_message(input)
                })
                .update(update).view(view).run()?;
            let error = failure.borrow_mut().take();
            if let Some(error) = error {
                return Err(error.into());
            }
        }
        [arg] if arg == "--headless" => verify::smoke()?,
        [arg] if arg == "--benchmark" => verify::benchmark()?,
        [arg, path] if arg == "--snapshot" => verify::snapshot(path, false)?,
        [arg, path, night] if arg == "--snapshot" && night == "--night" => {
            verify::snapshot(path, true)?
        }
        _ => return Err(
            "Usage: rsille-minecraft [--input-log FILE | --headless | --benchmark | --snapshot FILE.svg [--night]]"
                .into(),
        ),
    }
    Ok(())
}
