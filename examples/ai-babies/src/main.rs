//! Generated raster mascots, embedded in the binary, with quiet idle motion.
mod export;
mod scene;

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use rsille::{
    canvas::{Canvas, Encoding},
    render::{area::Size, chunk::Chunk, Builder, Draw, DrawErr, Update},
};
use std::{error::Error, time::Instant};

const INLINE_HEIGHT: u16 = 32;
const PREVIEW_WIDTH: u16 = 128;

struct Nursery {
    time: f64,
    last: Option<Instant>,
    quit: bool,
    paused: bool,
    focus: Option<usize>,
    canvas: Canvas,
}

impl Default for Nursery {
    fn default() -> Self {
        Self {
            time: 0.0,
            last: None,
            quit: false,
            paused: false,
            focus: None,
            canvas: Canvas::with_encoding(Encoding::HalfBlock),
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
                    KeyCode::Char(' ') => {
                        self.paused = !self.paused;
                        self.last = None;
                    }
                    KeyCode::Char('0') => self.focus = None,
                    KeyCode::Char(n @ '1'..='3') => self.focus = Some(n as usize - '1' as usize),
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
                self.time = (self.time + now.duration_since(last).as_secs_f64().min(0.1))
                    .rem_euclid(scene::PERIOD);
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
        let w = size.width as usize;
        let h = size.height as usize * 2;
        let pixels = scene::render(w, h, self.time, self.focus);
        self.canvas.clear();
        self.canvas.set_background(scene::color(scene::BACKGROUND));
        self.canvas
            .set_bound((0, size.width as i32 - 1), (0, size.height as i32 - 1));
        for y in 0..h {
            for x in 0..w {
                self.canvas.set_color(
                    x as f64,
                    (h - 1 - y) as f64,
                    scene::color(pixels[y * w + x]),
                );
            }
        }
        self.canvas.draw(chunk)?;
        Ok(size)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        return export::command(&args);
    }
    let (width, height) = crossterm::terminal::size()?;
    Builder::new()
        .size((width, height.min(INLINE_HEIGHT)))
        .inline_mode(true)
        .inline_max_height(INLINE_HEIGHT)
        .enable_raw_mode()
        .enable_hide_cursor()
        .frame_limit(24)
        .build_event_loop(Nursery::default())
        .run()?;
    println!();
    Ok(())
}

#[cfg(test)]
mod tests;
