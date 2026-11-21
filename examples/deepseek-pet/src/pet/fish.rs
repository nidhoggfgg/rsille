use crate::screen::{clear_span, Screen};
use rsille::render::raster_output::CellPixels;

// Each dot leaves the shell visible through the sprite.
const TILE: [&[u8; 24]; 12] = [
    b"...........kkk..........",
    b"..........kggakk........",
    b".........kggakkkkkk.....",
    b".kk....kkccccgggcccckk..",
    b".kgkk.kggggcgggggccegak.",
    b"..kggkaggggcaggcggceggak",
    b"...kgaagggagggaggggggggk",
    b"..kggkraaagggagggaaggaak",
    b".kakk.krraaaaaaaaarrrkk.",
    b".kk....kkrrraakkkkkkk...",
    b".........kkkkkagk.......",
    b"..............kk........",
];

fn color(ink: u8) -> u16 {
    match ink {
        b'.' => 256,
        b'k' => 0b011_010_00, // toasted outline
        b'r' => 0b101_011_00,
        b'a' => 0b110_100_01,
        b'g' => 0b111_101_01,
        b'c' => 0b111_111_10, // glint on the scales
        b'e' => 0b001_001_00, // eye
        _ => unreachable!("unknown fish sprite color"),
    }
}

// Coordinates in the 320px snack frames, before scaling or mirroring.
#[derive(Clone, Copy, PartialEq)]
pub(super) struct FishBite {
    left: usize,
    top: usize,
}

impl FishBite {
    pub(super) fn at(age: u16) -> Option<Self> {
        let age = usize::from(age);
        let (left, top) = match age {
            // Lift the whole fish to the lips, then slide it head-first into the mouth.
            0..=5 => (90 + age, 196 - 6 * age),
            6..=8 => (96, 160),
            9..=15 => (96 + 8 * (age - 8), 160),
            _ => return None,
        };
        Some(Self { left, top })
    }

    pub(super) fn pixel(self, x: usize, y: usize) -> Option<u16> {
        const WIDTH: usize = 64;
        const HEIGHT: usize = WIDTH / 2;
        if x < self.left
            || x >= (self.left + WIDTH).min(160)
            || !(self.top..self.top + HEIGHT).contains(&y)
        {
            return None;
        }
        let row = TILE[(y - self.top) * TILE.len() / HEIGHT];
        let ink = row[(x - self.left) * row.len() / WIDTH];
        (ink != b'.').then(|| color(ink))
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct FishIcon {
    col: u16,
    row: u16,
    span: u16,
    width: usize,
    height: usize,
}

impl FishIcon {
    pub(super) fn place((col, row): (u16, u16), cols: u16, rows: u16, cell: CellPixels) -> Self {
        let span: usize = if cols >= 4 { 3 } else { 1 };
        let cw = usize::from(cell.width);
        let ch = usize::from(cell.height);
        // Even a one-pixel sprite occupies a six-row Sixel band.
        let band_rows = 6_u16.div_ceil(cell.height);
        let row = row.min(rows - band_rows + 1);
        let available = usize::from(rows - row + 1) * ch / 6 * 6;
        let width = (span * cw)
            .min(2 * ch.min(available))
            .min(TILE[0].len() * 2);
        let span = width.div_ceil(cw) as u16;
        Self {
            col: col.saturating_sub(span / 2).clamp(1, cols - span),
            row,
            span,
            width,
            height: width.div_ceil(2),
        }
    }

    pub(super) fn contains(self, col: u16, row: u16) -> bool {
        row == self.row && (self.col..self.col + self.span).contains(&col)
    }

    pub(super) fn erase(self, cols: u16, rows: u16, screen: Option<&Screen>, out: &mut Vec<u8>) {
        if self.col <= cols && self.row <= rows {
            let width = self.span.min(cols - self.col + 1);
            out.extend_from_slice(b"\x1b[49m");
            clear_span(screen, self.col, self.row, width, out);
        }
    }

    pub(super) fn draw(self, out: &mut Vec<u8>) {
        let mut pixels = Vec::with_capacity(self.width * self.height);
        for y in 0..self.height {
            let line = TILE[(2 * y + 1) * TILE.len() / (2 * self.height)];
            for x in 0..self.width {
                let ink = line[(2 * x + 1) * line.len() / (2 * self.width)];
                pixels.push(color(ink));
            }
        }
        super::sixel::draw(self.col, self.row, self.width, &pixels, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_a_transparent_sprite_inside_the_bottom_right_corner() {
        for (cols, rows, cw, ch) in [(80, 24, 10, 20), (2, 1, 8, 8), (3, 6, 1, 1)] {
            let cell = CellPixels {
                width: cw,
                height: ch,
            };
            let fish = FishIcon::place((cols - 1, rows), cols, rows, cell);
            assert!(
                usize::from(fish.col - 1) * usize::from(cw) + fish.width
                    <= usize::from(cols - 1) * usize::from(cw)
            );
            assert!(
                usize::from(fish.row - 1) * usize::from(ch) + fish.height.next_multiple_of(6)
                    <= usize::from(rows) * usize::from(ch)
            );
            let mut out = Vec::new();
            fish.draw(&mut out);
            let header = format!(
                "\x1b[{};{}H\x1bP0;1q\"1;1;{};{}#",
                fish.row, fish.col, fish.width, fish.height
            );
            assert!(out.starts_with(header.as_bytes()));
            assert!(out.ends_with(b"\x1b\\"));
        }
    }
}
