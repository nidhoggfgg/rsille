//! World-space sample drawing with selectable terminal encoding. All primitives clip before rasterization.
use super::canvas::CanvasContext;
use crate::canvas::Canvas;
use crossterm::style::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanvasPoint {
    pub x: f64,
    pub y: f64,
}
impl CanvasPoint {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    fn finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

/// Cartesian bounds: x grows right and y grows upwards. Width/height must be
/// positive and finite. Scaling the viewport changes the visible world extent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanvasViewport {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}
impl CanvasViewport {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        assert!(
            x.is_finite()
                && y.is_finite()
                && width.is_finite()
                && height.is_finite()
                && width > 0.0
                && height > 0.0
                && (x + width).is_finite()
                && (y + height).is_finite()
                && x + width > x
                && y + height > y,
            "invalid canvas viewport"
        );
        Self {
            x,
            y,
            width,
            height,
        }
    }
    pub fn x(self) -> f64 {
        self.x
    }
    pub fn y(self) -> f64 {
        self.y
    }
    pub fn width(self) -> f64 {
        self.width
    }
    pub fn height(self) -> f64 {
        self.height
    }
    pub fn contains(self, point: CanvasPoint) -> bool {
        point.finite()
            && point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }
    pub fn pan(self, dx: f64, dy: f64) -> Self {
        Self::new(self.x + dx, self.y + dy, self.width, self.height)
    }
    /// A factor above one zooms in while keeping the anchor at the same screen position.
    pub fn zoom_at(self, factor: f64, anchor: CanvasPoint) -> Self {
        assert!(
            factor.is_finite() && factor > 0.0 && anchor.finite(),
            "invalid canvas zoom"
        );
        Self::new(
            anchor.x + (self.x - anchor.x) / factor,
            anchor.y + (self.y - anchor.y) / factor,
            self.width / factor,
            self.height / factor,
        )
    }
}

/// Drawing adapter obtained from `CanvasContext::world`. The raw Canvas API is
/// still available in the original callback for sample-coordinate drawings.
pub struct WorldCanvas<'a> {
    canvas: &'a mut Canvas,
    ctx: CanvasContext,
    color: Option<Color>,
}
impl<'a> WorldCanvas<'a> {
    pub(crate) fn new(canvas: &'a mut Canvas, ctx: CanvasContext) -> Self {
        assert_eq!(
            canvas.encoding(),
            ctx.encoding(),
            "canvas/context encoding mismatch"
        );
        Self {
            canvas,
            ctx,
            color: None,
        }
    }
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }
    fn sample(&mut self, p: CanvasPoint) {
        if !p.finite() {
            return;
        }
        if let Some(color) = self.color {
            self.canvas.set_color(p.x, p.y, color);
        } else {
            self.canvas.set(p.x, p.y);
        }
    }
    pub fn point(&mut self, point: CanvasPoint) -> &mut Self {
        if self.ctx.viewport().contains(point) {
            if let Some(dot) = self.ctx.world_to_sample(point) {
                self.sample(dot);
            }
        }
        self
    }
    pub fn line(&mut self, from: CanvasPoint, to: CanvasPoint) -> &mut Self {
        if !from.finite() || !to.finite() {
            return self;
        }
        // Liang–Barsky clips to the world viewport before converting to pixels.
        let v = self.ctx.viewport();
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        if !dx.is_finite() || !dy.is_finite() {
            return self;
        }
        let (mut enter, mut leave) = (0.0_f64, 1.0_f64);
        for (p, q) in [
            (-dx, from.x - v.x),
            (dx, v.x + v.width - from.x),
            (-dy, from.y - v.y),
            (dy, v.y + v.height - from.y),
        ] {
            if p == 0.0 {
                if q < 0.0 {
                    return self;
                }
            } else {
                let r = q / p;
                if p < 0.0 {
                    enter = enter.max(r);
                } else {
                    leave = leave.min(r);
                }
                if enter > leave {
                    return self;
                }
            }
        }
        let a = self
            .ctx
            .world_to_sample(CanvasPoint::new(from.x + enter * dx, from.y + enter * dy));
        let b = self
            .ctx
            .world_to_sample(CanvasPoint::new(from.x + leave * dx, from.y + leave * dy));
        if let (Some(a), Some(b)) = (a, b) {
            let steps = (b.x - a.x).abs().max((b.y - a.y).abs()).ceil() as usize;
            for i in 0..=steps {
                let t = if steps == 0 {
                    0.0
                } else {
                    i as f64 / steps as f64
                };
                self.sample(CanvasPoint::new(
                    a.x + (b.x - a.x) * t,
                    a.y + (b.y - a.y) * t,
                ));
            }
        }
        self
    }
    pub fn rect(&mut self, bounds: CanvasViewport) -> &mut Self {
        let a = CanvasPoint::new(bounds.x, bounds.y);
        let b = CanvasPoint::new(bounds.x + bounds.width, bounds.y);
        let c = CanvasPoint::new(bounds.x + bounds.width, bounds.y + bounds.height);
        let d = CanvasPoint::new(bounds.x, bounds.y + bounds.height);
        self.line(a, b).line(b, c).line(c, d).line(d, a)
    }
    pub fn circle(&mut self, center: CanvasPoint, radius: f64) -> &mut Self {
        if !center.finite() || !radius.is_finite() || radius <= 0.0 {
            return self;
        }
        let steps =
            (self.ctx.sample_width().max(self.ctx.sample_height()) as usize * 2).clamp(12, 4096);
        let mut previous = CanvasPoint::new(center.x + radius, center.y);
        for i in 1..=steps {
            let angle = std::f64::consts::TAU * i as f64 / steps as f64;
            let next = CanvasPoint::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            );
            self.line(previous, next);
            previous = next;
        }
        self
    }
}
