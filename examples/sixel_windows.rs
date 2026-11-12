//! Two draggable windows, one native Sixel surface and one ordinary Canvas.
//! Run `cargo run --release --features sixel --example sixel_windows`.
//! Use `--headless` to exercise real mouse events.

use std::{error::Error, sync::Arc};

use rsille::{
    raster::RasterCell,
    render::{area::Area, chunk::Chunk, style::Stylized},
    tui::prelude::*,
};

const BACKGROUND: Rgb = Rgb(8, 13, 25);
const CYAN: Color = Color::Rgb(92, 218, 230);
const AMBER: Color = Color::Rgb(255, 191, 115);
const HELP: &str = "SIXEL / CANVAS — two surfaces, one desktop

Run from the repository root:
  cargo run --release --features sixel --example sixel_windows
  cargo run --example sixel_windows -- --headless

Options:

  --cell-pixels WxH     Skip OS/query discovery: physical pixels per character cell
  --force-cell-pixels   Confirm an override equal to the terminal columns x rows
  --headless            Check dragging, resizing, stacking and restoration in memory
  --help                Show this guide

Drag a title to move; drag the bottom grip to resize; click to bring forward.
Tab focuses a title; arrows move it; Shift+arrows resize it.
e cycles Canvas HalfBlock/Braille/Quadrant; r resets windows; q / Esc quits.
Suggested terminal size: 100 columns by 32 rows (NOT --cell-pixels 100x32).

Cell pixels are the width x height of ONE character cell, not the terminal grid,
window pixels, font point size, or Canvas samples. For example, a 960x648-pixel
text area with 120 columns and 36 rows has 8x18-pixel cells, NOT 120x36.
Omit --cell-pixels to try OS window metrics first. Unavailable, zero, non-integral,
or unsupported OS dimensions trigger CSI 16 t on Unix (including WSL), with a
500 ms absolute timeout. Stdin/stdout must be TTYs on the controlling terminal.
The CSI 6 ; height ; width t reply is pixels per cell directly, not window pixels;
do not divide it by columns/rows. The result is stored once at startup, not refreshed
while running. After font or DPI changes, restart to measure again.
Native Windows has no query fallback; explicit dimensions are still needed there.
If both automatic sources fail, native startup fails: measure text-area pixels /
columns and rows; do not guess 8x18. Explicit overrides skip all OS metric lookup
and terminal queries, and stay fixed after font changes. An override matching the
grid requires --force-cell-pixels; use it only for a verified unusual cell size.
Dimensions must be 1..=1024 with area <=65536 pixels per character cell.
Metric queries do not detect Sixel support.
Startup probing must precede any input reader. It may consume unrelated input;
late replies after timeout may reach the subsequent reader. Runtime input uses
native crossterm. Standalone/embedded hosts can call CellPixels::probe(timeout)
before starting input and store the result as Some(pixels) in their configuration.
Incorrect metrics can distort images and misalign graphics while dragging.

Normal invocation requires native Sixel; it never silently falls back. Build with
--features sixel and use a terminal AND multiplexer that support Sixel, ED 2 graphics
erasure, stable palettes, and non-scrolling positioned output.
Partial final six-pixel bands must preserve transparent padding without scrolling.
Actual cell sizes such as 10x20 are supported; do not round the height to 18 or 24.
No automatic capability detection is performed.
The --headless checks do not certify terminal compatibility.";

#[derive(Default, Debug)]
struct Options {
    pixels: Option<CellPixels>,
    force_cell_pixels: bool,
    headless: bool,
    help: bool,
}

impl Options {
    fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut options = Self::default();
        let mut args = arguments.into_iter();
        while let Some(argument) = args.next() {
            match argument.as_str() {
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
                "--headless" => options.headless = true,
                "--help" | "-h" => options.help = true,
                _ => return Err(format!("unknown argument: {argument}; use --help")),
            }
        }

