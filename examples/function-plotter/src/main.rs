mod export;
mod formula;
mod grid;
mod notebook;
mod plot;

use notebook::{application, State};

use plot::Plot;
use rsille::tui::prelude::*;
use std::error::Error;

const X_RANGE: [f64; 2] = [-10.0, 10.0];
#[cfg(test)]
const BOUNDS: [f64; 4] = [-10.0, 10.0, -3.0, 3.0];
const HELP: &str = "Function plotter — 2D curves and shaded 3D surfaces

Normal invocation uses native Sixel via rsille's default features.

  --expr 'sin(x); x^2/8'   Initial functions (up to six)
  --grid                  Independent demo panels; no expr: four presets
                          Repeat --expr for 1–6 panels (4096 bytes each).
                          Semicolons still overlay within one panel.
                          Live preview; Enter/F5 freezes, commits and exits.
                          Responsive 1–2 columns; no grid editor or history.
  --x-range MIN MAX      Finite increasing x domain (default -10 10)
  --cell-pixels WxH       Actual pixels per character cell, NOT terminal size
  --frames DIRECTORY      Export 90 RGB preview frames (15 FPS); new directory
  --snapshot FILE.ppm     Export a static RGB preview; new file
                          Standalone options; no terminal required.
  --headless              Exercise inline REPL commits without a terminal
  --help                  Show help

Live preview below input after 500ms idle (also on history recall); no submission.
Edits clear old previews; empty input clears preview. Preview errors do not block Enter.
Single-plot REPL: Enter / F5 evaluates In[n]:= and commits Out[n]= or an error.
A new blank prompt follows; old output stays in terminal history, not editable cells.
Up/Down recalls the newest 24 inputs and restores your draft. Esc: quit.
REPL commands (type then Enter; no math syntax changes):
  :grid     Enter empty grid composition (up to six independent panels)
  :plot     Discard the grid and return to single plots
In grid: Enter adds the expression as a panel; F5 adds any nonempty draft then
freezes and commits the whole grid. :submit commits already-added panels.
:undo removes the last panel; :clear removes all. Invalid/full-grid drafts stay
editable and block submission. A commit opens another empty grid, not an exit.
Drafts preview after 500ms; added 3D panels keep rotating until commit.
Commands are not expression history; added expressions are. :grid in grid is a no-op.
Inputs are capped at 4096 bytes. No session is saved; committed graphs are released.
Mouse capture is disabled: use your terminal's normal scrollback controls.
Single plots: formulas and graphs share max(70 columns, native formula width),
capped by available terminal width.
Single graphs stay 14 rows high; grids request 30 rows with responsive 1–2 columns.
Bounds auto-scale independently from finite samples.
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
3D syntax (up to six surfaces/curves; cannot mix with 2D):
  surface(sin(x)*cos(y), -pi, pi, -pi, pi)  Height field z(x,y)
  param3(cos(t), sin(t), t/4, 0, 6*pi)      Space curve x(t), y(t), z(t)
3D uses orthographic projection, equal world units and depth-tested shading.
Live 3D previews rotate around vertical z; submission freezes the displayed angle.
Explicit ranges override --x-range. Fixed sampling can miss narrow features/poles.

Normal invocation requires that your terminal/multiplexer supports Sixel, ED 2 and EL2
 graphics erase, graphics following text scroll, stable palettes and non-scrolling
 positioned output. Image scrollback retention is terminal dependent. No capability
 detection is performed. Cell metrics are resolved before input starts; restart
 after font/DPI changes. If automatic measurement fails, supply measured cell
 pixels (native Windows has no query fallback). Never guess the dimensions.";

#[derive(Default)]
struct Options {
    pixels: Option<CellPixels>,
    expr: Option<String>,
    panels: Vec<String>,
    grid: bool,
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
                "--grid" => result.grid = true,
                "--expr" => {
                    let source = args.next().ok_or("--expr needs an expression")?;
                    if source.len() > 4096 {
                        return Err("Expression exceeds 4096 bytes".into());
                    }
                    result.expr = Some(source.clone());
                    result.panels.push(source);
                }
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

        if result.grid && result.panels.len() > grid::MAX_PANELS {
            return Err("--grid accepts at most six --expr panels".into());
        }
        Ok(result)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if matches!(
        args.first().map(String::as_str),
        Some("--frames" | "--snapshot")
    ) {
        return export::command(&args);
    }
    let options = Options::parse(args)?;
    if options.help {
        println!("{HELP}");
        return Ok(());
    }
    let x_range = options.x_range.unwrap_or(X_RANGE);
    if options.headless {
        return if options.grid {
            grid::smoke_test(grid::State::new(options.panels, x_range)?)
        } else {
            notebook::smoke_test(State::new(options.expr, x_range)?)
        };
    }
    let output = RasterOutput {
        capabilities: RasterCapabilities {
            sixel: true,
            reliable_erase: true,
            reliable_inline_erase: true,
            no_scroll: true,
            cell_pixels: Some(CellPixels::resolve(options.pixels)?),
        },
    };
    if options.grid {
        grid::application(grid::State::new(options.panels, x_range)?, output).run()?;
    } else {
        application(State::new(options.expr, x_range)?, output).run()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grid_cli_keeps_panels_separate_and_preserves_legacy_last_expr() {
        let args = ["--expr", "x; x^2", "--expr", "polar(1,0,pi)"];
        let legacy = Options::parse(args.map(str::to_owned)).unwrap();
        assert_eq!(legacy.expr.as_deref(), Some("polar(1,0,pi)"));
        let grid = Options::parse(args.into_iter().chain(["--grid"]).map(str::to_owned)).unwrap();
        assert!(grid.grid);
        assert_eq!(grid.panels, ["x; x^2", "polar(1,0,pi)"]);
        assert!(Options::parse(
            std::iter::once("--grid".to_owned())
                .chain((0..7).flat_map(|_| ["--expr".to_owned(), "x".to_owned()]))
        )
        .is_err());
    }

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
    fn normal_invocation_and_cell_metrics_need_no_mode_flag() {
        let options = Options::parse([]).unwrap();
        assert!(!options.help);
        assert!(!options.headless);
        assert!(options.pixels.is_none());
        let options = Options::parse(["--cell-pixels", "8x18"].map(str::to_owned)).unwrap();
        let pixels = options.pixels.unwrap();
        assert_eq!(pixels.width, 8);
        assert_eq!(pixels.height, 18);
    }

    #[test]
    fn help_and_headless_remain_explicit_modes() {
        for flag in ["--help", "-h"] {
            assert!(Options::parse([flag.to_owned()]).unwrap().help);
        }
        assert!(Options::parse(["--headless".to_owned()]).unwrap().headless);
        assert!(!HELP.contains("--sixel"));
        assert!(!HELP.contains("--features"));
        assert!(HELP.contains("Sixel via rsille's default features"));
    }

    #[test]
    fn rejects_removed_flags_and_missing_values() {
        for args in [
            vec!["--sixel"],
            vec!["--text-only"],
            vec!["--cell-pixels"],
            vec!["--cell-pixels", "invalid"],
            vec!["--expr"],
        ] {
            assert!(Options::parse(args.into_iter().map(str::to_owned)).is_err());
        }
    }
}
