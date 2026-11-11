//! Typst block math displayed directly through rsille's raster widget.

use std::{error::Error, sync::Arc};

use rsille::tui::prelude::*;
use rsille_typst::{MathRenderer, RenderOptions};

const DEFAULT_FORMULA: &str = "sum_(k=1)^n k = frac(n(n+1), 2)";
const HELP: &str = "Typst block formula — direct terminal display

Usage: formula (--sixel | --text-only) [OPTIONS] ['Typst math contents']
  --sixel              Require native Sixel; never silently fall back
  --text-only          HalfBlock preview; NOT a readable-formula guarantee
  --cell-pixels WxH    Measured physical pixels per character cell
  --force-cell-pixels  Confirm a measured override equal to the terminal grid
  --headless           Check the same widget/Runtime in memory, without TTY I/O
  --help, -h           Show usage (also shown with no arguments)
  --                   Treat the remaining argument as formula source

q / Esc quits. Formula defaults to sum_(k=1)^n k = frac(n(n+1), 2).
Use Typst math contents, without surrounding dollar signs; not LaTeX.
Inline on the normal screen, without mouse capture or an alternate screen.
Original pixels are never enlarged: reserve formula-sized rows plus a title.
Narrow/short terminals clip the formula; resize reveals it again, without reflow.
The framework caps inline content at 50 rows and reserves Sixel bottom guards.

--sixel explicitly asserts that your terminal AND multiplexer support Sixel,
local EL 2 graphics erasure, images following text scrolling, stable palettes,
non-scrolling positioned output, and
transparent padding in partial final six-pixel bands. No capability detection.
Build with --features sixel. RequireSixel errors rather than degrading to text.

Cell pixels are ONE character cell's physical width x height, NOT columns x rows,
window pixels or font points. Dimensions: 1..=1024, area <=65536.
Without an override, CellPixels::resolve calls CellPixels::probe before any input
reader: OS metrics first, then a Unix CSI 16 t query with a 500 ms timeout.
Queries require controlling-terminal stdin/stdout TTYs; native Windows has no
query fallback. If discovery fails, supply measured --cell-pixels WxH.
Overrides skip probing. A grid-sized override requires --force-cell-pixels.
Metrics are stored once; restart after font/DPI changes. Queries do not detect
Sixel support and may consume unrelated input; late replies may reach the reader.
Text-only never probes. Headless Sixel requires explicit --cell-pixels (synthetic
metrics are suitable for checks only); headless does not certify a real terminal.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Output {
    Sixel,
    Text,
}

