//! Typst block math displayed directly through rsille's raster widget.

use std::{error::Error, sync::Arc};

use rsille::tui::prelude::*;
use rsille_typst::{MathRenderer, RenderOptions};

const DEFAULT_FORMULA: &str = "sum_(k=1)^n k = frac(n(n+1), 2)";
const HELP: &str = "Typst block formula — direct terminal display

Usage: formula [OPTIONS] ['Typst math contents']
  --cell-pixels WxH    Measured physical pixels per character cell
  --force-cell-pixels  Confirm a measured override equal to the terminal grid
  --headless           Check the same widget/Runtime in memory, without TTY I/O
  --help, -h           Show usage
  --                   Treat the remaining argument as formula source

q / Esc quits. Formula defaults to sum_(k=1)^n k = frac(n(n+1), 2).
Use Typst math contents, without surrounding dollar signs; not LaTeX.
Inline on the normal screen, without mouse capture or an alternate screen.
Original pixels are never enlarged: reserve formula-sized rows plus a title.
Narrow/short terminals clip the formula; resize reveals it again, without reflow.
The framework caps inline content at 50 rows and reserves Sixel bottom guards.

Interactive output always uses Sixel, including with no arguments.
Running interactively assumes your terminal AND multiplexer support Sixel,
local EL 2 graphics erasure, images following text scrolling, stable palettes,
non-scrolling positioned output, and
transparent padding in partial final six-pixel bands. No capability detection.
Build with --features sixel. Unsupported raster output errors rather than degrading to text.

Cell pixels are ONE character cell's physical width x height, NOT columns x rows,
window pixels or font points. Dimensions: 1..=1024, area <=65536.
Without an override, CellPixels::resolve calls CellPixels::probe before any input
reader: OS metrics first, then a Unix CSI 16 t query with a 500 ms timeout.
Queries require controlling-terminal stdin/stdout TTYs; native Windows has no
query fallback. If discovery fails, supply measured --cell-pixels WxH.
Overrides skip probing. A grid-sized override requires --force-cell-pixels.
Metrics are stored once; restart after font/DPI changes. Queries do not detect
Sixel support and may consume unrelated input; late replies may reach the reader.
--headless alone checks metadata without the sixel feature or probing, using
synthetic 10x20 layout metrics. Adding --cell-pixels selects in-memory Sixel
encoding and requires --features sixel; it never probes. Synthetic metrics are
suitable for checks only; headless does not certify a real terminal.";

#[derive(Default, Debug)]
struct Options {
    pixels: Option<CellPixels>,
    force_cell_pixels: bool,
    headless: bool,
    help: bool,
    formula: Option<String>,
}

