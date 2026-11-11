mod formula;
mod notebook;
mod plot;

use notebook::{application, State};

use plot::Plot;
use rsille::tui::prelude::*;
use std::error::Error;

const X_RANGE: [f64; 2] = [-10.0, 10.0];
#[cfg(test)]
const BOUNDS: [f64; 4] = [-10.0, 10.0, -3.0, 3.0];
const HELP: &str = "Function plotter — Cartesian, polar and parametric curves

  --sixel                 Require native Sixel (build with --features sixel)
  --text-only             Character preview, no terminal pixel queries
  --expr 'sin(x); x^2/8'   Initial functions (up to six)
  --x-range MIN MAX      Finite increasing x domain (default -10 10)
  --cell-pixels WxH       Actual pixels per character cell, NOT terminal size
  --headless              Exercise inline REPL commits without a terminal
  --help                  Show help

Live preview below input after 500ms idle (also on history recall); no submission.
Edits clear old previews; empty input clears preview. Preview errors do not block Enter.
Inline REPL: Enter / F5 immediately evaluates In[n]:= and commits Out[n]= or an error.
A new blank prompt follows; old output stays in terminal history, not editable cells.
Up/Down recalls the newest 24 inputs and restores your draft. Esc: quit.
Inputs are capped at 4096 bytes. No session is saved; graphs are not retained in memory.
Mouse capture is disabled: use your terminal's normal scrollback controls.
Graphs fit within 70 columns by 14 rows; bounds auto-scale from finite samples.
Polar/parametric plots use equal pixel-unit axes; Cartesian-only plots fill the frame.
Pole outliers may be clipped; sampled bounds do not guarantee global extrema.
Start blank at In[1]; only explicit --expr evaluates and commits at startup.
Functions: sin, cos, tan, sqrt, exp, ln, abs; constants: pi, e; powers: x^2.
Use explicit multiplication: 2*x, not 2x. This is not a symbolic solver.
Mix up to six curves with semicolons. Curve syntax:
  sin(x)                              Cartesian; domain from --x-range
  polar(1+cos(theta), 0, 2*pi)          Polar radius; angle theta in radians
  param(sin(3*t), sin(2*t), 0, 2*pi)   Parametric x(t), y(t)
Polar/parametric ranges are required, finite and increasing; endpoints may use pi/e.

--sixel asserts that your terminal/multiplexer supports Sixel, ED 2 and EL2
 graphics erase, graphics following text scroll, stable palettes and non-scrolling
 positioned output. Image scrollback retention is terminal dependent. No capability
 detection is performed. Cell metrics are resolved before input starts; restart
 after font/DPI changes. If automatic measurement fails, supply measured cell
 pixels (native Windows has no query fallback). Never guess the dimensions.";

#[derive(Default)]
struct Options {
    sixel: Option<bool>,
    pixels: Option<CellPixels>,
    expr: Option<String>,
    x_range: Option<[f64; 2]>,
    headless: bool,
    help: bool,
}
impl Options {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut result = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--sixel" | "--text-only" => {
                    let native = arg == "--sixel";
                    if result.sixel.is_some_and(|old| old != native) {
                        return Err("choose --sixel OR --text-only".into());
                    }
                    result.sixel = Some(native);
                }
                "--x-range" => {
                    let mut range = [0.0_f64; 2];
                    for value in &mut range {
                        *value = args
                            .next()
                            .ok_or("--x-range needs MIN MAX")?
                            .parse()
                            .map_err(|_| "invalid x range number")?;
                    }
                    if !range.iter().all(|v| v.is_finite())
                        || range[0] >= range[1]
                        || !(range[1] - range[0]).is_finite()
                    {
                        return Err("--x-range needs finite MIN < MAX with a finite span".into());
                    }
                    result.x_range = Some(range);
                }
                "--expr" => result.expr = Some(args.next().ok_or("--expr needs an expression")?),
                "--cell-pixels" => {
                    let value = args.next().ok_or("--cell-pixels needs WxH")?;
                    let (w, h) = value.split_once('x').ok_or("expected WxH")?;
                    result.pixels = Some(CellPixels {
                        width: w.parse().map_err(|_| "invalid cell width")?,
                        height: h.parse().map_err(|_| "invalid cell height")?,
                    });
                }
                "--headless" => result.headless = true,
                "--help" | "-h" => result.help = true,
                _ => return Err(format!("unknown option: {arg}")),
            }
        }
        if result.pixels.is_some() && result.sixel != Some(true) {
            return Err("--cell-pixels requires --sixel".into());
        }
        Ok(result)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse(std::env::args().skip(1))?;
    if options.help || (options.sixel.is_none() && !options.headless) {
        println!("{HELP}");
        return Ok(());
    }
    let state = State::new(options.expr, options.x_range.unwrap_or(X_RANGE))?;
    if options.headless {
        return notebook::smoke_test(state);
    }
    let output = if options.sixel == Some(true) {
        if !cfg!(feature = "sixel") {
            return Err("--sixel needs --features sixel".into());
        }
        RasterOutput {
            policy: RasterPolicy::RequireSixel,
            capabilities: RasterCapabilities {
                sixel: true,
                reliable_erase: true,
                reliable_inline_erase: true,
                no_scroll: true,
                cell_pixels: Some(CellPixels::resolve(options.pixels)?),
            },
        }
    } else {
        RasterOutput::text_only()
    };
    application(state, output).run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn x_range_accepts_negatives_and_rejects_invalid_domains() {
        let options = Options::parse(["--x-range", "-2.5", "0.1"].map(str::to_owned)).unwrap();
        assert_eq!(options.x_range, Some([-2.5, 0.1]));
        assert!(options.expr.is_none());
        for args in [
            vec!["--x-range"],
            vec!["--x-range", "-1"],
            vec!["--x-range", "0", "0"],
            vec!["--x-range", "2", "-2"],
            vec!["--x-range", "NaN", "1"],
            vec!["--x-range", "0", "inf"],
            vec!["--x-range", "a", "1"],
            vec!["--x-range", "-1e308", "1e308"],
        ] {
            assert!(Options::parse(args.into_iter().map(str::to_owned)).is_err());
        }
    }

    #[test]
    fn rejects_conflicting_modes_and_orphan_metrics() {
        for args in [
            vec!["--sixel", "--text-only"],
            vec!["--cell-pixels", "8x18"],
            vec!["--expr"],
        ] {
            assert!(Options::parse(args.into_iter().map(str::to_owned)).is_err());
        }
    }
}
