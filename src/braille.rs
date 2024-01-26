use core::fmt;

use crate::utils::normalize;

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

fn get_pixel(x: f64, y: f64) -> u32 {
    let (x, y) = (normalize(x), normalize(y));
    PIXEL_MAP[y % 4][x % 2]
}

fn make_braille_unchecked(p: u32) -> char {
    unsafe { char::from_u32_unchecked(BASE_CHAR + p) }
}

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
}

impl fmt::Display for Pixel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", make_braille_unchecked(self.code))
    }
}
