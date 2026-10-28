mod export;
mod machine;
mod math;
mod raster;

use crossterm::{
    event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind},
    style::{Color, Colors},
};
use machine::{Machine, NAMES};
use raster::{Camera, Raster};
use rsille::{
    canvas::{Canvas, Encoding},
    render::{area::Size, chunk::Chunk, style::Style, Builder, Draw, DrawErr, Update},
};
use std::{error::Error, time::Instant};

struct Demo {
    machine: Machine,
    camera: Camera,
    orbit: bool,
    paused: bool,
    clean: bool,
    quit: bool,
    encoding: Encoding,
    last: Option<Instant>,
    drag: Option<(u16, u16)>,
    raster: Raster,
    canvas: Canvas,
}

impl Default for Demo {
    fn default() -> Self {
        Self {
            machine: Machine::default(),
            camera: Camera::default(),
            orbit: true,
            paused: false,
            clean: false,
            quit: false,
            encoding: Encoding::HalfBlock,
            last: None,
            drag: None,
            raster: Raster::default(),
            canvas: Canvas::with_encoding(Encoding::HalfBlock),
        }
    }
}

impl Demo {
    fn advance(&mut self, delta: f64) {
        if !self.paused {
            self.machine.advance(delta);
        }
    }
    fn orbit_by(&mut self, x: f32, y: f32) {
        self.orbit = false;
        self.camera.yaw += x;
        self.camera.pitch = (self.camera.pitch + y).clamp(0.12, 1.15);
    }
}

impl Update for Demo {
    fn on_events(&mut self, events: &[Event]) -> Result<(), DrawErr> {
        for event in events {
            match event {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => self.quit = true,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.quit = true
                    }
                    KeyCode::Char(' ') | KeyCode::Char('p') => self.paused = !self.paused,
                    KeyCode::Char(n @ '1'..='3') => self.machine.select(n as usize - '1' as usize),
                    KeyCode::Char('a') => {
                        self.machine.autoplay = !self.machine.autoplay;
                        if self.machine.autoplay {
                            self.machine.age = self.machine.age.min(10.0);
                        }
                    }
                    KeyCode::Char('k') => self.machine.kick(),
                    KeyCode::Char('x') => self.machine.exploded = !self.machine.exploded,
                    KeyCode::Char('o') => self.orbit = !self.orbit,
                    KeyCode::Char('h') => self.clean = !self.clean,
                    KeyCode::Char('b') => {
                        self.encoding = match self.encoding {
                            Encoding::HalfBlock => Encoding::Braille,
                            _ => Encoding::HalfBlock,
                        };
                        self.canvas = Canvas::with_encoding(self.encoding);
                    }
                    KeyCode::Left => self.orbit_by(-0.14, 0.0),
                    KeyCode::Right => self.orbit_by(0.14, 0.0),
                    KeyCode::Up => self.orbit_by(0.0, 0.08),
                    KeyCode::Down => self.orbit_by(0.0, -0.08),
                    KeyCode::Char('+') | KeyCode::Char('=') => {
                        self.camera.distance = (self.camera.distance - 0.4).max(5.8)
                    }
                    KeyCode::Char('-') => {
                        self.camera.distance = (self.camera.distance + 0.4).min(13.0)
                    }
                    _ => {}
                },
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        self.drag = Some((mouse.column, mouse.row))
                    }
                    MouseEventKind::Drag(MouseButton::Left) => {
                        if let Some((x, y)) = self.drag {
                            self.orbit_by(
                                (mouse.column as f32 - x as f32) * 0.018,
                                (mouse.row as f32 - y as f32) * 0.025,
                            );
                        }
                        self.drag = Some((mouse.column, mouse.row));
                    }
                    MouseEventKind::Up(MouseButton::Left) => self.drag = None,
                    MouseEventKind::Down(MouseButton::Right) => self.machine.kick(),
                    MouseEventKind::ScrollUp => {
                        self.camera.distance = (self.camera.distance - 0.4).max(5.8)
                    }
                    MouseEventKind::ScrollDown => {
                        self.camera.distance = (self.camera.distance + 0.4).min(13.0)
                    }
                    _ => {}
                },
                Event::FocusLost => self.drag = None,
                _ => {}
            }
        }
        Ok(())
    }
    fn update(&mut self) -> Result<bool, DrawErr> {
        let now = Instant::now();
        if let Some(last) = self.last {
            // Avoid a long catch-up stall after a suspended terminal or debugger pause.
            self.advance(now.duration_since(last).as_secs_f64().min(0.1));
        }
        self.last = Some(now);
        Ok(true)
    }
    fn should_quit(&self) -> bool {
        self.quit
    }
}

impl Draw for Demo {
    fn draw(&mut self, mut chunk: Chunk) -> Result<Size, DrawErr> {
        let size = chunk.area().size();
        if size.width == 0 || size.height == 0 {
            return Ok(size);
        }
        let rows = size.height - u16::from(!self.clean);
        if rows > 0 {
            let (sx, sy) = self.encoding.samples_per_cell();
            let (w, h) = (
                size.width as usize * sx as usize,
                rows as usize * sy as usize,
            );
            let mut camera = self.camera;
            if self.orbit {
                camera.yaw += (self.machine.time as f32 * 0.15).sin() * 0.28;
            }
            // Supersample solid blocks so small moving cogs do not flicker between pixels.
            let scale = if self.encoding == Encoding::HalfBlock {
                2
            } else {
                1
            };
            self.raster
                .render(w * scale, h * scale, &self.machine, camera);
            self.canvas.clear();
            self.canvas.set_background(Color::Black);
            self.canvas
                .set_bound((0, size.width as i32 - 1), (0, rows as i32 - 1));
            for y in 0..h {
                for x in 0..w {
                    let mut rgb = [0.0; 3];
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let p = self.raster.pixels
                                [(y * scale + dy) * self.raster.width + x * scale + dx];
                            for c in 0..3 {
                                rgb[c] += p[c] / (scale * scale) as f32;
                            }
                        }
                    }
                    let [r, g, b] = rgb.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8);
                    self.canvas
                        .set_color(x as f64, (h - 1 - y) as f64, Color::Rgb { r, g, b });
                }
            }
            self.canvas
                .draw(chunk.shrink(0, size.height - rows, 0, 0)?)?;
        }
        if !self.clean {
            let style = Style::with_colors(Colors::new(
                Color::Rgb {
                    r: 177,
                    g: 181,
                    b: 172,
                },
                Color::Rgb { r: 6, g: 8, b: 12 },
            ));
            let title = format!(" CLOCKWORK / {} / {}{} | q quit  1-3 form  k pulse  x explode  a auto  drag orbit  SPACE pause",
                NAMES[self.machine.form],self.machine.phase(),if self.paused {" / PAUSED"} else {""});
            chunk.set_string(0, rows, &" ".repeat(size.width as usize), style)?;
            chunk.set_string(0, rows, &title, style)?;
        }
        Ok(size)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        return export::command(&args);
    }
    Builder::new()
        .size(crossterm::terminal::size()?)
        .enable_raw_mode()
        .enable_alt_screen()
        .enable_hide_cursor()
        .enable_mouse_capture()
        .focus_events(true)
        .frame_limit(30)
        .build_event_loop(Demo::default())
        .run()?;
    Ok(())
}

#[cfg(test)]
mod tests;
