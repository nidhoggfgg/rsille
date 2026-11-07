//! Low-level construction of individual Unicode Braille cells.
//!
//! A [`Pixel`] stores eight dots in a 2-column, 4-row grid. [`PixelOp`] uses
//! Cartesian coordinates with `(0, 0)` at the bottom left. Coordinates wrap
//! within the cell, including negative values. For a drawing spanning multiple
//! cells or using other encodings, use [`Canvas`](super::Canvas).
//!
//! ```
//! use rsille::canvas::braille::{Pixel, PixelOp};
//!
//! let mut pixel = Pixel::new();
//! pixel.set(0, 3).set(1, 3); // The two top dots.
//! assert_eq!(pixel.to_char(), '⠉');
//! assert_eq!(Pixel::from(0xff).to_char(), '⣿');
//! ```

use core::fmt;
use std::io;

use crate::render::style::Stylized;
use crossterm::{queue, style::Print};

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

/// One Braille character, represented by its eight-dot bit mask.
///
/// Bits follow Unicode dot numbering, from bit 0 for dot 1 to bit 7 for dot 8:
///
/// ```text
/// 1 4   y = 3
/// 2 5   y = 2
/// 3 6   y = 1
/// 7 8   y = 0
/// ```
///
/// The default value is the blank Braille character `U+2800`, not an ASCII
/// space. Formatting with [`Display`](fmt::Display) emits the Braille character.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pixel {
    code: u8,
}

impl Pixel {
    /// Creates a blank Braille cell with all dots unset.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self { code: 0 }
    }

    /// Creates a cell from a Unicode Braille bit mask.
    ///
    /// Every `u8` value is valid; `0` is blank and `255` sets all eight dots.
    #[inline]
    #[must_use]
    pub fn from(code: u8) -> Self {
        Self { code }
    }

    /// Creates a blank Braille cell, equivalent to [`new`](Self::new).
    ///
    /// Its character is `U+2800`, not the ASCII space `U+0020`.
    #[inline]
    #[must_use]
    pub fn space() -> Self {
        Self { code: 0 }
    }

    /// Queues this character on `buffer` without flushing it.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if writing the character fails.
    #[inline]
    pub fn queue(&self, buffer: &mut impl io::Write) -> io::Result<()> {
        queue!(buffer, Print(self))
    }

    /// Returns the Unicode character in `U+2800..=U+28FF` for this cell.
    #[inline]
    #[must_use]
    pub fn to_char(&self) -> char {
        make_braille_unchecked(self.code)
    }
}

/// Mutates individual dots using Cartesian sample coordinates.
///
/// For [`Pixel`], `x` wraps modulo 2 and `y` wraps modulo 4, with the origin at
/// the bottom left. For example, `(-1, -1)` selects the same dot as `(1, 3)`.
/// All `i32` coordinates are accepted. Each method returns the cell so that
/// operations can be chained.
pub trait PixelOp {
    /// Clears the selected dot, leaving an already empty dot unchanged.
    fn unset(&mut self, x: i32, y: i32) -> &mut Self;
    /// Sets the selected dot, leaving an already occupied dot unchanged.
    fn set(&mut self, x: i32, y: i32) -> &mut Self;
    /// Inverts the selected dot's occupancy.
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

/// Returns the Unicode dot bit for coordinates wrapped within a Braille cell.
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

/// Converts an eight-dot mask to its Unicode Braille character.
///
/// Every `u8` offset from `U+2800` is a valid Unicode scalar value.
#[inline]
fn make_braille_unchecked(p: u8) -> char {
    unsafe { char::from_u32_unchecked(BASE_CHAR + p as u32) }
}
