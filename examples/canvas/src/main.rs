//! Canvas debugging with character drawing and native raster windows.
//! Run `cargo run --release --manifest-path examples/canvas/Cargo.toml` to try both backends.
//! Use `--backend sixel` or `--backend kitty` to test a specific protocol.

use std::{error::Error, sync::Arc};

use rsille::{
    raster::RasterCell,
    render::{area::Area, chunk::Chunk, style::Stylized},
    tui::prelude::*,
};

const BACKGROUND: Rgb = Rgb(8, 13, 25);
const CYAN: Color = Color::Rgb(92, 218, 230);
const AMBER: Color = Color::Rgb(255, 191, 115);
const HELP: &str = "Canvas debug — character drawing and native raster windows

  cargo run --manifest-path examples/canvas/Cargo.toml -- --basic
  cargo run --release --manifest-path examples/canvas/Cargo.toml
  cargo run --release --manifest-path examples/canvas/Cargo.toml -- --backend kitty
  cargo run --manifest-path examples/canvas/Cargo.toml -- --backend sixel
  cargo run --manifest-path examples/canvas/Cargo.toml -- --headless

  --backend auto|kitty|sixel  Auto prefers Kitty, then Sixel, among compiled backends.
  --cell-pixels WxH           Measured physical pixels per character cell.
  --force-cell-pixels         Confirm a measurement equal to the terminal grid.
  --basic                     Print a sine wave using the character Canvas.
  --headless                  Exercise window interactions in memory.
  --help                      Show this guide.

The header shows the active protocol and cell size. Drag one title across the
other window. The foreground border and text should cover the planet. Move
the window away to check that the hidden image and desktop reappear. Click
either window to raise it. Drag the bottom grip to check image fitting during
resize. Tab focuses a title, arrows move it, and Shift+arrows resize it.
e cycles character encoding, r restores the windows, and q or Esc exits.

Unix (including WSL) queries protocol support once, even with explicit metrics.
Cell size comes from the override, OS window metrics or CSI 16 t, in that order.
On native Windows, specify a host-verified backend and measured cell size.
Fullscreen Sixel requires ED 2 graphics erasure, stable palettes, transparent
band padding and positioned rendering without scrolling.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scene {
    Windows,
    Basic,
    Headless,
}

#[derive(Debug)]
struct Options {
    backend: RasterBackend,
    pixels: Option<CellPixels>,
    force_cell_pixels: bool,
    scene: Scene,
    help: bool,
}

impl Options {
    fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut options = Self {
            backend: RasterBackend::Auto,
            pixels: None,
            force_cell_pixels: false,
            scene: Scene::Windows,
            help: false,
        };
        let mut args = arguments.into_iter();
        while let Some(argument) = args.next() {
            match argument.as_str() {
                "--backend" => {
                    options.backend = match args.next().as_deref() {
                        Some("auto") => RasterBackend::Auto,
                        Some("sixel") => RasterBackend::Sixel,
                        Some("kitty") => RasterBackend::Kitty,
                        _ => return Err("--backend expects auto, kitty or sixel".into()),
                    };
                }
                "--cell-pixels" => {
                    let value = args.next().ok_or(
                        "--cell-pixels needs WxH: physical pixels per character cell, not terminal columns x rows; use --help",
                    )?;
                    let (width, height) = value.split_once('x').ok_or(
                        "cell pixels must be WxH: physical pixels per character cell; use --help",
                    )?;
                    let width = width.parse().map_err(|_| "invalid cell pixel width")?;
                    let height = height.parse().map_err(|_| "invalid cell pixel height")?;
                    options.pixels = Some(CellPixels { width, height });
                }
                "--force-cell-pixels" => options.force_cell_pixels = true,
                "--basic" | "--headless" => {
                    let scene = if argument == "--basic" {
                        Scene::Basic
                    } else {
                        Scene::Headless
                    };
                    if options.scene != Scene::Windows && options.scene != scene {
                        return Err("--basic and --headless select different scenes".into());
                    }
                    options.scene = scene;
                }
                "--help" | "-h" => options.help = true,
                _ => return Err(format!("unknown argument: {argument}; use --help")),
            }
        }

