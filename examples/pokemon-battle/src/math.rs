use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V3(pub f32, pub f32, pub f32);

impl V3 {
    pub const ZERO: Self = Self(0.0, 0.0, 0.0);
    pub const Y: Self = Self(0.0, 1.0, 0.0);
    pub const Z: Self = Self(0.0, 0.0, 1.0);
    pub fn dot(self, b: Self) -> f32 {
        self.0 * b.0 + self.1 * b.1 + self.2 * b.2
    }
    pub fn cross(self, b: Self) -> Self {
        Self(
            self.1 * b.2 - self.2 * b.1,
            self.2 * b.0 - self.0 * b.2,
            self.0 * b.1 - self.1 * b.0,
        )
    }
    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }
    pub fn unit(self) -> Self {
        let length = self.length();
        if length < 1e-6 {
            Self::Y
        } else {
            self / length
        }
    }
    pub fn lerp(self, b: Self, t: f32) -> Self {
        self * (1.0 - t) + b * t
    }
}

impl Add for V3 {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        Self(self.0 + b.0, self.1 + b.1, self.2 + b.2)
    }
}
impl Sub for V3 {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        Self(self.0 - b.0, self.1 - b.1, self.2 - b.2)
    }
}
impl Mul<f32> for V3 {
    type Output = Self;
    fn mul(self, b: f32) -> Self {
        Self(self.0 * b, self.1 * b, self.2 * b)
    }
}
impl Div<f32> for V3 {
    type Output = Self;
    fn div(self, b: f32) -> Self {
        Self(self.0 / b, self.1 / b, self.2 / b)
    }
}
impl Neg for V3 {
    type Output = Self;
    fn neg(self) -> Self {
        self * -1.0
    }
}

pub fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
pub fn random(index: usize) -> f32 {
    let n = (index as u32)
        .wrapping_mul(747796405)
        .wrapping_add(2891336453);
    let n = ((n >> ((n >> 28) + 4)) ^ n).wrapping_mul(277803737);
    ((n >> 22) ^ n) as f32 / u32::MAX as f32
}
