//! Three selectable raster pets, playing embedded, pre-drawn animation clips.
mod export;
mod options;
mod raster;
mod scene;
mod static_image;

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use options::{animation_args, Options, Output};
use rsille::{
    canvas::{Canvas, Encoding},
    raster::RasterOptions,
    render::{area::Size, chunk::Chunk, raster_output::CellPixels, Builder, Draw, DrawErr, Update},
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
    raster: Option<[raster::Frames; 3]>,
    options: Options,
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
            raster: None,
            options: Options::default(),
        }
    }
}

impl Nursery {
    fn new(options: Options) -> Result<Self, Box<dyn Error>> {
        let raster = if options.output == Output::Quadrant {
            None
        } else {
            // Decode before entering terminal modes; switching pets only selects cached frames.
            Some([
                raster::Frames::new(scene::Character::ChatGpt, options.background)?,
                raster::Frames::new(scene::Character::DeepSeek, options.background)?,
                raster::Frames::new(scene::Character::Claude, options.background)?,
            ])
        };
        Ok(Self {
            character: options.character,
            raster,
            options,
            ..Self::default()
        })
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
    fn draw(&mut self, mut chunk: Chunk) -> Result<Size, DrawErr> {
        let size = chunk.area().size();
        if size.width == 0 || size.height == 0 {
            return Ok(size);
        }
        if let Some(frames) = &self.raster {
            let index = scene::current_frame(self.time, self.action);
            let character = match self.character {
                scene::Character::ChatGpt => 0,
                scene::Character::DeepSeek => 1,
                scene::Character::Claude => 2,
            };
            // Bound native output to roughly 320 physical pixels per axis; Fit
            // preserves square proportions when the viewport is smaller.
            let pixels = self.options.pixels.unwrap_or(CellPixels {
                width: 1,
                height: 2,
            });
            let width = size
                .width
                .min(raster::SIDE.div_ceil(u32::from(pixels.width)) as u16);
            let height = size
                .height
                .min(self.options.height)
                .min(raster::SIDE.div_ceil(u32::from(pixels.height)) as u16);
            let dx = size.width - width;
            let dy = size.height - height;
            chunk
                .shrink(dy / 2, dy - dy / 2, dx / 2, dx - dx / 2)?
                .raster(
                    frames[character].frame(index),
                    RasterOptions {
                        background: self.options.background,
                        ..RasterOptions::default()
                    },
                )?;
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

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "--static") {
        return static_image::command(&args[1..]);
    }
    let (mut options, args) = animation_args(&args)?;
    if !args.is_empty() {
        return export::command(args, options);
    }
    if options.output == Output::Sixel && !cfg!(feature = "sixel") {
        return Err(
            "--sixel requires cargo --features sixel; use --raster for a character preview".into(),
        );
    }
    let (width, height) = crossterm::terminal::size()?;
    // Resolve metrics before any input reader starts, and retain this snapshot.
    let output = options.native_config((width, height))?;
    options.pixels = output.capabilities.cell_pixels;
    let nursery = Nursery::new(options)?;
    let mut builder = Builder::new();
    builder
        .raster_output(output)
        .enable_raw_mode()
        .enable_hide_cursor()
        .frame_limit(24);
    if options.output == Output::Sixel {
        builder.size((width, height)).enable_alt_screen();
    } else {
        builder
            .size((width, height.min(options.height)))
            .inline_mode(true)
            .inline_max_height(options.height);
    }
    builder.build_event_loop(nursery).run()?;
    println!();
    Ok(())
}

#[cfg(test)]
mod raster_tests;
#[cfg(test)]
mod tests;
