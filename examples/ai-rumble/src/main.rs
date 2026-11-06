//! A self-playing chibi battle using rsille's public drawing API.
mod characters;
mod export;
mod motion;
mod raster;
mod scene;

use std::{error::Error, time::Instant};

use crossterm::{
    event::{Event, KeyCode, KeyEventKind, KeyModifiers},
    style::Color,
};
use rsille::{
    canvas::{Canvas, Encoding},
    render::{area::Size, chunk::Chunk, Builder, Draw, DrawErr, Update},
};

const INLINE_HEIGHT: u16 = 32;
const PREVIEW_WIDTH: u16 = 128;

struct Battle {
    time: f64,
    last: Option<Instant>,
    quit: bool,
    scene: scene::Scene,
    canvas: Canvas,
}

impl Default for Battle {
    fn default() -> Self {
        Self {
            time: 0.0,
            last: None,
            quit: false,
            scene: scene::Scene::default(),
            canvas: Canvas::with_encoding(Encoding::HalfBlock),
        }
    }
}

impl Update for Battle {
    fn on_events(&mut self, events: &[Event]) -> Result<(), DrawErr> {
        for event in events {
            if let Event::Key(key) = event {
                if key.kind == KeyEventKind::Press
                    && (matches!(key.code, KeyCode::Esc | KeyCode::Char('q'))
                        || (key.code == KeyCode::Char('c')
                            && key.modifiers.contains(KeyModifiers::CONTROL)))
                {
                    self.quit = true;
                }
            }
        }
        Ok(())
    }

    fn update(&mut self) -> Result<bool, DrawErr> {
        let now = Instant::now();
        if let Some(last) = self.last {
            // Resume gently after terminal suspension instead of jumping ahead.
            self.time += now.duration_since(last).as_secs_f64().min(0.1);
        }
        self.last = Some(now);
        Ok(true)
    }

    fn should_quit(&self) -> bool {
        self.quit
    }
}

impl Draw for Battle {
    fn draw(&mut self, chunk: Chunk) -> Result<Size, DrawErr> {
        let size = chunk.area().size();
        if size.width == 0 || size.height == 0 {
            return Ok(size);
        }
        let w = size.width as usize;
        let h = size.height as usize * 2;
        // Two samples per axis smooth thin tentacles at terminal resolution.
        self.scene.render(w * 2, h * 2, self.time);
        self.canvas.clear();
        self.canvas.set_background(Color::Black);
        self.canvas
            .set_bound((0, size.width as i32 - 1), (0, size.height as i32 - 1));
        for y in 0..h {
            for x in 0..w {
                let mut rgb = [0.0; 3];
                for dy in 0..2 {
                    for dx in 0..2 {
                        let pixel = self.scene.raster.pixels[(y * 2 + dy) * w * 2 + x * 2 + dx];
                        for c in 0..3 {
                            rgb[c] += pixel[c] * 0.25;
                        }
                    }
                }
                let [r, g, b] = rgb.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
                self.canvas
                    .set_color(x as f64, (h - 1 - y) as f64, Color::Rgb { r, g, b });
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
        .frame_limit(60)
        .build_event_loop(Battle::default())
        .run()?;
    println!();
    Ok(())
}

#[cfg(test)]
mod tests;