#[derive(Default, Debug)]
struct Options {
    output: Option<Output>,
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
                "--sixel" | "--text-only" => {
                    let output = if argument == "--sixel" {
                        Output::Sixel
                    } else {
                        Output::Text
                    };
                    if options.output.is_some_and(|previous| previous != output) {
                        return Err("choose either --sixel or --text-only".into());
                    }
                    options.output = Some(output);
                }
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
        if options.pixels.is_some() && options.output != Some(Output::Sixel) {
            return Err("--cell-pixels requires --sixel".into());
        }
        if options.force_cell_pixels && options.pixels.is_none() {
            return Err("--force-cell-pixels requires --sixel --cell-pixels WxH".into());
        }
        Ok(options)
    }

    fn config(&self, grid: (u16, u16)) -> Result<RasterOutput, String> {
        if self.output != Some(Output::Sixel) {
            return Ok(RasterOutput::text_only());
        }
        if !cfg!(feature = "sixel") {
            return Err(
                "native output needs --features sixel; alternatively use --text-only".into(),
            );
        }
        if self.headless && self.pixels.is_none() {
            return Err(
                "headless Sixel checks require explicit --cell-pixels WxH (no probing)".into(),
            );
        }
        // resolve(None) probes before App::run starts its input reader. An explicit
        // override is validated without OS lookup or terminal queries.
        let pixels = CellPixels::resolve(self.pixels).map_err(|error| error.to_string())?;
        if self.pixels.is_some() && (pixels.width, pixels.height) == grid && !self.force_cell_pixels
        {
            return Err("--cell-pixels matches columns x rows, not usually physical cell pixels; confirm measured values with --force-cell-pixels".into());
        }
        Ok(RasterOutput {
            policy: RasterPolicy::RequireSixel,
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
    output: Output,
    formula_size: (u16, u16),
}

// HalfBlock has one horizontal and two vertical samples per character cell.
fn formula_size(canvas: &RasterCanvas, config: RasterOutput) -> (u16, u16) {
    let pixels = config.capabilities.cell_pixels.unwrap_or(CellPixels {
        width: 1,
        height: 2,
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
    col()
        .child(label(match state.output {
            Output::Sixel => "Typst / Sixel / q or Esc: quit",
            Output::Text => "Typst / TEXT PREVIEW (readability not guaranteed) / q or Esc: quit",
        }))
        .child(
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
    if config.policy == RasterPolicy::RequireSixel {
        assert_eq!(stats.tiles_sent, frame.raster_cells().count() as u64);
        assert!(bytes.windows(2).any(|part| part == b"\x1bP"));
        assert!(bytes.windows(2).any(|part| part == b"\x1b\\"));
    } else {
        assert!(!bytes.windows(2).any(|part| part == b"\x1bP"));
    }
    Ok(())
}

fn headless(
    canvas: Arc<RasterCanvas>,
    output: Output,
    config: RasterOutput,
) -> Result<(), Box<dyn Error>> {
    let formula_size = formula_size(&canvas, config);
    let rows = formula_size.1.saturating_add(1).min(50);
    for key in [KeyCode::Char('q'), KeyCode::Esc] {
        let mut runtime = application(
            State {
                canvas: Arc::clone(&canvas),
                output,
                formula_size,
            },
            config,
            (100, rows.min(29)),
        );
        runtime.step(&[])?;
        assert!(runtime.frame().has_raster());
        check_output(runtime.frame(), config)?;
        let bounds = runtime
            .bounds(&WidgetPath::root().child("formula"))
            .unwrap();
        assert_eq!(bounds.size().width, formula_size.0.min(100));
        assert_eq!(bounds.size().height, formula_size.1.min(rows.min(29) - 1));
        runtime.step(&[Event::Resize(12, rows.min(15))])?;
        assert!(runtime.frame().has_raster());
        check_output(runtime.frame(), config)?;
        assert!(!runtime.frame_text_only().has_raster());
        runtime.step(&[Event::Resize(100, rows.min(29))])?;
        check_output(runtime.frame(), config)?;
        runtime.step(&[Event::Key(key.into())])?;
        assert!(runtime.exit_reason().is_some());
    }
    println!("Headless checks passed: inline, natural size {formula_size:?}, content cap {rows} rows, no alternate screen/mouse/global erase; Typst -> RasterCanvas -> App/raster_canvas -> encoded output, resize, unchanged-frame reuse, text fallback, q/Esc. Real terminal behavior was not tested.");
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse(std::env::args().skip(1))?;
    if options.help || (options.output.is_none() && !options.headless) {
        println!("{HELP}");
        return Ok(());
    }
    let size = if options.headless {
        (100, 30)
    } else {
        crossterm::terminal::size()?
    };
    let config = options.config(size)?;
    let output = options.output.unwrap_or(Output::Text);
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
        headless(canvas, output, config)
    } else {
        let formula_size = formula_size(&canvas, config);
        application(
            State {
                canvas,
                output,
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
            let mut config = RasterOutput::text_only();
            config.capabilities.cell_pixels = Some(CellPixels { width, height });
            assert_eq!(
                formula_size(&canvas, config),
                (
                    canvas.width().div_ceil(u32::from(width)) as u16,
                    canvas.height().div_ceil(u32::from(height)) as u16,
                )
            );
        }
        headless(Arc::new(canvas), Output::Text, RasterOutput::text_only()).unwrap();
    }

    #[test]
    fn explicit_modes_and_arguments() {
        assert!(options(&[]).unwrap().output.is_none());
        assert!(options(&["--sixel", "--text-only"]).is_err());
        assert!(options(&["--cell-pixels", "8x18"]).is_err());
        assert!(options(&["--force-cell-pixels"]).is_err());
        assert!(options(&["--text-only", "x", "y"]).is_err());
        assert_eq!(
            options(&["--text-only", "--", "-x"])
                .unwrap()
                .formula
                .as_deref(),
            Some("-x")
        );
    }

    #[cfg(feature = "sixel")]
    #[test]
    fn default_and_custom_formula_reach_native_output() {
        let renderer =
            MathRenderer::new(typst_assets::fonts(), "New Computer Modern Math").unwrap();
        let config = options(&["--sixel", "--headless", "--cell-pixels", "10x20"])
            .unwrap()
            .config((100, 30))
            .unwrap();
        for formula in [DEFAULT_FORMULA, "integral_0^1 x^2 dif x = frac(1, 3)"] {
            let canvas = renderer.render(formula, &RenderOptions::default()).unwrap();
            headless(Arc::new(canvas), Output::Sixel, config).unwrap();
        }
    }

    #[test]
    fn native_mode_is_strict_and_headless_never_probes() {
        assert!(options(&["--sixel", "--headless"])
            .unwrap()
            .config((100, 30))
            .is_err());
        let config = |pixels: &str| {
            options(&["--sixel", "--headless", "--cell-pixels", pixels])
                .unwrap()
                .config((100, 30))
        };
        assert!(config("0x18").is_err());
        assert!(config("100x30").is_err());
        if cfg!(feature = "sixel") {
            let native = config("10x20").unwrap();
            assert_eq!(native.policy, RasterPolicy::RequireSixel);
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
                "--sixel",
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
        }
    }
}
