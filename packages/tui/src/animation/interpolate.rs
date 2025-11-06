//! Interpolation utilities for animating values

use crate::style::Color;

/// Trait for types that can be interpolated
pub trait Interpolate: Sized {
    /// Interpolate between self and other with factor t (0.0 to 1.0)
    fn interpolate(&self, other: &Self, t: f32) -> Self;
}

impl Interpolate for f32 {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        self + (other - self) * t
    }
}

impl Interpolate for f64 {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        self + (other - self) * t as f64
    }
}

impl Interpolate for i32 {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        (*self as f32 + (other - self) as f32 * t) as i32
    }
}

impl Interpolate for u32 {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        (*self as f32 + (*other as f32 - *self as f32) * t) as u32
    }
}

impl Interpolate for u16 {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        (*self as f32 + (*other as f32 - *self as f32) * t) as u16
    }
}

impl Interpolate for u8 {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        (*self as f32 + (*other as f32 - *self as f32) * t) as u8
    }
}

/// Specialized color interpolation trait
pub trait ColorInterpolate {
    /// Interpolate between two colors
    fn interpolate_color(from: &Color, to: &Color, t: f32) -> Color;
}

impl ColorInterpolate for Color {
    fn interpolate_color(from: &Color, to: &Color, t: f32) -> Color {
        match (from, to) {
            // RGB to RGB interpolation
            (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
                Color::Rgb(
                    r1.interpolate(r2, t),
                    g1.interpolate(g2, t),
                    b1.interpolate(b2, t),
                )
            }
            // Convert indexed colors to RGB for interpolation
            (from_color, to_color) => {
                let from_rgb = color_to_rgb(from_color);
                let to_rgb = color_to_rgb(to_color);
                Color::Rgb(
                    from_rgb.0.interpolate(&to_rgb.0, t),
                    from_rgb.1.interpolate(&to_rgb.1, t),
                    from_rgb.2.interpolate(&to_rgb.2, t),
                )
            }
        }
    }
}

/// Convert any color to RGB for interpolation
fn color_to_rgb(color: &Color) -> (u8, u8, u8) {
    match color {
        Color::Black => (0, 0, 0),
        Color::Red => (205, 49, 49),
        Color::Green => (13, 188, 121),
        Color::Yellow => (229, 229, 16),
        Color::Blue => (36, 114, 200),
        Color::Magenta => (188, 63, 188),
        Color::Cyan => (17, 168, 205),
        Color::White => (229, 229, 229),
        Color::Rgb(r, g, b) => (*r, *g, *b),
        Color::Indexed(i) => indexed_to_rgb(*i),
    }
}

/// Convert indexed color to approximate RGB
fn indexed_to_rgb(index: u8) -> (u8, u8, u8) {
    match index {
        // Standard colors (0-15)
        0 => (0, 0, 0),       // Black
        1 => (128, 0, 0),     // Maroon
        2 => (0, 128, 0),     // Green
        3 => (128, 128, 0),   // Olive
        4 => (0, 0, 128),     // Navy
        5 => (128, 0, 128),   // Purple
        6 => (0, 128, 128),   // Teal
        7 => (192, 192, 192), // Silver
        8 => (128, 128, 128), // Grey
        9 => (255, 0, 0),     // Red
        10 => (0, 255, 0),    // Lime
        11 => (255, 255, 0),  // Yellow
        12 => (0, 0, 255),    // Blue
        13 => (255, 0, 255),  // Fuchsia
        14 => (0, 255, 255),  // Aqua
        15 => (255, 255, 255), // White
        // 216 color cube (16-231)
        16..=231 => {
            let index = index - 16;
            let r = (index / 36) * 51;
            let g = ((index % 36) / 6) * 51;
            let b = (index % 6) * 51;
            (r, g, b)
        }
        // Grayscale (232-255)
        232..=255 => {
            let gray = 8 + (index - 232) * 10;
            (gray, gray, gray)
        }
    }
}

/// 2D point interpolation
#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Interpolate for Point {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        Point {
            x: self.x.interpolate(&other.x, t),
            y: self.y.interpolate(&other.y, t),
        }
    }
}

/// Rectangle interpolation
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Interpolate for Rect {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        Rect {
            x: self.x.interpolate(&other.x, t),
            y: self.y.interpolate(&other.y, t),
            width: self.width.interpolate(&other.width, t),
            height: self.height.interpolate(&other.height, t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_f32_interpolate() {
        assert_eq!(0.0_f32.interpolate(&10.0, 0.0), 0.0);
        assert_eq!(0.0_f32.interpolate(&10.0, 0.5), 5.0);
        assert_eq!(0.0_f32.interpolate(&10.0, 1.0), 10.0);
    }

    #[test]
    fn test_u8_interpolate() {
        assert_eq!(0_u8.interpolate(&100, 0.0), 0);
        assert_eq!(0_u8.interpolate(&100, 0.5), 50);
        assert_eq!(0_u8.interpolate(&100, 1.0), 100);
    }

    #[test]
    fn test_color_interpolate() {
        let black = Color::Rgb(0, 0, 0);
        let white = Color::Rgb(255, 255, 255);

        let mid = Color::interpolate_color(&black, &white, 0.5);
        if let Color::Rgb(r, g, b) = mid {
            assert!(r > 120 && r < 135);
            assert!(g > 120 && g < 135);
            assert!(b > 120 && b < 135);
        } else {
            panic!("Expected RGB color");
        }
    }

    #[test]
    fn test_point_interpolate() {
        let p1 = Point { x: 0.0, y: 0.0 };
        let p2 = Point { x: 10.0, y: 20.0 };

        let mid = p1.interpolate(&p2, 0.5);
        assert_eq!(mid.x, 5.0);
        assert_eq!(mid.y, 10.0);
    }
}