        if options.force_cell_pixels && options.pixels.is_none() {
            return Err("--force-cell-pixels requires --cell-pixels WxH".into());
        }
        Ok(options)
    }

    fn native_config(&self, grid: (u16, u16)) -> Result<RasterOutput, String> {
        self.native_config_with_resolver(grid, CellPixels::resolve)
    }

    fn native_config_with_resolver(
        &self,
        grid: (u16, u16),
        resolve: impl FnOnce(Option<CellPixels>) -> std::io::Result<CellPixels>,
    ) -> Result<RasterOutput, String> {
        if !cfg!(feature = "sixel") {
            return Err("native output needs --features sixel".into());
        }
        // Resolve once before starting input; rendering only uses this snapshot.
        let pixels = resolve(self.pixels).map_err(|error| error.to_string())?;
        if self.pixels.is_some() && (pixels.width, pixels.height) == grid && !self.force_cell_pixels
        {
            return Err(format!(
                "--cell-pixels {}x{} matches the terminal grid ({} columns x {} rows); this looks like columns x rows rather than physical pixels per character cell. Omit --cell-pixels to use automatic detection, or supply measured physical pixels per character cell (text-area pixels / columns and rows). If this unusual cell size is intentional, confirm it with --force-cell-pixels",
                pixels.width, pixels.height, grid.0, grid.1,
            ));
        }

        Ok(RasterOutput {
            capabilities: RasterCapabilities {
                // Normal invocation requires the terminal capabilities documented above.
                sixel: true,
                reliable_erase: true,
                reliable_inline_erase: false,
                no_scroll: true,
                cell_pixels: Some(pixels),
            },
        })
    }
}