        if options.force_cell_pixels && options.pixels.is_none() {
            return Err("--force-cell-pixels requires --cell-pixels WxH".into());
        }
        Ok(options)
    }

    fn measured_pixels(&self, grid: (u16, u16)) -> Result<Option<CellPixels>, String> {
        let Some(pixels) = self.pixels else {
            return Ok(None);
        };
        let pixels = CellPixels::resolve(Some(pixels)).map_err(|e| e.to_string())?;
        if (pixels.width, pixels.height) == grid && !self.force_cell_pixels {
            return Err(format!(
                "--cell-pixels {}x{} matches the terminal grid ({} columns x {} rows); this looks like columns x rows rather than physical pixels per character cell. Omit --cell-pixels to use automatic detection, or supply measured physical pixels per character cell (text-area pixels / columns and rows). If this unusual cell size is intentional, confirm it with --force-cell-pixels",
                pixels.width, pixels.height, grid.0, grid.1,
            ));
        }

        Ok(Some(pixels))
    }

    fn session(&self, grid: (u16, u16)) -> Result<RasterSession, String> {
        let pixels = self.measured_pixels(grid)?;
        #[cfg(unix)]
        {
            self.backend
                .negotiate_fullscreen(pixels, std::time::Duration::from_millis(500))
                .map_err(|error| format!("{:?} raster startup: {error}", self.backend))
        }
        #[cfg(not(unix))]
        {
            let pixels = pixels.ok_or("supply measured cell dimensions with --cell-pixels WxH")?;
            match self.backend {
                RasterBackend::Auto => Err("automatic negotiation requires Unix or WSL; specify a verified --backend kitty or sixel".into()),
                RasterBackend::Kitty if cfg!(feature = "kitty") => Ok(RasterSession::KittyFullscreen(pixels)),
                RasterBackend::Sixel if cfg!(feature = "sixel") => Ok(RasterSession::SixelFullscreen(pixels)),
                backend => Err(format!("the {backend:?} backend was not compiled")),
            }
        }
    }
}

struct State {
    session: Option<RasterSession>,
    image: Arc<RasterCanvas>,
    windows: [WindowRect; 2],
    initial: [WindowRect; 2],
    encoding: Encoding,
}

#[derive(Debug)]
enum Msg {
    Move(usize, WindowRect),
    CycleEncoding,
    Reset,
    Quit,
}

impl State {
    fn new(width: u16, height: u16, session: Option<RasterSession>) -> Result<Self, RasterError> {
        let available_height = height.saturating_sub(4);
        let w = ((u32::from(width) * 3 / 5) as u16)
            .clamp(24, 58)
            .min(width.saturating_sub(2));
        let h = ((u32::from(available_height) * 3 / 4) as u16)
            .clamp(10, 24)
            .min(available_height.saturating_sub(2));
        let windows = [
            WindowRect::new(2, 1, w, h),
            WindowRect::new(
                i32::from(width.saturating_sub(w + 2)),
                i32::from(available_height.saturating_sub(h + 1)),
                w,
                h,
            ),
        ];
        Ok(Self {
            session,
            image: Arc::new(planet()?),
            windows,
            initial: windows,
            encoding: Encoding::HalfBlock,
        })
    }
}

fn update(state: &mut State, message: Msg, ctx: &mut UpdateCtx<Msg>) {
    match message {
        Msg::Move(index, rect) => state.windows[index] = rect,
        Msg::CycleEncoding => {
            state.encoding = match state.encoding {
                Encoding::HalfBlock => Encoding::Braille,
                Encoding::Braille => Encoding::Quadrant,
                _ => Encoding::HalfBlock,
            }
        }
        Msg::Reset => state.windows = state.initial,
        Msg::Quit => ctx.exit(),
    }
}

