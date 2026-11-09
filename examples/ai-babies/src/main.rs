//! Three selectable raster pets, playing embedded, pre-drawn animation clips.
mod export;
mod scene;
mod static_image;

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use rsille::{
    canvas::{Canvas, Encoding},
    render::{area::Size, chunk::Chunk, Builder, Draw, DrawErr, Update},
};
use std::{error::Error, time::Instant};

const INLINE_HEIGHT: u16 = 24;
const PREVIEW_WIDTH: u16 = 80;

struct Nursery {
    time: f64,
    last: Option<Instant>,
    quit: bool,
    paused: bool,
    action: Option<scene::Action>,
    character: scene::Character,
    canvas: Canvas,
}

impl Default for Nursery {
    fn default() -> Self {
        Self {
            time: 0.0,
            last: None,
            quit: false,
            paused: false,
            action: None,
            character: scene::Character::default(),
            canvas: Canvas::with_encoding(Encoding::Quadrant),
        }
    }
}

impl Update for Nursery {
    fn on_events(&mut self, events: &[Event]) -> Result<(), DrawErr> {
        for event in events {
            if let Event::Key(key) = event {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => self.quit = true,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.quit = true
                    }
                    KeyCode::Tab | KeyCode::Char('g' | 'd' | 'c') => {
                        self.character = match key.code {
                            KeyCode::Char('g') => scene::Character::ChatGpt,
                            KeyCode::Char('d') => scene::Character::DeepSeek,
                            KeyCode::Char('c') => scene::Character::Claude,
                            _ => self.character.next(),
                        };
                        self.time = 0.0;
                        self.last = None;
                    }
                    KeyCode::Char(' ') => {
                        self.paused = !self.paused;
                        self.last = None;
                    }
                    KeyCode::Char(n @ '0'..='6') => {
                        self.action = n
                            .to_digit(10)
                            .and_then(|n| n.checked_sub(1).map(|i| scene::Action::ALL[i as usize]));
                        self.time = 0.0;
                        self.last = None;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn update(&mut self) -> Result<bool, DrawErr> {
        let now = Instant::now();
        if !self.paused {
            if let Some(last) = self.last {
                let period = self.action.map_or(scene::PERIOD, scene::Action::duration);
                self.time = (self.time + now.duration_since(last).as_secs_f64().min(0.1))
                    .rem_euclid(period);
            }
        }
        self.last = Some(now);
        Ok(true)
    }

    fn should_quit(&self) -> bool {
        self.quit
    }
}

impl Draw for Nursery {
    fn draw(&mut self, chunk: Chunk) -> Result<Size, DrawErr> {
        let size = chunk.area().size();
        if size.width == 0 || size.height == 0 {
            return Ok(size);
        }
        let w = size.width as usize * 2;
        let h = size.height as usize * 2;
        let pixels = scene::render(w, h, self.time, self.action, self.character);
        self.canvas.clear();
        self.canvas.clear_background();
        self.canvas
            .set_bound((0, size.width as i32 - 1), (0, size.height as i32 - 1));
        for y in 0..h {
            for x in 0..w {
                if let Some(rgb) = pixels[y * w + x] {
                    self.canvas
                        .set_color(x as f64, (h - 1 - y) as f64, scene::color(rgb));
                }
            }
        }
        self.canvas.draw(chunk)?;
        Ok(size)
    }
}

fn animation_args(
    mut args: &[String],
) -> Result<(scene::Character, u16, &[String]), Box<dyn Error>> {
    let mut character = scene::Character::default();
    let mut height = INLINE_HEIGHT;
    while let Some(arg) = args.first() {
        match arg.as_str() {
            "--character" => {
                let name = args
                    .get(1)
                    .ok_or("--character requires chatgpt/deepseek/claude")?;
                character = scene::Character::parse(name)?;
            }
            "--height" => {
                height = args
                    .get(1)
                    .ok_or("--height requires ROWS (24 or 32)")?
                    .parse()?;
                if ![24, 32].contains(&height) {
                    return Err("--height must be 24 or 32 rows".into());
                }
            }
            _ => break,
        }
        args = &args[2..];
    }
    Ok((character, height, args))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "--static") {
        return static_image::command(&args[1..]);
    }
    let (character, requested_height, args) = animation_args(&args)?;
    if !args.is_empty() {
        return export::command(args, character, requested_height);
    }
    let (width, height) = crossterm::terminal::size()?;
    Builder::new()
        .size((width, height.min(requested_height)))
        .inline_mode(true)
        .inline_max_height(requested_height)
        .enable_raw_mode()
        .enable_hide_cursor()
        .frame_limit(24)
        .build_event_loop(Nursery {
            character,
            ..Nursery::default()
        })
        .run()?;
    println!();
    Ok(())
}

#[cfg(test)]
mod tests;
