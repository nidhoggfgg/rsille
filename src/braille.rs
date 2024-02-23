use core::fmt;

use crate::utils::round;

pub const SPACE: char = '⠀';
// http://www.alanwood.net/unicode/braille_patterns.html
// dots:
//    ,___,
//    |1 4|
//    |2 5|
//    |3 6|
//    |7 8|
//    `````
#[rustfmt::skip]
const PIXEL_MAP: [[u32; 2]; 4] = [[0x01, 0x08],
                                  [0x02, 0x10],
                                  [0x04, 0x20],
                                  [0x40, 0x80]];
// braille unicode characters starts at 0x2800
const BASE_CHAR: u32 = 0x2800;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub struct Pixel {
    code: u32,
}

impl Pixel {
    pub fn new() -> Self {
        Self { code: 0 }
    }
}

pub trait PixelOp {
    fn unset(&mut self, x: f64, y: f64);
    fn set(&mut self, x: f64, y: f64);
    fn toggle(&mut self, x: f64, y: f64);
    fn fill(&mut self);
}

impl PixelOp for Pixel {
    fn unset(&mut self, x: f64, y: f64) {
        let p = get_pixel(x, y);
        self.code &= !p;
    }

    fn set(&mut self, x: f64, y: f64) {
        let p = get_pixel(x, y);
        self.code |= p;
    }

    fn toggle(&mut self, x: f64, y: f64) {
        let p = get_pixel(x, y);
        if self.code & p != 0 {
            self.unset(x, y);
        } else {
            self.set(x, y);
        }
    }

    fn fill(&mut self) {
        self.code = 0xff;
    }
}

impl fmt::Display for Pixel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", make_braille_unchecked(self.code))
    }
}

fn get_pixel(x: f64, y: f64) -> u32 {
    let (x, y) = (round(x), round(y));
    let y = if y >= 0 {
        [3, 2, 1, 0][(y % 4) as usize]
    } else {
        [3, 0, 1, 2][(y % 4).unsigned_abs() as usize]
    };
    PIXEL_MAP[y as usize][(x % 2).unsigned_abs() as usize]
}

// it's safety, dw :)
fn make_braille_unchecked(p: u32) -> char {
    unsafe { char::from_u32_unchecked(BASE_CHAR + p) }
}
