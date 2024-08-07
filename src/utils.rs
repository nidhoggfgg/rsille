use core::fmt;
use std::error::Error;

use crate::braille::Pixel;

pub const MIN_ZOOM: f64 = 0.001;
pub const MIN_DIFFERENCE: f64 = 1E-10;

pub(crate) fn round<T>(v: T) -> i32
where
    T: Into<f64>,
{
    v.into().round() as i32
}

pub(crate) fn mean(a: &[f64]) -> f64 {
    let sum = a.iter().sum::<f64>();
    sum / a.len() as f64
}

pub(crate) fn check_zoom(v: f64) {
    if v <= MIN_ZOOM {
        panic!("zoom too small!");
    }
}

// the the (col, row) of (x, y)
pub fn get_pos<T>(x: T, y: T) -> (i32, i32)
where
    T: Into<f64>,
{
    let (x, y) = (round(x), round(y));
    let row = if y < 0 { (y + 1) / 4 - 1 } else { y / 4 };
    let col = if x < 0 { (x - 1) / 2 } else { x / 2 };
    (col, row)
}

#[allow(unused)]
pub(crate) fn make_braille(c: char) -> Option<Pixel> {
    let c = c as u32;
    if (0x2800..=0x28FF).contains(&c) {
        unsafe { Some(Pixel::from_unchecked(c - 0x2800)) }
    } else {
        None
    }
}

/// The error type used by this crate
#[derive(Debug, Clone)]
pub struct RsilleErr {
    msg: String,
}

impl RsilleErr {
    /// Return a new RsilleErr
    pub fn new(msg: String) -> Self {
        Self { msg }
    }

    /// Transform other error type to RsilleErr
    pub fn to_rsille_err<E: Error>(e: E) -> RsilleErr {
        RsilleErr::new(e.to_string())
    }
}

impl fmt::Display for RsilleErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

// pub trait Toi32 {
//     fn to_i32(&self) -> i32;
// }
//
// impl Toi32 for f64 {
//     fn to_i32(&self) -> i32 {
//         self.round() as i32
//     }
// }
//
// impl Toi32 for f32 {
//     fn to_i32(&self) -> i32 {
//         self.round() as i32
//     }
// }
//
// impl Toi32 for i32 {
//     fn to_i32(&self) -> i32 {
//         *self
//     }
// }
//
// // bad, but it's simple :)
// // only for usize, isize, i64, u64 and so on
// macro_rules! impl_round {
//     ($t:ty) => {
//         impl Toi32 for $t {
//             #[inline]
//             fn to_i32(&self) -> i32 {
//                 *self as i32
//             }
//         }
//     };
// }
//
// impl_round!(usize);
// impl_round!(isize);
// impl_round!(u64);
// impl_round!(i64);
// impl_round!(u16);
// impl_round!(i16);
// impl_round!(u8);
// impl_round!(i8);