fn planet() -> Result<RasterCanvas, RasterError> {
    let mut image = RasterCanvas::new(320, 200, BACKGROUND)?;
    for y in 0..200 {
        for x in 0..320 {
            let px = (f64::from(x) - 159.5) / 83.0;
            let py = (99.5 - f64::from(y)) / 83.0;
            let mut color = [8.0, 13.0, 25.0];
            let halo = (-1.8 * (px * px + py * py)).exp();
            for (channel, glow) in color.iter_mut().zip([12.0, 22.0, 34.0]) {
                *channel += halo * glow;
            }
            let radius = 0.66;
            let sphere = px * px + py * py < radius * radius;
            if sphere {
                let nx = px / radius;
                let ny = py / radius;
                let nz = (1.0 - nx * nx - ny * ny).sqrt();
                let light = (-0.45 * nx + 0.55 * ny + 0.70 * nz).max(0.0);
                let bands = (ny * 28.0 + (nx * 7.0).sin() * 0.8).sin();
                let fine = (ny * 81.0 + nx * 8.0).sin();
                let shade = 0.18 + 0.82 * light;
                color = [
                    (209.0 + 25.0 * bands + 8.0 * fine) * shade,
                    (149.0 + 29.0 * bands + 6.0 * fine) * shade,
                    (105.0 + 21.0 * bands + 5.0 * fine) * shade,
                ];
            }
            let u = px * 0.94 + py * 0.342;
            let v = -px * 0.342 + py * 0.94;
            let ring = (u * u + (v / 0.30).powi(2)).sqrt();
            if (0.84..1.37).contains(&ring) && (!sphere || v < 0.0) {
                let groove = 0.5 + 0.5 * (ring * 110.0).sin();
                let shade = if (1.10..1.15).contains(&ring) {
                    0.23
                } else {
                    0.66 + groove * 0.34
                };
                color = [218.0 * shade, 184.0 * shade, 137.0 * shade];
            }
            image.set_pixel(x, y, Rgb(color[0] as u8, color[1] as u8, color[2] as u8));
        }
    }
    // Deterministic stars stay well outside the ring silhouette.
    for i in 0..46 {
        let x = (i * 73 + 19) % 312 + 4;
        let y = (i * 47 + 11) % 192 + 4;
        let dx = (f64::from(x) - 159.5) / 83.0;
        let dy = (99.5 - f64::from(y)) / 83.0;
        if dx * dx + dy * dy > 1.65 {
            let brightness = 105 + (i * 31 % 145) as u8;
            image.set_pixel(x, y, Rgb(brightness, brightness, 245));
            if i % 9 == 0 {
                image.line(x - 2, y, x + 2, y, Rgb(70, 92, 135));
                image.line(x, y - 2, x, y + 2, Rgb(70, 92, 135));
                image.set_pixel(x, y, Rgb(240, 245, 255));
            }
        }
    }
    Ok(image)
}

/// Ordinary Canvas drawing: it never attaches native raster metadata to its cells.
fn draw_canvas(surface: &mut rsille::canvas::Canvas, ctx: CanvasContext, image: &RasterCanvas) {
    let (sx, sy) = ctx.encoding().samples_per_cell();
    let mut sample = RasterCell {
        canvas: image.clone(),
        target: (ctx.cell_width(), ctx.cell_height()).into(),
        x: 0,
        y: 0,
        options: RasterOptions {
            background: BACKGROUND,
            ..Default::default()
        },
    };
    for y in 0..ctx.sample_height() {
        for x in 0..ctx.sample_width() {
            sample.x = (x / i32::from(sx)) as u16;
            sample.y = (y / i32::from(sy)) as u16;
            let Rgb(r, g, b) = sample.sample(
                (x % i32::from(sx)) as u32,
                (y % i32::from(sy)) as u32,
                u16::from(sx),
                u16::from(sy),
            );
            // Raster's y axis points down; Canvas's y axis points up.
            surface.set_color(x, ctx.sample_height() - 1 - y, Color::Rgb(r, g, b));
        }
    }
}

struct Desktop;
impl Widget<Msg> for Desktop {
    fn constraints(&self) -> Constraints {
        Constraints::fill()
    }
    fn render(&self, chunk: &mut Chunk, _: &RenderCtx) {
        let style = rsille::render::style::Style::with_colors(crossterm::style::Colors {
            foreground: Some(crossterm::style::Color::Rgb {
                r: 31,
                g: 48,
                b: 66,
            }),
            background: Some(crossterm::style::Color::Rgb { r: 5, g: 9, b: 17 }),
        });
        for y in 0..chunk.area().height() {
            for x in 0..chunk.area().width() {
                let symbol = if x % 6 == 0 && y % 3 == 0 { '·' } else { ' ' };
                let _ = chunk.set_forced(x, y, Stylized::new(symbol, style));
            }
        }
    }
}

