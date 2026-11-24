use rsille::render::{area::Area, buffer::Buffer, chunk::Chunk, raster_output::KittyImage, Draw};
use std::io::{self, Write};

use unicode_normalization::UnicodeNormalization;

pub struct KittySprites {
    pub whale: KittyImage,
    pub fish: KittyImage,
    pub panel: KittyImage,
}

// Reserve adjacent layers so a treat or menu can overlap the whale.
pub(super) const WHALE_Z: i32 = i32::MAX - 2;
pub(super) const FISH_Z: i32 = i32::MAX - 1;

impl KittySprites {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            whale: KittyImage::new()?,
            fish: KittyImage::new()?,
            panel: KittyImage::new()?,
        })
    }

    // Draw the same cell layout used by the text backend into an owned placement.
    // Kitty scales the bitmap to those exact cells, including after a font resize.
    pub(super) fn draw_panel(
        &self,
        area: Area,
        mut panel: impl Draw,
        out: &mut Vec<u8>,
    ) -> io::Result<()> {
        let cells = area.size();
        let mut buffer = Buffer::new(cells);
        panel.draw(Chunk::new(&mut buffer, cells.into())?)?;
        let cell_width = 8.min(u16::MAX / cells.width);
        let width = cells.width * cell_width;
        let height = cells.height * 16;
        let stride = usize::from(width);
        let mut bitmap = vec![[0; 4]; stride * usize::from(height)];
        for y in 0..cells.height {
            for x in 0..cells.width {
                let Some(cell) = buffer.get((x, y).into()) else {
                    continue;
                };
                let symbol = cell.symbol();
                let mut chars = symbol
                    .nfc()
                    .filter(|&ch| !matches!(ch, '\u{fe0e}' | '\u{fe0f}'));
                let Some(ch) = chars.next().filter(|&ch| ch != ' ') else {
                    continue;
                };
                let glyph = chars
                    .next()
                    .is_none()
                    .then(|| unifont::get_glyph(ch))
                    .flatten()
                    .unwrap_or_else(|| unifont::get_glyph('�').unwrap());
                let left = usize::from(x * cell_width);
                let span = (cell.width() * usize::from(cell_width)).min(stride - left);
                for gy in 0..16 {
                    let start = (usize::from(y) * 16 + gy) * stride + left;
                    for dx in 0..span {
                        if glyph.get_pixel(dx * glyph.get_width() / span, gy) {
                            bitmap[start + dx] = [195, 244, 244, 255];
                        }
                    }
                }
            }
        }
        write!(out, "\x1b[{};{}H", area.y() + 1, area.x() + 1)?;
        self.panel
            .draw_sized(out, (width, height).into(), &bitmap, i32::MAX, Some(cells))
    }
}
