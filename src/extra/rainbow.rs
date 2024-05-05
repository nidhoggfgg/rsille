pub use colorgrad::*;

use crate::color;

pub trait ToColor {
    fn to_color(self) -> color::Color;
}

impl ToColor for colorgrad::Color {
    fn to_color(self) -> color::Color {
        let c = self.to_rgba8();
        color::Color::Rgb {
            r: c[0],
            g: c[1],
            b: c[2]
        }
    }
}