fn view(state: &State) -> impl Widget<Msg> {
    let source = Arc::clone(&state.image);

    let backend = match state.session {
        Some(RasterSession::KittyFullscreen(p)) => {
            format!("Kitty | {}x{} px/cell", p.width, p.height)
        }
        Some(RasterSession::SixelFullscreen(p)) => {
            format!("Sixel | {}x{} px/cell", p.width, p.height)
        }
        None => "headless".to_owned(),
    };
    let window_style = |accent| Style::default().fg(accent).bg(Color::Rgb(8, 13, 25));
    let native = floating_window(
        col()
            .child(
                raster_canvas(Arc::clone(&state.image))
                    .key("raster")
                    .background(BACKGROUND),
            )
            .child(label(" 320 x 200 RGB / fixed source / fit").style(window_style(CYAN))),
    )
    .key("native")
    .title("RasterCanvas / RGB")
    .title_key("native-title")
    .rect(state.windows[0])
    .on_rect_change(|rect| Msg::Move(0, rect))
    .min_size(22, 10)
    .max_size(60, 28)
    .collapsible(false)
    .style(window_style(Color::Rgb(65, 124, 146)))
    .active_style(window_style(CYAN));
    let characters = floating_window(
        col()
            .child(
                canvas(move |surface, ctx| draw_canvas(surface, ctx, &source))
                    .key("canvas")
                    .encoding(state.encoding),
            )
            .child(
                label(format!(
                    " {:?} / samples per cell {:?} / e",
                    state.encoding,
                    state.encoding.samples_per_cell()
                ))
                .style(window_style(AMBER)),
            ),
    )
    .key("characters")
    .title(format!("Canvas / {:?}", state.encoding))
    .title_key("canvas-title")
    .rect(state.windows[1])
    .on_rect_change(|rect| Msg::Move(1, rect))
    .min_size(22, 10)
    .max_size(60, 28)
    .collapsible(false)
    .style(window_style(Color::Rgb(146, 109, 70)))
    .active_style(window_style(AMBER));
    col()
        .style(
            Style::default()
                .fg(Color::Rgb(161, 179, 198))
                .bg(Color::Rgb(5, 9, 17)),
        )
        .child(label(format!(" Canvas debug | {backend}")).style(Style::default().fg(CYAN).bold()))
        .child(label(
            " Drag Canvas across the planet, then uncover it and check for trails.",
        ))
        .child(
            window_host(Desktop)
                .key("desktop")
                .snap_distance(1)
                .default_active_window(Some("characters".into()))
                .window(native)
                .window(characters),
        )
        .child(label(
            " Drag title: move | bottom grip: resize | click: raise | e: encoding",
        ))
        .child(label(" r: reset windows | q / Esc: quit"))
}

fn application(
    width: u16,
    height: u16,
    session: Option<RasterSession>,
) -> Result<Runtime<State, Msg>, RasterError> {
    let mut app = App::new(State::new(width, height, session)?);
    if let Some(session) = session {
        app = app.raster_session(session);
    }
    Ok(app
        .mode(Mode::Fullscreen)
        .focus_events(true)
        .with_motion_policy(MotionPolicy::disabled())
        .with_quit_key(KeyCode::Esc)
        .on_key(KeyCode::Char('q'), || Msg::Quit)
        .on_key(KeyCode::Char('e'), || Msg::CycleEncoding)
        .on_key(KeyCode::Char('r'), || Msg::Reset)
        .update(update)
        .view(view)
        .build()
        .size(width, height))
}

fn mouse(kind: MouseEventKind, x: u16, y: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: KeyModifiers::empty(),
    })
}

fn window_path(name: &str) -> WidgetPath {
    WidgetPath::root().child("desktop").child(name)
}

fn drag(
    runtime: &mut Runtime<State, Msg>,
    name: &str,
    title: &str,
    dx: u16,
    dy: u16,
) -> Result<(), rsille::render::DrawErr> {
    let area = runtime
        .bounds(&window_path(name).child(title))
        .expect("visible window title");
    // The far end stays exposed when the other window covers the title's left.
    let (x, y) = (area.x() + area.width().saturating_sub(8), area.y());
    for event in [
        mouse(MouseEventKind::Down(MouseButton::Left), x, y),
        mouse(MouseEventKind::Drag(MouseButton::Left), x + dx, y + dy),
        mouse(MouseEventKind::Up(MouseButton::Left), x + dx, y + dy),
    ] {
        runtime.step(&[event])?;
    }
    Ok(())
}

