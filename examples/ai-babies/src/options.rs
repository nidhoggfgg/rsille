use std::error::Error;

use rsille::{
    raster::Rgb,
    render::raster_output::{CellPixels, InlineRasterSession, RasterBackend},
};

use crate::{scene::Character, INLINE_HEIGHT};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Output {
    #[default]
    Quadrant,
    Sixel,
    Kitty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub character: Character,
    pub height: u16,
    pub output: Output,
    pub background: Option<Rgb>,
    pub pixels: Option<CellPixels>,
    pub force_cell_pixels: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            character: Character::default(),
            height: INLINE_HEIGHT,
            output: Output::Quadrant,
            background: None,
            pixels: None,
            force_cell_pixels: false,
        }
    }
}

pub fn animation_args(mut args: &[String]) -> Result<(Options, &[String]), Box<dyn Error>> {
    let mut options = Options::default();

    while let Some(arg) = args.first() {
        let mut consumed = 2;
        match arg.as_str() {
            "--character" => {
                options.character = Character::parse(
                    args.get(1)
                        .ok_or("--character requires chatgpt/deepseek/claude")?,
                )?;
            }
            "--height" => {
                options.height = args
                    .get(1)
                    .ok_or("--height requires ROWS (24 or 32)")?
                    .parse()?;
                if ![24, 32].contains(&options.height) {
                    return Err("--height must be 24 or 32 rows".into());
                }
            }
            "--raster" | "--text-only" => {
                return Err("Raster character previews were removed; use --kitty or --sixel".into())
            }
            "--sixel" | "--kitty" => {
                let requested = if arg == "--kitty" {
                    Output::Kitty
                } else {
                    Output::Sixel
                };
                if options.output != Output::Quadrant && options.output != requested {
                    return Err("choose either --kitty or --sixel".into());
                }
                options.output = requested;
                consumed = 1;
            }
            "--background" => {
                let value = args.get(1).ok_or("--background requires RRGGBB")?;
                let value = value.strip_prefix('#').unwrap_or(value);
                if value.len() != 6 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err("--background requires six hexadecimal digits (RRGGBB)".into());
                }
                let rgb = u32::from_str_radix(value, 16)?;
                options.background = Some(Rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8));
            }
            "--cell-pixels" => {
                let (width, height) = args.get(1).and_then(|s| s.split_once('x'))
                    .ok_or("--cell-pixels requires WxH: physical pixels per character cell, not terminal columns x rows")?;
                options.pixels = Some(CellPixels::resolve(Some(CellPixels {
                    width: width.parse()?,
                    height: height.parse()?,
                }))?);
            }
            "--force-cell-pixels" => {
                options.force_cell_pixels = true;
                consumed = 1;
            }
            _ => break,
        }
        args = &args[consumed..];
    }
    if options.background.is_some() && options.output == Output::Quadrant {
        return Err("--background requires --kitty or --sixel".into());
    }
    if options.pixels.is_some() && options.output == Output::Quadrant {
        return Err("--cell-pixels requires --kitty or --sixel".into());
    }
    if options.force_cell_pixels && options.pixels.is_none() {
        return Err("--force-cell-pixels requires --kitty or --sixel --cell-pixels WxH".into());
    }
    Ok((options, args))
}

impl Options {
    pub fn session(&self, grid: (u16, u16)) -> Result<Option<InlineRasterSession>, Box<dyn Error>> {
        self.session_with(grid, |output, explicit| match output {
            Output::Kitty if cfg!(unix) => RasterBackend::Kitty
                .negotiate_inline(explicit, std::time::Duration::from_millis(500)),
            Output::Kitty => CellPixels::resolve(explicit).map(InlineRasterSession::Kitty),
            Output::Sixel => CellPixels::resolve(explicit).map(InlineRasterSession::Sixel),
            Output::Quadrant => unreachable!(),
        })
    }

    pub fn session_with(
        &self,
        grid: (u16, u16),
        resolve: impl FnOnce(Output, Option<CellPixels>) -> std::io::Result<InlineRasterSession>,
    ) -> Result<Option<InlineRasterSession>, Box<dyn Error>> {
        if self.output == Output::Quadrant {
            return Ok(None);
        }
        if self.pixels.is_some_and(|p| (p.width, p.height) == grid) && !self.force_cell_pixels {
            return Err("--cell-pixels matches terminal columns x rows; supply measured physical pixels per character cell, or confirm with --force-cell-pixels".into());
        }
        Ok(Some(resolve(self.output, self.pixels)?))
    }
}
