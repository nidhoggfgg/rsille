use std::error::Error;

use rsille::{
    raster::Rgb,
    render::raster_output::{CellPixels, RasterCapabilities, RasterOutput},
};

use crate::{scene::Character, INLINE_HEIGHT};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Output {
    #[default]
    Quadrant,
    Sixel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub character: Character,
    pub height: u16,
    pub output: Output,
    pub background: Rgb,
    pub pixels: Option<CellPixels>,
    pub force_cell_pixels: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            character: Character::default(),
            height: INLINE_HEIGHT,
            output: Output::Quadrant,
            background: Rgb(0, 0, 0),
            pixels: None,
            force_cell_pixels: false,
        }
    }
}

pub fn animation_args(mut args: &[String]) -> Result<(Options, &[String]), Box<dyn Error>> {
    let mut options = Options::default();
    let mut background_set = false;
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
                return Err("Raster character previews were removed; use --sixel".into())
            }
            "--sixel" => {
                options.output = Output::Sixel;
                consumed = 1;
            }
            "--background" => {
                let value = args.get(1).ok_or("--background requires RRGGBB")?;
                let value = value.strip_prefix('#').unwrap_or(value);
                if value.len() != 6 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err("--background requires six hexadecimal digits (RRGGBB)".into());
                }
                let rgb = u32::from_str_radix(value, 16)?;
                options.background = Rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8);
                background_set = true;
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
    if background_set && options.output == Output::Quadrant {
        return Err("--background requires --sixel".into());
    }
    if options.pixels.is_some() && options.output != Output::Sixel {
        return Err("--cell-pixels requires --sixel".into());
    }
    if options.force_cell_pixels && options.pixels.is_none() {
        return Err("--force-cell-pixels requires --sixel --cell-pixels WxH".into());
    }
    Ok((options, args))
}

impl Options {
    pub fn native_config(&self, grid: (u16, u16)) -> Result<RasterOutput, Box<dyn Error>> {
        self.native_config_with(grid, CellPixels::resolve)
    }

    pub fn native_config_with(
        &self,
        grid: (u16, u16),
        resolve: impl FnOnce(Option<CellPixels>) -> std::io::Result<CellPixels>,
    ) -> Result<RasterOutput, Box<dyn Error>> {
        if self.output != Output::Sixel {
            return Ok(RasterOutput::default());
        }
        if !cfg!(feature = "sixel") {
            return Err("--sixel requires cargo --features sixel".into());
        }
        let pixels = resolve(self.pixels)?;
        if self.pixels.is_some() && (pixels.width, pixels.height) == grid && !self.force_cell_pixels
        {
            return Err("--cell-pixels matches terminal columns x rows; supply measured physical pixels per character cell, or confirm with --force-cell-pixels".into());
        }
        Ok(RasterOutput {
            capabilities: RasterCapabilities {
                // --sixel asserts local EL 2 graphics erasure and images following
                // text scrolling on the normal screen; this is not detection.
                sixel: true,
                reliable_erase: false,
                reliable_inline_erase: true,
                no_scroll: true,
                cell_pixels: Some(pixels),
            },
        })
    }
}