struct State {
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
    fn new(width: u16, height: u16) -> Result<Self, RasterError> {
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
    .key("sixel")
    .title("01 / SIXEL / RGB pixels")
    .title_key("sixel-title")
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
    .title(format!("02 / CANVAS / {:?}", state.encoding))
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
        .child(label(" RSILLE / ORBITAL DESKTOP").style(Style::default().fg(CYAN).bold()))
        .child(label(
            " One scene. Two renderers. Drag the windows across each other.",
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
        .child(label(
            " r: reset windows | q / Esc: quit | RGB source stays cached while moving",
        ))
}

fn application(
    width: u16,
    height: u16,
    config: RasterOutput,
) -> Result<Runtime<State, Msg>, RasterError> {
    Ok(App::new(State::new(width, height)?)
        .mode(Mode::Fullscreen)
        .focus_events(true)
        .with_motion_policy(MotionPolicy::disabled())
        .raster_output(config)
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
    let mut runtime = application(110, 36, RasterOutput::default())?;
    runtime.step(&[])?;
    let initial = runtime.state().windows;
    let identity = runtime.state().image.id();
    let version = runtime.state().image.version();
    assert!(runtime.frame().has_raster());
    drag(&mut runtime, "sixel", "sixel-title", 5, 2)?;
    assert_ne!(runtime.state().windows[0], initial[0]);
    drag(&mut runtime, "characters", "canvas-title", 3, 1)?;
    assert_ne!(runtime.state().windows[1], initial[1]);
    let covered = runtime.frame().raster_cells().count();
    let foreground = runtime.bounds(&window_path("characters")).unwrap();
    assert!(runtime
        .frame()
        .raster_cells()
        .all(|(x, y, _)| !inside(foreground, x, y)));

    // Clicking an exposed title raises the RGB window and restores its pixels.
    let title = runtime
        .bounds(&window_path("sixel").child("sixel-title"))
        .unwrap();
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        runtime.step(&[mouse(kind, title.x() + 3, title.y())])?;
    }
    assert!(runtime.frame().raster_cells().count() > covered);

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

fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse(std::env::args().skip(1))?;
    if options.help {
        println!("{HELP}");
        return Ok(());
    }
    if options.headless {
        return headless();
    }
    let (width, height) = crossterm::terminal::size()?;
    let config = options.native_config((width, height))?;
    application(width, height, config)?.run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(args: &[&str]) -> Result<Options, String> {
        Options::parse(args.iter().map(|arg| arg.to_string()))
    }

    #[test]
    fn native_is_default_and_obsolete_flags_are_rejected() {
        let defaults = options(&[]).unwrap();
        assert!(!defaults.help);
        assert!(!defaults.headless);
        assert_eq!(defaults.pixels, None);
        assert!(options(&["--text-only"]).is_err());
        assert_eq!(
            options(&["--sixel"]).unwrap_err(),
            "unknown argument: --sixel; use --help"
        );
        assert!(options(&["--cell-pixels", "8x18"]).is_ok());
        assert!(options(&["--cell-pixels", "8"]).is_err());
        assert!(options(&["--cell-pixels"]).is_err());
        assert!(options(&["--unknown"]).is_err());
    }

    #[test]
    fn help_and_headless_remain_available() {
        assert!(options(&["--help"]).unwrap().help);
        assert!(options(&["-h"]).unwrap().help);
        assert!(options(&["--headless"]).unwrap().headless);
        assert!(HELP.contains("--features sixel"));
        assert!(HELP.contains("Normal invocation requires native Sixel"));
        assert!(!HELP.contains("--sixel"));
    }

    #[test]
    fn inline_argument_is_rejected() {
        for args in [
            vec!["--inline"],
            vec!["--cell-pixels", "8x18", "--inline"],
            vec!["--headless", "--inline"],
        ] {
            assert_eq!(
                options(&args).unwrap_err(),
                "unknown argument: --inline; use --help"
            );
        }
    }

    #[test]
    fn native_mode_never_silently_falls_back() {
        let valid = options(&["--cell-pixels", "10x20"])
            .unwrap()
            .native_config((110, 36));
        if cfg!(feature = "sixel") {
            let config = valid.unwrap();
            assert_eq!(
                config.capabilities.cell_pixels,
                Some(CellPixels {
                    width: 10,
                    height: 20
                })
            );
        } else {
            assert!(valid.is_err());
        }
        for value in ["0x18", "8x0", "1025x18", "8x1025", "257x256", "1024x1020"] {
            assert!(options(&["--cell-pixels", value])
                .unwrap()
                .native_config((110, 36))
                .is_err());
        }
        assert!(options(&[])
            .unwrap()
            .native_config_with_resolver((110, 36), |explicit| {
                assert_eq!(explicit, None);
                Err(std::io::Error::other("OS cell pixels unavailable (test)"))
            })
            .is_err());
    }

    #[test]
    fn forcing_grid_sized_cells_requires_an_explicit_override() {
        assert_eq!(
            options(&["--force-cell-pixels"]).unwrap_err(),
            "--force-cell-pixels requires --cell-pixels WxH"
        );
        for args in [
            vec!["--cell-pixels", "120x36", "--force-cell-pixels"],
            vec!["--force-cell-pixels", "--cell-pixels", "120x36"],
        ] {
            assert!(options(&args).unwrap().force_cell_pixels);
        }
    }

    #[cfg(feature = "sixel")]
    #[test]
    fn terminal_grid_is_not_a_cell_pixel_measurement() {
        let opts = options(&["--cell-pixels", "120x36"]).unwrap();
        let error = opts.native_config((120, 36)).unwrap_err();
        assert!(error.contains("120 columns x 36 rows"));
        assert!(error.contains("physical pixels per character cell"));
        assert!(error.contains("Omit --cell-pixels"));
        assert!(error.contains("--force-cell-pixels"));
        // Detection is based on the current grid, not a hardcoded 120x36 ban.
        let error = options(&["--cell-pixels", "80x24"])
            .unwrap()
            .native_config((80, 24))
            .unwrap_err();
        assert!(error.contains("80 columns x 24 rows"));
        assert!(opts.native_config((100, 32)).is_ok());
    }

    #[cfg(feature = "sixel")]
    #[test]
    fn unusual_grid_sized_cells_can_be_confirmed() {
        let pixels = CellPixels {
            width: 120,
            height: 36,
        };
        let forced = options(&["--cell-pixels", "120x36", "--force-cell-pixels"]).unwrap();
        assert_eq!(
            forced
                .native_config((120, 36))
                .unwrap()
                .capabilities
                .cell_pixels,
            Some(pixels),
        );
        // Confirmation bypasses only the grid ambiguity, never backend limits.
        for (value, grid) in [
            ("0x18", (0, 18)),
            ("8x0", (8, 0)),
            ("1025x18", (1025, 18)),
            ("8x1025", (8, 1025)),
            ("257x256", (257, 256)),
            ("1024x1020", (1024, 1020)),
        ] {
            assert!(options(&["--cell-pixels", value, "--force-cell-pixels"])
                .unwrap()
                .native_config(grid)
                .is_err());
        }
    }

    #[cfg(feature = "sixel")]
    #[test]
    fn explicit_metrics_stay_authoritative_without_aspect_ratio_rules() {
        for value in [
            "8x16", "8x18", "9x18", "10x20", "18x18", "36x6", "120x36", "1x1024", "1024x1",
            "256x256", "1024x64",
        ] {
            let opts = options(&["--cell-pixels", value]).unwrap();
            assert_eq!(
                opts.native_config((110, 36))
                    .unwrap()
                    .capabilities
                    .cell_pixels,
                opts.pixels,
            );
        }
    }

    #[cfg(feature = "sixel")]
    #[test]
    fn startup_metrics_are_validated_and_stored_in_configuration() {
        let opts = options(&[]).unwrap();
        for pixels in [
            CellPixels {
                width: 10,
                height: 20,
            },
            CellPixels {
                width: 8,
                height: 18,
            },
            CellPixels {
                width: 120,
                height: 36,
            },
        ] {
            let config = opts
                .native_config_with_resolver((120, 36), |explicit| {
                    assert_eq!(explicit, None);
                    CellPixels::resolve(Some(pixels))
                })
                .unwrap();
            assert_eq!(config.capabilities.cell_pixels, Some(pixels));
        }
        let error = opts
            .native_config_with_resolver((120, 36), |explicit| {
                assert_eq!(explicit, None);
                Err(std::io::Error::other("OS cell pixels unavailable (test)"))
            })
            .unwrap_err();
        assert_eq!(error, "OS cell pixels unavailable (test)");
        for pixels in [
            CellPixels {
                width: 0,
                height: 20,
            },
            CellPixels {
                width: 10,
                height: 0,
            },
            CellPixels {
                width: 1025,
                height: 20,
            },
            CellPixels {
                width: 10,
                height: 1025,
            },
            CellPixels {
                width: 257,
                height: 256,
            },
        ] {
            let expected = CellPixels::resolve(Some(pixels)).unwrap_err().to_string();
            let error = opts
                .native_config_with_resolver((120, 36), |explicit| {
                    assert_eq!(explicit, None);
                    CellPixels::resolve(Some(pixels))
                })
                .unwrap_err();
            assert_eq!(error, expected);
        }
        assert_eq!(opts.pixels, None);
    }

    #[test]
    fn two_windows_really_drag_and_reuse_source() {
        headless().unwrap();
    }

    #[test]
    fn resize_and_encoding_changes_keep_both_renderers_visible() {
        let mut runtime = application(110, 36, RasterOutput::default()).unwrap();
        for size in [(80, 24), (48, 16), (110, 36)] {
            runtime.resize(size.into());
            runtime.step(&[]).unwrap();
            assert!(runtime.bounds(&window_path("sixel")).is_some());
            assert!(runtime.bounds(&window_path("characters")).is_some());
            assert!(runtime.frame().has_raster());
            assert!(!runtime.frame().has_unsupported_raster_composition());
            let identity = runtime.state().image.id();
            for encoding in [Encoding::Braille, Encoding::Quadrant, Encoding::HalfBlock] {
                runtime
                    .step(&[Event::Key(KeyCode::Char('e').into())])
                    .unwrap();
                assert_eq!(runtime.state().encoding, encoding);
                assert_eq!(runtime.state().image.id(), identity);
            }
        }
    }

    #[test]
    fn both_documented_quit_keys_exit() {
        for key in [KeyCode::Esc, KeyCode::Char('q')] {
            let mut runtime = application(80, 24, RasterOutput::default()).unwrap();
            runtime.step(&[]).unwrap();
            runtime.step(&[Event::Key(key.into())]).unwrap();
            assert!(runtime.exit_reason().is_some());
        }
    }

    #[cfg(feature = "sixel")]
    #[test]
    fn native_window_emits_sixel_while_canvas_remains_character_output() {
        use rsille::render::{area::Size, buffer::Buffer, Builder, Draw, DrawErr};
        struct Snapshot(Buffer);
        impl Draw for Snapshot {
            fn draw(&mut self, mut target: Chunk) -> Result<Size, DrawErr> {
                for (x, y, cell) in self.0.all_cells() {
                    if let Some(raster) = self.0.raster_at((x, y).into()) {
                        target.set_raster_cell(x, y, raster.clone())?;
                    } else {
                        target.set_forced(x, y, cell.content.clone())?;
                    }
                }
                Ok(self.0.size())
            }
        }
        // Synthetic verified capabilities apply ONLY to this memory writer.
        let config = options(&["--cell-pixels", "10x20"])
            .unwrap()
            .native_config((110, 36))
            .unwrap();
        let mut runtime = application(110, 36, config).unwrap();
        runtime.step(&[]).unwrap();
        let native_cells = runtime.frame().raster_cells().count();
        assert!(native_cells > 0);
        assert!(!runtime.frame().has_unsupported_raster_composition());
        let mut output = Vec::new();
        {
            let mut renderer = Builder::new()
                .size((110, 36))
                .enable_alt_screen()
                .raster_output(config)
                .build_render(Snapshot(runtime.frame().clone()), &mut output);
            renderer.render().unwrap();
            assert_eq!(renderer.raster_stats().tiles_sent, native_cells as u64);
            let stats = renderer.raster_stats();
            renderer.render().unwrap();
            assert_eq!(renderer.raster_stats(), stats);
        }
        let header = b"\x1bP7;1;0q\"1;1;10;20";
        assert_eq!(
            output
                .windows(header.len())
                .filter(|bytes| *bytes == header)
                .count(),
            native_cells
        );
        assert!(String::from_utf8(output).unwrap().contains('▀'));
    }
}