fn inside(area: Area, x: u16, y: u16) -> bool {
    (area.x()..area.x() + area.width()).contains(&x)
        && (area.y()..area.y() + area.height()).contains(&y)
}

fn headless() -> Result<(), Box<dyn Error>> {
    let mut runtime = application(110, 36, None)?;
    runtime.step(&[])?;
    let initial = runtime.state().windows;
    let identity = runtime.state().image.id();
    let version = runtime.state().image.version();
    assert!(runtime.frame().has_raster());
    drag(&mut runtime, "native", "native-title", 5, 2)?;
    assert_ne!(runtime.state().windows[0], initial[0]);
    let exposed = runtime.frame().clone();
    drag(&mut runtime, "characters", "canvas-title", 3, 1)?;
    assert_ne!(runtime.state().windows[1], initial[1]);
    let covered = runtime.frame().raster_cells().count();
    let foreground = runtime.bounds(&window_path("characters")).unwrap();
    assert!(runtime
        .frame()
        .raster_cells()
        .all(|(x, y, _)| !inside(foreground, x, y)));

    assert!(covered > 0 && covered < exposed.raster_cells().count());
    for (x, y, cell) in exposed.raster_cells() {
        let expected = (!inside(foreground, x, y)).then_some(cell);
        assert_eq!(runtime.frame().raster_at((x, y).into()), expected);
    }

    // Clicking an exposed title restores each cell with its original crop.
    let title = runtime
        .bounds(&window_path("native").child("native-title"))
        .unwrap();
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        runtime.step(&[mouse(kind, title.x() + 3, title.y())])?;
    }
    assert!(exposed.raster_cells().eq(runtime.frame().raster_cells()));

    // The Canvas grip is below the RGB window and remains exposed.
    let grip = runtime
        .bounds(&window_path("characters").child("window-resize"))
        .unwrap();
    let (x, y) = (grip.x() + grip.width() - 2, grip.y());
    let before = runtime.state().windows[1];
    for event in [
        mouse(MouseEventKind::Down(MouseButton::Left), x, y),
        mouse(MouseEventKind::Drag(MouseButton::Left), x - 6, y - 3),
        mouse(MouseEventKind::Up(MouseButton::Left), x - 6, y - 3),
    ] {
        runtime.step(&[event])?;
    }
    assert!(runtime.state().windows[1].width < before.width);
    assert!(runtime.state().windows[1].height < before.height);
    assert_eq!(runtime.state().image.id(), identity);
    assert_eq!(runtime.state().image.version(), version);
    runtime.step(&[Event::Key(KeyCode::Char('e').into())])?;
    assert_eq!(runtime.state().encoding, Encoding::Braille);
    runtime.step(&[Event::Key(KeyCode::Char('r').into())])?;
    assert_eq!(runtime.state().windows, initial);
    assert!(runtime.frame().has_raster());
    println!("Headless checks passed: mouse dragging, resizing, stacking, raster occlusion/restoration, shared source, encoding and reset. Native terminal output was not tested.");
    Ok(())
}

fn basic(out: impl std::io::Write) -> std::io::Result<()> {
    let mut surface = rsille::canvas::Canvas::new();
    for x in 0..1800 {
        let x = f64::from(x);
        surface.set(x / 10.0, 15.0 + x.to_radians().sin() * 10.0);
    }
    rsille::render::Builder::new()
        .size((40, 20))
        .append_newline(true)
        .build_render(surface, out)
        .render()
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse(std::env::args().skip(1))?;
    if options.help {
        println!("{HELP}");
        return Ok(());
    }
    match options.scene {
        Scene::Basic => return Ok(basic(std::io::stdout())?),
        Scene::Headless => return headless(),
        Scene::Windows => {}
    }
    let grid = crossterm::terminal::size()?;
    let session = options.session(grid)?;
    application(grid.0, grid.1, Some(session))?.run()?;
    Ok(())
}

#[cfg(test)]
mod tests;
