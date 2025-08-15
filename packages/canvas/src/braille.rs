use core::fmt;
use std::io;

use render::style::Stylized;
use term::crossterm::{queue, style::Print};

// http://www.alanwood.net/unicode/braille_patterns.html
// dots:
//    ,___,
//    |1 4|
//    |2 5|
//    |3 6|
//    |7 8|
//    `````
#[rustfmt::skip]
const PIXEL_MAP: [[u8; 2]; 4] = [[0x01, 0x08],
                                 [0x02, 0x10],
                                 [0x04, 0x20],
                                 [0x40, 0x80]];
// braille unicode characters starts at 0x2800
const BASE_CHAR: u32 = 0x2800;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pixel {
    code: u8,
}

impl Pixel {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self { code: 0 }
    }

    #[inline]
    #[must_use]
    pub fn from(code: u8) -> Self {
        Self { code }
    }

    #[inline]
    #[must_use]
    pub fn space() -> Self {
        Self { code: 0 }
    }

    #[inline]
    pub fn queue(&self, buffer: &mut impl io::Write) -> io::Result<()> {
        queue!(buffer, Print(self))
    }

    #[inline]
    #[must_use]
    pub fn to_char(&self) -> char {
        make_braille_unchecked(self.code)
    }
}

pub trait PixelOp {
    fn unset(&mut self, x: i32, y: i32) -> &mut Self;
    fn set(&mut self, x: i32, y: i32) -> &mut Self;
    fn toggle(&mut self, x: i32, y: i32) -> &mut Self;
}

impl PixelOp for Pixel {
    fn unset(&mut self, x: i32, y: i32) -> &mut Self {
        let p = get_pixel(x, y);
        self.code &= !p;
        self
    }

    fn set(&mut self, x: i32, y: i32) -> &mut Self {
        let p = get_pixel(x, y);
        self.code |= p;
        self
    }

    fn toggle(&mut self, x: i32, y: i32) -> &mut Self {
        let p = get_pixel(x, y);
        self.code ^= p;
        self
    }
}

impl fmt::Display for Pixel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", make_braille_unchecked(self.code))
    }
}

impl From<Pixel> for Stylized {
    fn from(value: Pixel) -> Self {
        Stylized::new(value.to_char(), Default::default())
    }
}

#[rustfmt::skip]                                      //              axis to index in PIXEL_MAP
fn get_pixel(x: i32, y: i32) -> u8 {                  //       ^ y                |  y ^    
    let y = if y >= 0 {                        //  -2 -1|   x  x: -1 -> 1  |  3 | * *     x: 0 -> 0
        [3, 2, 1, 0][(y % 4) as usize]                // ------+--->     -2 -> 0  |  2 | * *        1 -> 1      
    } else {                                          //   * * | -1   y: -1 -> 0  |  1 | * *     y: 0 -> 3      
        [3, 0, 1, 2][(y % 4).unsigned_abs() as usize] //   * * | -2      -2 -> 1  |  0 | * *        1 -> 2
    };                                                //   * * | -3      -3 -> 2  |  --+------>     2 -> 1
    let x = (x % 2).unsigned_abs() as usize;   //   * * | -4      -4 -> 3  |    | 0 1  x     3 -> 0
    PIXEL_MAP[y][x]                                   // those dots in braille is defined as PIXEL_MAP on the top of this file
}

// it's safety, dw :)
#[inline]
fn make_braille_unchecked(p: u8) -> char {
    unsafe { char::from_u32_unchecked(BASE_CHAR + p as u32) }
}
