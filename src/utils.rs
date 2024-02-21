use core::fmt;
use std::error::Error;

pub const MIN_ZOOM: f64 = 0.001;
pub const MIN_DIFFERENCE: f64 = 1E-10;

pub fn normalize(v: f64) -> usize {
    v.round() as usize
}

pub fn mean(a: &[f64]) -> f64 {
    let sum = a.iter().sum::<f64>();
    sum / a.len() as f64
}

pub fn check_zoom(v: f64) {
    if v <= MIN_ZOOM {
        panic!("zoom too small!");
    }
}

pub type Offset = (usize, usize);

// a multi-type mean function, but it's too complex
// pub fn mean<'a, T>(a: &'a [T]) -> f64
// where
//     T: 'a
//         + std::iter::Sum<&'a T>
//         + std::iter::Sum<&'a T>
//         + std::ops::Div<Output = f64>
//         + std::convert::From<usize>,
// {
//     let sum = a.iter().sum::<T>();
//     sum / a.len().try_into().expect("")
// }

#[derive(Debug, Clone)]
pub struct RsilleErr {
    msg: String,
}

pub fn to_rsille_err<E: Error>(e: E) -> RsilleErr {
    RsilleErr::new(e.to_string())
}

impl RsilleErr {
    pub fn new(msg: String) -> Self {
        Self { msg }
    }
}

impl fmt::Display for RsilleErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}