impl Options {
    fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut options = Self::default();
        let mut args = arguments.into_iter();
        let mut literal = false;
        while let Some(argument) = args.next() {
            if literal || !argument.starts_with('-') {
                if options.formula.replace(argument).is_some() {
                    return Err("provide one quoted Typst formula; use --help".into());
                }
                continue;
            }
            match argument.as_str() {
                "--" => literal = true,
                "--cell-pixels" => {
                    let value = args.next().ok_or("--cell-pixels needs WxH")?;
                    let (width, height) = value.split_once('x').ok_or("cell pixels must be WxH")?;
                    options.pixels = Some(CellPixels {
                        width: width.parse().map_err(|_| "invalid cell pixel width")?,
                        height: height.parse().map_err(|_| "invalid cell pixel height")?,
                    });
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

    fn config(&self, grid: (u16, u16)) -> Result<RasterOutput, String> {
        if self.headless && self.pixels.is_none() {
            return Ok(RasterOutput::default());
        }
        if !cfg!(feature = "sixel") {
            return Err("native output needs --features sixel".into());
        }

        // resolve(None) probes before App::run starts its input reader. An explicit
        // override is validated without OS lookup or terminal queries.
        let pixels = CellPixels::resolve(self.pixels).map_err(|error| error.to_string())?;
        if self.pixels.is_some() && (pixels.width, pixels.height) == grid && !self.force_cell_pixels
        {
            return Err("--cell-pixels matches columns x rows, not usually physical cell pixels; confirm measured values with --force-cell-pixels".into());
        }
        Ok(RasterOutput {
            capabilities: RasterCapabilities {
                sixel: true,
                reliable_erase: false,
                reliable_inline_erase: true,
                no_scroll: true,
                cell_pixels: Some(pixels),
            },
        })
    }
}

struct State {
    canvas: Arc<RasterCanvas>,
    formula_size: (u16, u16),
}

// Headless metadata checks use synthetic cell dimensions, never terminal discovery.
fn formula_size(canvas: &RasterCanvas, config: RasterOutput) -> (u16, u16) {
    let pixels = config.capabilities.cell_pixels.unwrap_or(CellPixels {
        width: 10,
        height: 20,
    });
    (
        canvas.width().div_ceil(u32::from(pixels.width)) as u16,
        canvas.height().div_ceil(u32::from(pixels.height)) as u16,
    )
}

#[derive(Clone)]
enum Msg {
    Quit,
}

fn view(state: &State) -> impl Widget<Msg> {
    col().child(label("Typst / Sixel / q or Esc: quit")).child(
        raster_canvas(Arc::clone(&state.canvas))
            .key("formula")
            .scale(RasterScale::Original)
            .fixed(state.formula_size.0, state.formula_size.1),
    )
}

fn application(state: State, config: RasterOutput, size: (u16, u16)) -> Runtime<State, Msg> {
    App::new(state)
        .mode(Mode::Inline)
        .disable_mouse_capture()
        .with_motion_policy(MotionPolicy::disabled())
        .raster_output(config)
        .with_quit_key(KeyCode::Esc)
        .on_key(KeyCode::Char('q'), || Msg::Quit)
        .update(|_: &mut State, Msg::Quit, ctx: &mut UpdateCtx<Msg>| ctx.exit())
        .view(view)
        .build()
        .size(size.0, size.1)
}

// Exercise the same renderer used by the terminal loop, not just layout metadata.
fn check_output(
    frame: &rsille::render::buffer::Buffer,
    config: RasterOutput,
) -> Result<(), Box<dyn Error>> {
    use rsille::render::{area::Size, buffer::Buffer, chunk::Chunk, Builder, Draw, DrawErr};
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
    let mut bytes = Vec::new();
    let mut renderer = Builder::new()
        .size(frame.size().into())
        .inline_mode(true)
        .inline_max_height(frame.size().height)
        .raster_output(config)
        .build_render(Snapshot(frame.clone()), &mut bytes);
    renderer.render()?;
    let stats = renderer.raster_stats();
    renderer.render()?;
    assert_eq!(
        renderer.raster_stats(),
        stats,
        "unchanged frame resends tiles"
    );
    drop(renderer);
    for forbidden in [b"\x1b[?1049h".as_slice(), b"\x1b[?1000h", b"\x1b[2J"] {
        assert!(!bytes.windows(forbidden.len()).any(|part| part == forbidden));
    }
    {
        assert_eq!(stats.tiles_sent, frame.raster_cells().count() as u64);
        assert!(bytes.windows(2).any(|part| part == b"\x1bP"));
        assert!(bytes.windows(2).any(|part| part == b"\x1b\\"));
    }
    Ok(())
}

fn headless(canvas: Arc<RasterCanvas>, config: RasterOutput) -> Result<(), Box<dyn Error>> {
    let formula_size = formula_size(&canvas, config);
    let rows = formula_size.1.saturating_add(1).min(50);
    for key in [KeyCode::Char('q'), KeyCode::Esc] {
        let mut runtime = application(
            State {
                canvas: Arc::clone(&canvas),
                formula_size,
            },
            config,
            (100, rows.min(29)),
        );
        runtime.step(&[])?;
        assert!(runtime.frame().has_raster());
        if config.capabilities.sixel {
            check_output(runtime.frame(), config)?;
        }
        let bounds = runtime
            .bounds(&WidgetPath::root().child("formula"))
            .unwrap();
        assert_eq!(bounds.size().width, formula_size.0.min(100));
        assert_eq!(bounds.size().height, formula_size.1.min(rows.min(29) - 1));
        runtime.step(&[Event::Resize(12, rows.min(15))])?;
        assert!(runtime.frame().has_raster());
        if config.capabilities.sixel {
            check_output(runtime.frame(), config)?;
        }
        runtime.step(&[Event::Resize(100, rows.min(29))])?;
        if config.capabilities.sixel {
            check_output(runtime.frame(), config)?;
        }
        runtime.step(&[Event::Key(key.into())])?;
        assert!(runtime.exit_reason().is_some());
    }
    println!("Headless checks passed: inline, natural size {formula_size:?}, content cap {rows} rows, no alternate screen/mouse/global erase; Typst -> RasterCanvas -> App/raster_canvas, resize and q/Esc; Sixel configurations additionally check encoded output and unchanged-frame reuse. Real terminal behavior was not tested.");
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse(std::env::args().skip(1))?;
    if options.help {
        println!("{HELP}");
        return Ok(());
    }
    let size = if options.headless {
        (100, 30)
    } else {
        crossterm::terminal::size()?
    };
    let config = options.config(size)?;

    // Fixed, explicit font assets; no system font search or runtime downloads.
    let renderer = MathRenderer::new(typst_assets::fonts(), "New Computer Modern Math")?;
    let result = renderer.render_with_diagnostics(
        options.formula.as_deref().unwrap_or(DEFAULT_FORMULA),
        &RenderOptions::default(),
    )?;
    for warning in result.warnings {
        eprintln!("Typst warning: {}", warning.message);
    }
    let canvas = Arc::new(result.canvas);
    if options.headless {
        headless(canvas, config)
    } else {
        let formula_size = formula_size(&canvas, config);
        application(
            State {
                canvas,
                formula_size,
            },
            config,
            size,
        )
        .run()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(args: &[&str]) -> Result<Options, String> {
        Options::parse(args.iter().map(|arg| (*arg).to_owned()))
    }

    #[test]
    fn natural_dimensions_use_cell_metrics_not_viewport() {
        let renderer =
            MathRenderer::new(typst_assets::fonts(), "New Computer Modern Math").unwrap();
        let canvas = renderer
            .render(DEFAULT_FORMULA, &RenderOptions::default())
            .unwrap();
        for (width, height) in [(10, 20), (8, 18), (1, 1)] {
            let mut config = RasterOutput::default();
            config.capabilities.cell_pixels = Some(CellPixels { width, height });
            assert_eq!(
                formula_size(&canvas, config),
                (
                    canvas.width().div_ceil(u32::from(width)) as u16,
                    canvas.height().div_ceil(u32::from(height)) as u16,
                )
            );
        }
        headless(Arc::new(canvas), RasterOutput::default()).unwrap();
    }

    #[test]
    fn explicit_modes_and_arguments() {
        let default = options(&[]).unwrap();
        assert!(!default.help && !default.headless && default.pixels.is_none());
        assert!(options(&["--cell-pixels", "8x18"]).is_ok());
        assert!(options(&["--sixel"]).is_err());
        assert!(options(&["--help"]).unwrap().help);
        assert!(options(&["-h"]).unwrap().help);
        assert!(options(&["--force-cell-pixels"]).is_err());
        assert!(options(&["--text-only"]).is_err());
        assert!(options(&["x", "y"]).is_err());
        assert_eq!(
            options(&["--", "-x"]).unwrap().formula.as_deref(),
            Some("-x")
        );
    }

    #[cfg(feature = "sixel")]
    #[test]
    fn default_and_custom_formula_reach_native_output() {
        let renderer =
            MathRenderer::new(typst_assets::fonts(), "New Computer Modern Math").unwrap();
        let config = options(&["--headless", "--cell-pixels", "10x20"])
            .unwrap()
            .config((100, 30))
            .unwrap();
        for formula in [DEFAULT_FORMULA, "integral_0^1 x^2 dif x = frac(1, 3)"] {
            let canvas = renderer.render(formula, &RenderOptions::default()).unwrap();
            headless(Arc::new(canvas), config).unwrap();
        }
    }

    #[test]
    fn native_mode_is_strict_and_headless_never_probes() {
        let metadata = options(&["--headless"]).unwrap().config((100, 30)).unwrap();
        assert!(!metadata.capabilities.sixel);
        assert!(metadata.capabilities.cell_pixels.is_none());
        let config = |pixels: &str| {
            options(&["--headless", "--cell-pixels", pixels])
                .unwrap()
                .config((100, 30))
        };
        assert!(config("0x18").is_err());
        assert!(config("100x30").is_err());
        if cfg!(feature = "sixel") {
            let interactive = options(&["--cell-pixels", "10x20"])
                .unwrap()
                .config((100, 30))
                .unwrap();
            assert!(interactive.capabilities.sixel);
            let native = config("10x20").unwrap();
            assert!(native.capabilities.sixel);
            assert!(native.capabilities.reliable_inline_erase);
            assert!(!native.capabilities.reliable_erase);
            assert_eq!(
                native.capabilities.cell_pixels,
                Some(CellPixels {
                    width: 10,
                    height: 20
                })
            );
            assert!(options(&[
                "--headless",
                "--cell-pixels",
                "100x30",
                "--force-cell-pixels"
            ])
            .unwrap()
            .config((100, 30))
            .is_ok());
        } else {
            assert!(config("10x20").is_err());
            assert_eq!(
                options(&[]).unwrap().config((100, 30)).unwrap_err(),
                "native output needs --features sixel"
            );
        }
    }
}
