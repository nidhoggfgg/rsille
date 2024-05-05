//! This module can be used to paint math function or math plot easily
//!
//! Like if you want to see the `y=sin(x)` and `y=cos(x)` on `(0, 5)`,
//! it's very very easy:
//! ```
//! use rsille::figure;
//! figure!((|x| x.sin(), (0, 5)), (|x| x.cos(), (0, 5)));
//! ```

mod figure;
pub mod glm;
mod plot;

pub use figure::Figure;
pub use figure::Plotable;
pub use plot::Plot;
