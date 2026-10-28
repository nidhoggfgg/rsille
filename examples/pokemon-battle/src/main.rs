mod battle;
mod battle_ui;
mod environment;
mod export;
mod geometry;
mod glass;
mod math;
mod raster;
mod scene;

use battle::{Battle, Phase};
use raster::{Camera, Raster};
use rsille::{render::area::Size, tui::prelude::*};
use std::{cell::RefCell, error::Error, rc::Rc};

#[derive(Clone)]
struct Game {
    battle: Battle,
    camera: Camera,
    paused: bool,
    encoding: Encoding,
    viewport: Size,
    last_move: usize,
    // A drawing cache, not application or interaction state.
    raster: Rc<RefCell<Raster>>,
    backdrop: glass::SharedBackdrop,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            battle: Battle::default(),
            camera: Camera::default(),
            paused: false,
            encoding: Encoding::HalfBlock,
            viewport: (160, 50).into(),
            last_move: 0,
            raster: Rc::new(RefCell::new(Raster::default())),
            backdrop: Default::default(),
        }
    }
}
impl Game {
    fn available(&self, index: usize) -> bool {
        self.battle.phase == Phase::Choose
            && !self.paused
            && self.battle.pp[index] > 0
            && !(index == 3 && self.battle.paralyzed)
    }
    fn orbit(&mut self, x: f32, y: f32) {
        self.camera.yaw += x;
        self.camera.pitch = (self.camera.pitch + y).clamp(0.10, 1.15);
    }
}

#[derive(Clone, Copy)]
enum Msg {
    Frame(FrameInfo),
    Use(usize),
    Pause,
    Rematch,
    Orbit(f32, f32),
    Zoom(f32),
    ResetCamera,
    Encoding,
    Resize(u16, u16),
    Quit,
    Ignore,
}

fn input(event: InputEvent) -> Option<Msg> {
    match event {
        InputEvent::Terminal(Event::Resize(w, h)) => Some(Msg::Resize(w, h)),
        InputEvent::Terminal(Event::Mouse(mouse)) => match mouse.kind {
            MouseEventKind::ScrollUp => Some(Msg::Zoom(-0.4)),
            MouseEventKind::ScrollDown => Some(Msg::Zoom(0.4)),
            _ => None,
        },
        InputEvent::Terminal(Event::Key(key)) if key.kind != KeyEventKind::Release => {
            let once = key.kind == KeyEventKind::Press;
            Some(match key.code {
                KeyCode::Esc | KeyCode::Char('q') => Msg::Quit,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Msg::Quit,
                KeyCode::Char(n @ '1'..='4') if once => Msg::Use((n as u8 - b'1') as usize),
                KeyCode::Char('p') if once => Msg::Pause,
                KeyCode::Char('r') if once => Msg::Rematch,
                KeyCode::Char('b') if once => Msg::Encoding,
                KeyCode::Char('c') if once => Msg::ResetCamera,
                KeyCode::Char('a') => Msg::Orbit(-0.12, 0.0),
                KeyCode::Char('d') => Msg::Orbit(0.12, 0.0),
                KeyCode::Char('w') => Msg::Orbit(0.0, 0.07),
                KeyCode::Char('s') => Msg::Orbit(0.0, -0.07),
                KeyCode::Char('+') | KeyCode::Char('=') => Msg::Zoom(-0.4),
                KeyCode::Char('-') => Msg::Zoom(0.4),
                _ => return None,
            })
        }
        _ => None,
    }
}

fn app(game: Game) -> App<Game, Msg> {
    App::new(game)
        .disable_quit_key()
        .focus_events(true)
        .with_theme(battle_ui::theme())
        .on_frame(Msg::Frame)
        .on_input(input)
}

fn update(game: &mut Game, msg: Msg, ctx: &mut UpdateCtx<Msg>) {
    match msg {
        Msg::Frame(frame) if !game.paused => {
            game.battle.advance(frame.delta.as_secs_f64().min(0.1))
        }
        Msg::Use(index) if !game.paused => {
            if game.battle.choose(index) {
                game.last_move = index;
            }
        }
        Msg::Pause => game.paused = !game.paused,
        Msg::Rematch => {
            game.battle = Battle::default();
            game.paused = false;
            game.last_move = 0;
        }
        Msg::Orbit(x, y) => game.orbit(x, y),
        Msg::Zoom(delta) => game.camera.distance = (game.camera.distance + delta).clamp(7.5, 16.0),
        Msg::ResetCamera => game.camera = Camera::default(),
        Msg::Encoding => {
            game.encoding = if game.encoding == Encoding::HalfBlock {
                Encoding::Braille
            } else {
                Encoding::HalfBlock
            }
        }
        Msg::Resize(w, h) => game.viewport = (w, h).into(),
        Msg::Quit => ctx.exit(),
        _ => {}
    }
}

fn view(game: &Game) -> impl Widget<Msg> {
    battle_ui::view(game)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        return export::command(&args);
    }
    app(Game {
        viewport: crossterm::terminal::size()?.into(),
        ..Game::default()
    })
    .update(update)
    .view(view)
    .run()?;
    Ok(())
}

#[cfg(test)]
mod tests;
