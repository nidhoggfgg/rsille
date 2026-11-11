mod spatial;

use rsille::raster::{RasterCanvas, Rgb};
use std::{cell::RefCell, sync::Arc};

pub const COLORS: [Rgb; 6] = [
    Rgb(99, 210, 255),
    Rgb(255, 179, 92),
    Rgb(146, 231, 151),
    Rgb(238, 137, 204),
    Rgb(187, 165, 255),
    Rgb(255, 231, 123),
];

const BACKGROUND: Rgb = Rgb(18, 22, 30);
const GRID: Rgb = Rgb(39, 46, 59);
const AXIS: Rgb = Rgb(115, 130, 151);
const TEXT: Rgb = Rgb(174, 187, 205);
pub struct Plot {
    pub formula: crate::formula::Formula,
    functions: Vec<Box<dyn Fn(f64) -> f64>>,
    cache: RefCell<Option<Rendered>>,
    curves: Vec<Curve>,
    spatial: Option<spatial::Scene>,
    survey: RefCell<Option<([f64; 4], [f64; 2])>>,
}

enum Curve {
    Cartesian(usize),
    Geometric {
        sample: Box<dyn Fn(f64) -> (f64, f64)>,
        range: [f64; 2],
    },
}

struct Rendered {
    bounds: [f64; 4],
    size: (u32, u32),
    domain: [f64; 2],
    angle: f64,
    canvas: Arc<RasterCanvas>,
}

impl Plot {
    /// Parses up to six 2D curves or up to six 3D surfaces/curves.
    pub fn parse(source: &str) -> Result<Self, String> {
        let parts: Vec<_> = source.split(';').collect();
        if parts.len() > COLORS.len() {
            return Err("Expected one to six functions separated by semicolons".into());
        }
        let mut functions: Vec<Box<dyn Fn(f64) -> f64>> = Vec::new();
        let mut curves = Vec::new();
        let mut objects = Vec::new();
        for (index, source) in parts.iter().enumerate() {
            let source = source.trim();
            if let Some(object) =
                spatial::parse(source).map_err(|error| format!("Object {}: {error}", index + 1))?
            {
                objects.push(object);
                continue;
            }
            let geometric =
                parse_geometric(source).map_err(|error| format!("Curve {}: {error}", index + 1))?;
            if let Some(curve) = geometric {
                curves.push(curve);
                continue;
            }
            if source.is_empty() {
                return Err(format!("Function {} is empty", index + 1));
            }
            let expression = source
                .parse::<meval::Expr>()
                .map_err(|error| format!("Function {}: {error}", index + 1))?;
            let function = expression
                .bind("x")
                .map_err(|error| format!("Function {}: {error}", index + 1))?;
            curves.push(Curve::Cartesian(functions.len()));
            functions.push(Box::new(function));
        }
        if !objects.is_empty() && !curves.is_empty() {
            return Err("Cannot mix 2D curves and 3D objects in one plot".into());
        }
        Ok(Self {
            spatial: (!objects.is_empty()).then(|| spatial::Scene::new(objects)),
            formula: crate::formula::Formula::new(source),
            functions,
            curves,
            survey: RefCell::new(None),
            cache: RefCell::new(None),
        })
    }

    /// Resolution-independent survey; not a proof of extrema or continuity.
    pub fn auto_bounds(&self, x: [f64; 2]) -> [f64; 4] {
        if let Some(scene) = &self.spatial {
            return scene.bounds();
        }
        if self.has_geometry() {
            return self.geometric_bounds(x);
        }
        let mut low = f64::INFINITY;
        let mut high = f64::NEG_INFINITY;
        for function in &self.functions {
            let mut values: Vec<_> = (0..=1024)
                .map(|i| function(interpolate(x[0], x[1], i as f64 / 1024.0)))
                .filter(|v| v.is_finite())
                .collect();
            if values.is_empty() {
                continue;
            }
            values.sort_by(f64::total_cmp);
            let last = values.len() - 1;
            let q1 = values[last / 4];
            let q3 = values[last * 3 / 4];
            let spread = q3 - q1;
            // Per-function fences prevent a pole from setting the whole scale,
            // without discarding a separate constant or small-amplitude function.
            let (a, b) = if spread > 0.0 && spread.is_finite() {
                (
                    values[0].max(q1 - 3.0 * spread),
                    values[last].min(q3 + 3.0 * spread),
                )
            } else {
                (q1, q3)
            };
            low = low.min(a);
            high = high.max(b);
        }
        if !low.is_finite() || !high.is_finite() {
            return [x[0], x[1], -1.0, 1.0];
        }
        let padding = if low == high {
            if low == 0.0 {
                1.0
            } else {
                low.abs() * 0.05
            }
        } else {
            (high * 0.5 - low * 0.5) * 0.1
        }
        .max(f64::MIN_POSITIVE);
        [
            x[0],
            x[1],
            (low - padding).max(-f64::MAX),
            (high + padding).min(f64::MAX),
        ]
    }

    fn has_geometry(&self) -> bool {
        self.curves
            .iter()
            .any(|c| matches!(c, Curve::Geometric { .. }))
    }

    fn domain(&self, bounds: [f64; 4]) -> [f64; 2] {
        self.survey
            .borrow()
            .as_ref()
            .filter(|(survey, _)| *survey == bounds)
            .map_or([bounds[0], bounds[1]], |(_, domain)| *domain)
    }

    fn geometric_bounds(&self, domain: [f64; 2]) -> [f64; 4] {
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        for curve in &self.curves {
            let mut cx = Vec::new();
            let mut cy = Vec::new();
            for i in 0..=1024 {
                let u = i as f64 / 1024.0;
                let (x, y) = match curve {
                    Curve::Cartesian(index) => {
                        let x = interpolate(domain[0], domain[1], u);
                        (x, (self.functions[*index])(x))
                    }
                    Curve::Geometric { sample, range } => {
                        sample(interpolate(range[0], range[1], u))
                    }
                };
                if x.is_finite() && y.is_finite() {
                    cx.push(x);
                    cy.push(y);
                }
            }
            if let Some((low, high)) = survey_extent(cx) {
                xs.extend([low, high]);
            }
            if let Some((low, high)) = survey_extent(cy) {
                ys.extend([low, high]);
            }
        }
        let mut x = padded_extent(&xs);
        if !self.functions.is_empty() {
            x[0] = x[0].min(domain[0]);
            x[1] = x[1].max(domain[1]);
        }
        let y = padded_extent(&ys);
        let bounds = [x[0], x[1], y[0], y[1]];
        *self.survey.borrow_mut() = Some((bounds, domain));
        bounds
    }

    /// The immutable source owns one cached size/bounds pair. Redraws only clone
    /// the shared canvas; resizing replaces it without reparsing expressions.
    #[cfg(test)]
    pub fn image(&self, bounds: [f64; 4], size: (u32, u32)) -> Arc<RasterCanvas> {
        self.image_at(bounds, size, 0.0)
    }

    pub fn is_spatial(&self) -> bool {
        self.spatial.is_some()
    }

    /// Renders a fixed angle in radians around world z; never advances time.
    /// Nonfinite angles and angles on 2D plots are treated as zero.
    pub fn image_at(&self, bounds: [f64; 4], size: (u32, u32), angle: f64) -> Arc<RasterCanvas> {
        let angle = if self.is_spatial() && angle.is_finite() {
            angle
        } else {
            0.0
        };
        let domain = self.domain(bounds);
        let mut cache = self.cache.borrow_mut();
        if let Some(old) = cache.as_ref() {
            if old.bounds == bounds
                && old.size == size
                && old.domain == domain
                && old.angle == angle
            {
                return Arc::clone(&old.canvas);
            }
        }
        let canvas = Arc::new(match &self.spatial {
            Some(scene) => scene.render_at(bounds, size, angle),
            None => self.render(bounds, size),
        });
        *cache = Some(Rendered {
            bounds,
            size,
            domain,
            angle,
            canvas: Arc::clone(&canvas),
        });
        canvas
    }

    /// Draw directly on the final pixel grid, with finite, ordered bounds.
    fn render(&self, bounds: [f64; 4], size: (u32, u32)) -> RasterCanvas {
        if let Some(scene) = &self.spatial {
            return scene.render(bounds, size);
        }
        let domain = self.domain(bounds);
        let bounds = if self.has_geometry() {
            equal_bounds(bounds, size)
        } else {
            bounds
        };
        let mut canvas =
            RasterCanvas::new(size.0, size.1, BACKGROUND).expect("graph allocation failed");
        let view = View::new(bounds, size);
        let labels = size.0 >= 160 && size.1 >= 80;
        let nx = ((view.right - view.left) as usize / if labels { 48 } else { 12 }).clamp(1, 8);
        let ny = ((view.bottom - view.top) as usize / if labels { 24 } else { 8 }).clamp(1, 8);
        let (xticks, xstep) = ticks(bounds[0], bounds[1], nx);
        let (yticks, ystep) = ticks(bounds[2], bounds[3], ny);
        for value in xticks {
            let t = fraction(value, bounds[0], bounds[1]);
            let x = view.left + t * (view.right - view.left);
            canvas.line(
                x as i32,
                view.top as i32,
                x as i32,
                view.bottom as i32,
                GRID,
            );
            if labels {
                let label = tick(value, xstep);
                let x = (x as i32 - label.len() as i32 * 2)
                    .clamp(0, (size.0 as i32 - label.len() as i32 * 4).max(0));
                text(&mut canvas, x, view.bottom as i32 + 5, &label);
            }
        }
        for value in yticks {
            let t = fraction(value, bounds[2], bounds[3]);
            let y = view.bottom - t * (view.bottom - view.top);
            canvas.line(
                view.left as i32,
                y as i32,
                view.right as i32,
                y as i32,
                GRID,
            );
            if labels {
                let label = tick(value, ystep);
                if label.len() * 4 + 4 <= view.left as usize {
                    text(
                        &mut canvas,
                        view.left as i32 - 4 - label.len() as i32 * 4,
                        y as i32 - 2,
                        &label,
                    );
                }
            }
        }
        if bounds[0] <= 0.0 && bounds[1] >= 0.0 {
            let x = view.left + fraction(0.0, bounds[0], bounds[1]) * (view.right - view.left);
            canvas.line(
                x.round() as i32,
                view.top as i32,
                x.round() as i32,
                view.bottom as i32,
                AXIS,
            );
        }
        if bounds[2] <= 0.0 && bounds[3] >= 0.0 {
            let y = view.y(0.0).round() as i32;
            canvas.line(view.left as i32, y, view.right as i32, y, AXIS);
        }
        if self.has_geometry() {
            for (curve, color) in self.curves.iter().zip(COLORS) {
                let mut stroke = Stroke::new(size);
                match curve {
                    Curve::Cartesian(index) => {
                        draw_parametric(
                            &mut stroke,
                            &|t| (t, (self.functions[*index])(t)),
                            domain,
                            &view,
                        );
                    }
                    Curve::Geometric { sample, range } => {
                        draw_parametric(&mut stroke, sample, *range, &view);
                    }
                }
                stroke.composite(&mut canvas, color);
            }
            return canvas;
        }
        for (function, color) in self.functions.iter().zip(COLORS) {
            let mut stroke = Stroke::new(size);
            let sample = |pixel| view.y(function(view.x(pixel)));
            let mut a = (view.left, sample(view.left));
            if view.left == view.right && a.1.is_finite() && (view.top..=view.bottom).contains(&a.1)
            {
                stroke.line(a, a, &view);
            }
            for column in view.left as i32..view.right as i32 {
                let x = f64::from(column + 1);
                let b = (x, sample(x));
                // A separate budget per output pixel keeps poles from starving
                // the rest of the curve (at most 128 midpoint evaluations).
                let mut budget = 128;
                segment(&mut stroke, &sample, a, b, 0, &mut budget, &view);
                a = b;
            }
            stroke.composite(&mut canvas, color);
        }
        canvas
    }
}

fn bind(source: &str, variable: &str) -> Result<Box<dyn Fn(f64) -> f64>, String> {
    let expr = source.parse::<meval::Expr>().map_err(|e| e.to_string())?;
    Ok(Box::new(expr.bind(variable).map_err(|e| e.to_string())?))
}

fn parse_geometric(source: &str) -> Result<Option<Curve>, String> {
    let name = source
        .split(|c: char| !c.is_ascii_alphabetic())
        .next()
        .unwrap_or("");
    if name != "polar" && name != "param" {
        return Ok(None);
    }
    let body = source[name.len()..]
        .trim()
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| format!("Expected {name}(expressions, min, max)"))?;
    let mut depth = 0usize;
    let mut start = 0;
    let mut args = Vec::new();
    for (i, c) in body.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or("Unmatched closing parenthesis")?
            }
            ',' if depth == 0 => {
                args.push(body[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err("Unclosed parenthesis".into());
    }
    args.push(body[start..].trim());
    let expected = if name == "polar" { 3 } else { 4 };
    if args.len() != expected || args.iter().any(|s| s.is_empty()) {
        return Err(format!(
            "{name} requires {} expressions and explicit min, max bounds",
            expected - 2
        ));
    }
    let mut range = [0.0; 2];
    for i in 0..2 {
        range[i] = meval::eval_str(args[expected - 2 + i])
            .map_err(|e| format!("Invalid constant range bound: {e}"))?;
    }
    if !range.iter().all(|v| v.is_finite())
        || range[0] >= range[1]
        || !(range[1] - range[0]).is_finite()
    {
        return Err("Range bounds must be finite, increasing, with a finite span".into());
    }
    let sample: Box<dyn Fn(f64) -> (f64, f64)> = if name == "polar" {
        let radius = bind(args[0], "theta").map_err(|e| format!("Radius: {e}"))?;
        Box::new(move |theta| {
            let r = radius(theta);
            (r * theta.cos(), r * theta.sin())
        })
    } else {
        let x = bind(args[0], "t").map_err(|e| format!("x(t): {e}"))?;
        let y = bind(args[1], "t").map_err(|e| format!("y(t): {e}"))?;
        Box::new(move |t| (x(t), y(t)))
    };
    Ok(Some(Curve::Geometric { sample, range }))
}

fn survey_extent(mut values: Vec<f64>) -> Option<(f64, f64)> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let last = values.len() - 1;
    let (q1, q3) = (values[last / 4], values[last * 3 / 4]);
    let spread = q3 - q1;
    Some(if spread > 0.0 && spread.is_finite() {
        (
            values[0].max(q1 - 3.0 * spread),
            values[last].min(q3 + 3.0 * spread),
        )
    } else {
        (q1, q3)
    })
}

fn padded_extent(values: &[f64]) -> [f64; 2] {
    if values.is_empty() {
        return [-1.0, 1.0];
    }
    let low = values.iter().copied().fold(f64::INFINITY, f64::min);
    let high = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let padding = if low == high {
        if low == 0.0 {
            1.0
        } else {
            low.abs() * 0.05
        }
    } else {
        (high * 0.5 - low * 0.5) * 0.1
    }
    .max(f64::MIN_POSITIVE);
    [
        (low - padding).max(-f64::MAX),
        (high + padding).min(f64::MAX),
    ]
}

// Only the viewport grows; parameter ranges and the Cartesian domain do not.
fn equal_bounds(mut bounds: [f64; 4], size: (u32, u32)) -> [f64; 4] {
    let view = View::new(bounds, size);
    let width = (view.right - view.left).max(1.0);
    let height = (view.bottom - view.top).max(1.0);
    let half_x = bounds[1] * 0.5 - bounds[0] * 0.5;
    let half_y = bounds[3] * 0.5 - bounds[2] * 0.5;
    let unit = (half_x / width).max(half_y / height);
    for (axis, pixels) in [(0, width), (2, height)] {
        let middle = bounds[axis] * 0.5 + bounds[axis + 1] * 0.5;
        let half = unit * pixels;
        let (low, high) = (middle - half, middle + half);
        if low.is_finite() && high.is_finite() {
            bounds[axis] = bounds[axis].min(low);
            bounds[axis + 1] = bounds[axis + 1].max(high);
        }
    }
    bounds
}

type Point = (f64, f64);

fn draw_parametric(
    stroke: &mut Stroke,
    sample: &impl Fn(f64) -> Point,
    range: [f64; 2],
    view: &View,
) {
    let pixel = |t| {
        let (x, y) = sample(t);
        (
            view.left + fraction(x, view.bounds[0], view.bounds[1]) * (view.right - view.left),
            view.y(y),
        )
    };
    let seeds = ((view.right - view.left).max(view.bottom - view.top) as usize).clamp(64, 4096);
    let mut ta = range[0];
    let mut a = pixel(ta);
    for i in 1..=seeds {
        let tb = interpolate(range[0], range[1], i as f64 / seeds as f64);
        let b = pixel(tb);
        // Local budgets prevent one singularity from starving later intervals.
        param_segment(stroke, &pixel, (ta, a), (tb, b), 0, &mut 128, view);
        ta = tb;
        a = b;
    }
}

fn param_segment(
    stroke: &mut Stroke,
    sample: &impl Fn(f64) -> Point,
    a: (f64, Point),
    b: (f64, Point),
    depth: u8,
    budget: &mut usize,
    view: &View,
) {
    if *budget == 0 {
        return;
    }
    *budget -= 1;
    let t = interpolate(a.0, b.0, 0.5);
    let m = sample(t);
    let finite = |p: Point| p.0.is_finite() && p.1.is_finite();
    if finite(a.1) && finite(m) && finite(b.1) {
        let error = (m.0 - (a.1 .0 * 0.5 + b.1 .0 * 0.5))
            .abs()
            .max((m.1 - (a.1 .1 * 0.5 + b.1 .1 * 0.5)).abs());
        let step = (a.1 .0 - b.1 .0).abs().max((a.1 .1 - b.1 .1).abs());
        if error <= 0.5 && step <= 24.0 {
            clipped_line(stroke, a.1, m, view);
            clipped_line(stroke, m, b.1, view);
            return;
        }
    } else if !finite(a.1) && !finite(m) && !finite(b.1) {
        return;
    }
    // Never join unresolved jumps, poles or domain boundaries at the limit.
    if depth < 10 && t > a.0 && t < b.0 {
        param_segment(stroke, sample, a, (t, m), depth + 1, budget, view);
        param_segment(stroke, sample, (t, m), b, depth + 1, budget, view);
    }
}

struct View {
    bounds: [f64; 4],
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

impl View {
    fn new(bounds: [f64; 4], size: (u32, u32)) -> Self {
        let labels = size.0 >= 160 && size.1 >= 80;
        Self {
            bounds,
            left: if labels { 60.0 } else { 0.0 },
            right: f64::from(size.0.saturating_sub(if labels { 8 } else { 1 })),
            top: if labels { 4.0 } else { 0.0 },
            bottom: f64::from(size.1.saturating_sub(if labels { 14 } else { 1 })),
        }
    }

    fn x(&self, pixel: f64) -> f64 {
        interpolate(
            self.bounds[0],
            self.bounds[1],
            (pixel - self.left) / (self.right - self.left).max(1.0),
        )
    }

    fn y(&self, value: f64) -> f64 {
        self.bottom - fraction(value, self.bounds[2], self.bounds[3]) * (self.bottom - self.top)
    }
}

fn interpolate(low: f64, high: f64, t: f64) -> f64 {
    if (high - low).is_finite() {
        low + (high - low) * t
    } else {
        low * (1.0 - t) + high * t
    }
}

fn fraction(value: f64, low: f64, high: f64) -> f64 {
    let span = high - low;
    if span.is_finite() {
        (value - low) / span
    } else {
        (value * 0.5 - low * 0.5) / (high * 0.5 - low * 0.5)
    }
}

// Finite sampling cannot prove continuity. Small seed intervals, midpoint
// flatness and a maximum accepted vertical step conservatively reject poles
// and jumps; unresolved intervals are never joined at the recursion limit.
fn segment(
    stroke: &mut Stroke,
    sample: &impl Fn(f64) -> f64,
    a: (f64, f64),
    b: (f64, f64),
    depth: u8,
    budget: &mut usize,
    view: &View,
) {
    if *budget == 0 {
        return;
    }
    *budget -= 1;
    let m = ((a.0 + b.0) * 0.5, sample((a.0 + b.0) * 0.5));
    let finite = a.1.is_finite() && m.1.is_finite() && b.1.is_finite();
    if finite {
        if (a.1 < view.top && m.1 < view.top && b.1 < view.top)
            || (a.1 > view.bottom && m.1 > view.bottom && b.1 > view.bottom)
        {
            return;
        }
        let error = (m.1 - (a.1 * 0.5 + b.1 * 0.5)).abs();
        if error <= 0.5 && (a.1 - b.1).abs() <= 24.0 {
            clipped_line(stroke, a, m, view);
            clipped_line(stroke, m, b, view);
            return;
        }
    } else if !a.1.is_finite() && !m.1.is_finite() && !b.1.is_finite() {
        return;
    }
    if depth < 10 {
        segment(stroke, sample, a, m, depth + 1, budget, view);
        segment(stroke, sample, m, b, depth + 1, budget, view);
    }
}

fn clipped_line(stroke: &mut Stroke, mut a: (f64, f64), mut b: (f64, f64), view: &View) {
    if (a.1 < view.top && b.1 < view.top) || (a.1 > view.bottom && b.1 > view.bottom) {
        return;
    }
    if ![a.0, a.1, b.0, b.1].iter().all(|v| v.is_finite()) {
        return;
    }
    if (a.0 < view.left && b.0 < view.left) || (a.0 > view.right && b.0 > view.right) {
        return;
    }
    // Clip both axes in floating point before rasterizing.
    if a.0 < view.left || a.0 > view.right {
        let x = a.0.clamp(view.left, view.right);
        a.1 = interpolate(a.1, b.1, fraction(x, a.0, b.0));
        a.0 = x;
    }
    if b.0 < view.left || b.0 > view.right {
        let x = b.0.clamp(view.left, view.right);
        b.1 = interpolate(a.1, b.1, fraction(x, a.0, b.0));
        b.0 = x;
    }
    if (a.1 < view.top && b.1 < view.top) || (a.1 > view.bottom && b.1 > view.bottom) {
        return;
    }
    // Clip in floating point before rounding, retaining the graph margins.
    if a.1 < view.top || a.1 > view.bottom {
        let y = a.1.clamp(view.top, view.bottom);
        a.0 += (b.0 - a.0) * ((y - a.1) / (b.1 - a.1));
        a.1 = y;
    }
    if b.1 < view.top || b.1 > view.bottom {
        let y = b.1.clamp(view.top, view.bottom);
        b.0 = a.0 + (b.0 - a.0) * ((y - a.1) / (b.1 - a.1));
        b.1 = y;
    }
    stroke.line(a, b, view);
}

// A 1.5-pixel round stroke with a one-pixel coverage ramp. Keep floating
// endpoints: rounding before coverage calculation recreates the staircase.
struct Stroke {
    width: u32,
    coverage: Vec<f32>,
}

impl Stroke {
    fn new(size: (u32, u32)) -> Self {
        Self {
            width: size.0,
            coverage: vec![0.0; size.0 as usize * size.1 as usize],
        }
    }

    fn line(&mut self, a: (f64, f64), b: (f64, f64), view: &View) {
        let radius = 1.25;
        let left = (a.0.min(b.0) - radius).ceil().max(view.left) as i32;
        let right = (a.0.max(b.0) + radius).floor().min(view.right) as i32;
        let top = (a.1.min(b.1) - radius).ceil().max(view.top) as i32;
        let bottom = (a.1.max(b.1) + radius).floor().min(view.bottom) as i32;
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        let length2 = dx * dx + dy * dy;
        for y in top..=bottom {
            for x in left..=right {
                let px = f64::from(x) - a.0;
                let py = f64::from(y) - a.1;
                let t = if length2 == 0.0 {
                    0.0
                } else {
                    ((px * dx + py * dy) / length2).clamp(0.0, 1.0)
                };
                let distance = (px - t * dx).hypot(py - t * dy);
                let alpha = (radius - distance).clamp(0.0, 1.0) as f32;
                let coverage = &mut self.coverage[y as usize * self.width as usize + x as usize];
                // Union coverage, not repeated alpha-over at subdivision joints.
                *coverage = coverage.max(alpha);
            }
        }
    }

    fn composite(&self, canvas: &mut RasterCanvas, color: Rgb) {
        for (index, &alpha) in self.coverage.iter().enumerate() {
            if alpha == 0.0 {
                continue;
            }
            let old = canvas.pixels()[index];
            let blend =
                |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * alpha).round() as u8;
            canvas.set_pixel(
                (index % self.width as usize) as i32,
                (index / self.width as usize) as i32,
                Rgb(
                    blend(old.0, color.0),
                    blend(old.1, color.1),
                    blend(old.2, color.2),
                ),
            );
        }
    }
}

fn ticks(low: f64, high: f64, count: usize) -> (Vec<f64>, f64) {
    let raw = high / count as f64 - low / count as f64;
    let power = 10.0_f64.powf(raw.log10().floor());
    let factor = raw / power;
    let nice = if factor <= 1.0 {
        1.0
    } else if factor <= 2.0 {
        2.0
    } else if factor <= 5.0 {
        5.0
    } else {
        10.0
    };
    let step = nice * power;
    if !step.is_finite() || step <= 0.0 || !(low / step).is_finite() {
        return (
            (0..=count)
                .map(|i| interpolate(low, high, i as f64 / count as f64))
                .collect(),
            raw,
        );
    }
    let first = (low / step).ceil();
    let values = (0..=count + 1)
        .map(|i| (first + i as f64) * step)
        .filter(|v| v.is_finite() && *v >= low && *v <= high)
        .collect();
    (values, step)
}

fn tick(value: f64, step: f64) -> String {
    if value == 0.0 {
        return "0".into();
    }
    let decimals = (-step.abs().log10().floor()).max(0.0) as usize;
    if value.abs() >= 1e5 || value.abs() < 0.001 {
        let precision =
            (value.abs().log10().floor() - step.abs().log10().floor()).clamp(0.0, 15.0) as usize;
        format!("{value:.precision$e}")
    } else if decimals == 0 {
        format!("{value:.0}")
    } else {
        format!("{value:.prec$}", prec = decimals.min(16))
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    }
}

fn text(canvas: &mut RasterCanvas, x: i32, y: i32, label: &str) {
    for (i, ch) in label.chars().enumerate() {
        let rows = match ch {
            '0' => [7, 5, 5, 5, 7],
            '1' => [2, 6, 2, 2, 7],
            '2' => [7, 1, 7, 4, 7],
            '3' => [7, 1, 7, 1, 7],
            '4' => [5, 5, 7, 1, 1],
            '5' => [7, 4, 7, 1, 7],
            '6' => [7, 4, 7, 5, 7],
            '7' => [7, 1, 2, 2, 2],
            '8' => [7, 5, 7, 5, 7],
            '9' => [7, 5, 7, 1, 7],
            '-' => [0, 0, 7, 0, 0],
            '+' => [0, 2, 7, 2, 0],
            '.' => [0, 0, 0, 0, 2],
            'e' => [0, 7, 7, 4, 7],
            _ => [0; 5],
        };
        for (row, bits) in rows.iter().enumerate() {
            for column in 0..3 {
                if bits & (1 << (2 - column)) != 0 {
                    canvas.set_pixel(x + i as i32 * 4 + column, y + row as i32, TEXT);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn presented(canvas: RasterCanvas, columns: u16, cell: (u16, u16)) -> Vec<Vec<Rgb>> {
        use rsille::raster::{RasterAlign, RasterCell, RasterOptions};
        let mut sample = RasterCell {
            canvas,
            target: (columns, 14).into(),
            x: 0,
            y: 0,
            options: RasterOptions {
                align_x: RasterAlign::Start,
                ..Default::default()
            },
        };
        (0..14 * cell.1)
            .map(|y| {
                (0..columns * cell.0)
                    .map(|x| {
                        sample.x = x / cell.0;
                        sample.y = y / cell.1;
                        sample.sample(u32::from(x % cell.0), u32::from(y % cell.1), cell.0, cell.1)
                    })
                    .collect()
            })
            .collect()
    }

    // Cyan contribution, not exact full-coverage color. All underlay colors
    // have blue-green <= 21; even modest stroke coverage is distinguishable.
    fn ink(p: Rgb) -> bool {
        i16::from(p.2) - i16::from(p.1) > 24
    }

    fn mask(pixels: Vec<Vec<Rgb>>) -> Vec<Vec<bool>> {
        pixels
            .into_iter()
            .map(|row| row.into_iter().map(ink).collect())
            .collect()
    }

    fn components(mut pixels: Vec<Vec<bool>>) -> usize {
        let mut count = 0;
        for y in 0..pixels.len() {
            for x in 0..pixels[0].len() {
                if !pixels[y][x] {
                    continue;
                }
                count += 1;
                let mut pending = vec![(x, y)];
                pixels[y][x] = false;
                while let Some((x, y)) = pending.pop() {
                    for ny in y.saturating_sub(1)..=(y + 1).min(pixels.len() - 1) {
                        for nx in x.saturating_sub(1)..=(x + 1).min(pixels[0].len() - 1) {
                            if pixels[ny][nx] {
                                pixels[ny][nx] = false;
                                pending.push((nx, ny));
                            }
                        }
                    }
                }
            }
        }
        count
    }

    #[test]
    fn target_pixels_are_presented_without_resampling() {
        for columns in [1, 30, 70] {
            for cell in [(8, 16), (10, 20), (18, 40), (20, 40), (1, 2)] {
                let size = (u32::from(columns * cell.0), 14 * u32::from(cell.1));
                for source in ["sin(x)", "x", "x/5", "0.37"] {
                    let canvas = Plot::parse(source).unwrap().render(crate::BOUNDS, size);
                    assert_eq!((canvas.width(), canvas.height()), size);
                    let expected: Vec<Vec<Rgb>> = canvas
                        .pixels()
                        .chunks_exact(size.0 as usize)
                        .map(|row| row.to_vec())
                        .collect();
                    let actual = presented(canvas, columns, cell);
                    assert_eq!(actual, expected);
                    if columns > 1 {
                        assert_eq!(components(mask(actual.clone())), 1, "{source} {size:?}");
                        assert!(actual.iter().flatten().copied().any(ink));
                    }
                }
            }
        }
    }

    #[test]
    fn target_resolution_preserves_poles_and_domain_gaps() {
        for (source, bounds) in [
            ("1/x", [-2.0, 2.0, -4.0, 4.0]),
            ("1/(x-0.0037)", [-2.0, 2.0, -4.0, 4.0]),
            ("sqrt(x^2-1)", [-2.0, 2.0, -1.0, 2.0]),
        ] {
            for columns in [30, 70] {
                for cell in [(8, 16), (10, 20), (18, 40), (20, 40), (1, 2)] {
                    let size = (u32::from(columns * cell.0), 14 * u32::from(cell.1));
                    let canvas = Plot::parse(source).unwrap().render(bounds, size);
                    assert_eq!(
                        components(mask(presented(canvas, columns, cell))),
                        2,
                        "{source} {size:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn square_root_stays_in_domain_at_each_target_size() {
        let bounds = [-2.0, 2.0, -1.0, 2.0];
        for size in [(30, 28), (70, 28), (240, 224), (560, 224)] {
            let canvas = Plot::parse("sqrt(x)").unwrap().render(bounds, size);
            let view = View::new(bounds, size);
            let zero = (view.left + view.right) * 0.5;
            for row in canvas.pixels().chunks_exact(size.0 as usize) {
                // Round caps/AA may extend at most 1.25 pixels past the domain.
                assert!(!row[..(zero - 1.25).ceil() as usize]
                    .iter()
                    .copied()
                    .any(ink));
            }
            let x = (view.left + (view.right - view.left) * 0.75).round() as i32;
            let y = view.y(1.0).round() as i32;
            assert!((-1..=1).any(|dy| (-1..=1).any(|dx| {
                ink(canvas.pixels()[((y + dy) as u32 * size.0 + (x + dx) as u32) as usize])
            })));
        }
    }

    #[test]
    fn stroke_coverage_is_solid_at_all_slopes_and_subpixel_offsets() {
        let size = (80, 80);
        let view = View::new(crate::BOUNDS, size);
        for end in [(70.0, 12.0), (70.0, 70.0), (25.0, 70.0), (70.0, 25.0)] {
            for offset in [0.0, 0.25, 0.5, 0.75] {
                let a = (10.0, 12.0 + offset);
                let b = (end.0, end.1 + offset);
                let mut stroke = Stroke::new(size);
                stroke.line(a, b, &view);
                let steep = (b.1 - a.1).abs() > (b.0 - a.0).abs();
                // Every interior major-axis slice has a bright core and fringe;
                // integrated coverage must not collapse at staircase transitions.
                for major in 14..(if steep { b.1 } else { b.0 }) as usize - 2 {
                    let values: Vec<_> = (0..80)
                        .map(|minor| {
                            let (x, y) = if steep {
                                (minor, major)
                            } else {
                                (major, minor)
                            };
                            stroke.coverage[y * 80 + x]
                        })
                        .collect();
                    assert!(values.iter().copied().fold(0.0_f32, f32::max) >= 0.74);
                    assert!(values.iter().sum::<f32>() >= 1.49);
                    assert!(values.iter().any(|&v| v > 0.0 && v < 1.0));
                }
                let mut split = Stroke::new(size);
                let m = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
                split.line(a, m, &view);
                split.line(m, b, &view);
                for (whole, parts) in stroke.coverage.iter().zip(&split.coverage) {
                    assert!((whole - parts).abs() < 1e-6);
                }
            }
        }
    }

    #[test]
    fn coverage_blends_over_grid_and_axes_without_repeated_joint_blending() {
        for background in [BACKGROUND, GRID, AXIS, COLORS[1]] {
            let mut canvas = RasterCanvas::new(20, 20, background).unwrap();
            let mut stroke = Stroke::new((20, 20));
            let view = View::new(crate::BOUNDS, (20, 20));
            stroke.line((2.0, 10.0), (17.0, 10.0), &view);
            stroke.line((2.0, 10.0), (17.0, 10.0), &view);
            stroke.composite(&mut canvas, COLORS[0]);
            assert_eq!(canvas.pixels()[10 * 20 + 8], COLORS[0]);
            let blend = |a: u8, b: u8| (f32::from(a) * 0.75 + f32::from(b) * 0.25).round() as u8;
            assert_eq!(
                canvas.pixels()[9 * 20 + 8],
                Rgb(
                    blend(background.0, COLORS[0].0),
                    blend(background.1, COLORS[0].1),
                    blend(background.2, COLORS[0].2),
                )
            );
            assert_eq!(canvas.pixels()[8 * 20 + 8], background);
        }
    }

    #[cfg(feature = "sixel")]
    mod sixel_tests {
        // Keep the external legacy-render fixture's two-field literal intact.
        struct Plot {
            functions: Vec<Box<dyn Fn(f64) -> f64>>,
            cache: RefCell<Option<Rendered>>,
        }

        impl Plot {
            fn parse(source: &str) -> Result<super::Plot, String> {
                super::Plot::parse(source)
            }

            fn render(self, bounds: [f64; 4], size: (u32, u32)) -> RasterCanvas {
                super::Plot {
                    formula: crate::formula::Formula::new(""),
                    curves: (0..self.functions.len()).map(Curve::Cartesian).collect(),
                    functions: self.functions,
                    cache: self.cache,
                    spatial: None,
                    survey: RefCell::new(None),
                }
                .render(bounds, size)
            }
        }

        include!("sixel_tests.rs");
    }

    #[test]
    fn cache_reuses_evaluations_and_invalidates_size_and_bounds() {
        let calls = std::rc::Rc::new(std::cell::Cell::new(0));
        let count = calls.clone();
        let plot = Plot {
            formula: crate::formula::Formula::new("sin(x)"),
            functions: vec![Box::new(move |x| {
                count.set(count.get() + 1);
                x.sin()
            })],
            cache: RefCell::new(None),
            curves: vec![Curve::Cartesian(0)],
            spatial: None,
            survey: RefCell::new(None),
        };
        let first = plot.image(crate::BOUNDS, (70, 28));
        let evaluated = calls.get();
        assert!(evaluated > 0);
        assert!(Arc::ptr_eq(&first, &plot.image(crate::BOUNDS, (70, 28))));
        assert_eq!(calls.get(), evaluated);
        let resized = plot.image(crate::BOUNDS, (30, 28));
        assert_eq!((resized.width(), resized.height()), (30, 28));
        assert!(calls.get() > evaluated);
        let changed = plot.image([-2.0, 2.0, -2.0, 2.0], (30, 28));
        assert!(!Arc::ptr_eq(&resized, &changed));
        let other = Plot::parse("cos(x)")
            .unwrap()
            .image(crate::BOUNDS, (70, 28));
        assert_ne!(first.pixels(), other.pixels());
    }

    #[test]
    fn auto_scale_small_amplitudes_constants_and_multiple_functions() {
        for source in ["0.05+0.05*sin(x)", "-0.05+0.05*sin(x)", "1e-8*sin(x)"] {
            let plot = Plot::parse(source).unwrap();
            let bounds = plot.auto_bounds(crate::X_RANGE);
            let amplitude = if source.starts_with("1e") { 1e-8 } else { 0.05 };
            assert!(bounds[3] - bounds[2] < amplitude * 2.3, "{bounds:?}");
            assert!(bounds[3] - bounds[2] > amplitude * 2.0);
            assert!(plot
                .image(bounds, (560, 224))
                .pixels()
                .iter()
                .copied()
                .any(ink));
        }
        for source in ["0", "0.03", "-0.03", "1e300", "-1e300", "1e-300"] {
            let plot = Plot::parse(source).unwrap();
            let bounds = plot.auto_bounds(crate::X_RANGE);
            let value = (plot.functions[0])(0.0);
            assert!(bounds[2].is_finite() && bounds[3].is_finite());
            assert!(
                bounds[2] < value && bounds[3] > value,
                "{source}: {bounds:?}"
            );
        }
        let bounds = Plot::parse("0.01*sin(x); -0.2; 0.3")
            .unwrap()
            .auto_bounds(crate::X_RANGE);
        assert!(bounds[2] < -0.2 && bounds[3] > 0.3);
        assert!(bounds[3] - bounds[2] < 0.6);
    }

    #[test]
    fn auto_scale_poles_nonfinite_and_explicit_domain() {
        for source in ["1/x", "1/(x-0.0037)", "tan(x)"] {
            let bounds = Plot::parse(source).unwrap().auto_bounds(crate::X_RANGE);
            assert!(bounds[2] < 0.0 && bounds[3] > 0.0);
            assert!(
                bounds[2] > -20.0 && bounds[3] < 20.0,
                "{source}: {bounds:?}"
            );
        }
        let empty = Plot::parse("sqrt(-1); 1/0")
            .unwrap()
            .auto_bounds([2.0, 4.0]);
        assert_eq!(empty, [2.0, 4.0, -1.0, 1.0]);
        let mixed = Plot::parse("sqrt(-1); 0.02")
            .unwrap()
            .auto_bounds(crate::X_RANGE);
        assert!(mixed[2] < 0.02 && mixed[3] > 0.02 && mixed[3] < 0.03);
        let domain = Plot::parse("x^2; sqrt(x)").unwrap().auto_bounds([2.0, 4.0]);
        assert_eq!(&domain[..2], &[2.0, 4.0]);
        assert!(domain[2] < 2.0_f64.sqrt() && domain[3] > 16.0);
    }

    #[test]
    fn range_ticks_and_labels_resolve_small_scales() {
        for (low, high) in [(0.0, 0.1), (-0.1, 0.0), (-1e-9, 1e-9), (0.001, 0.00101)] {
            let (values, step) = ticks(low, high, 5);
            assert!(values.len() >= 2 && values.len() <= 7);
            let labels: Vec<_> = values.iter().map(|v| tick(*v, step)).collect();
            assert!(labels.windows(2).all(|w| w[0] != w[1]));
            for (value, label) in values.iter().zip(labels) {
                let parsed: f64 = label.parse().unwrap();
                assert!((parsed - value).abs() < step * 0.01);
            }
        }
        assert_eq!(tick(-0.0, 0.02), "0");
        assert_eq!(tick(0.02, 0.02), "0.02");
    }

    #[test]
    fn geometric_parser_nested_arguments_and_indexed_errors() {
        let plot = Plot::parse("x; polar(max(1,min(2,3)),0,2*pi); param(max(t,0),min(t,1),-pi,pi)")
            .unwrap();
        assert_eq!(plot.curves.len(), 3);
        assert_eq!(plot.functions.len(), 1);
        if let Curve::Geometric { sample, range } = &plot.curves[1] {
            assert_eq!(*range, [0.0, 2.0 * std::f64::consts::PI]);
            let p = sample(std::f64::consts::FRAC_PI_2);
            assert!(p.0.abs() < 1e-12 && (p.1 - 2.0).abs() < 1e-12);
        } else {
            panic!("expected polar curve");
        }
        for bad in [
            "polar(1)",
            "polar(1,0)",
            "polar(1,0,1,2)",
            "param(t,t,0)",
            "polar(x,0,1)",
            "param(theta,t,0,1)",
            "param(t,x,0,1)",
            "polar(1,t,1)",
            "polar(1,0,1/0)",
            "polar(1,0,sqrt(-1))",
            "polar(1,1,1)",
            "polar(1,2,1)",
            "polar(1,-1e308,1e308)",
            "polar(,0,1)",
            "param(t,,0,1)",
            "polar(1,0,1)+2",
            "polar(max(1,2),0,1))",
            "param(max(t,1),min(t,2),0,(1)",
        ] {
            let error = Plot::parse(&format!("x; {bad}")).err().expect(bad);
            assert!(error.contains("2"), "{bad}: {error}");
        }
        assert!(Plot::parse("polar(1,0,1);x;x;x;x;x").is_ok());
        assert!(Plot::parse("polar(1,0,1);x;x;x;x;x;x").is_err());
    }

    #[test]
    fn geometric_survey_both_axes_and_mixed_domain() {
        let bounds = Plot::parse("param(10+2*cos(t),-3+sin(t),0,2*pi)")
            .unwrap()
            .auto_bounds([-100.0, 100.0]);
        assert!(bounds[0] < 8.0 && bounds[0] > 7.0 && bounds[1] > 12.0 && bounds[1] < 13.0);
        assert!(bounds[2] < -4.0 && bounds[3] > -2.0);
        let plot = Plot::parse("x^2; param(20+t,t,0,1)").unwrap();
        let bounds = plot.auto_bounds([2.0, 4.0]);
        assert!(bounds[0] <= 2.0 && bounds[1] > 21.0);
        assert!(bounds[3] > 16.0 && bounds[3] < 20.0, "{bounds:?}");
        assert_eq!(plot.domain(bounds), [2.0, 4.0]);
        assert_eq!(plot.domain([-1.0, 1.0, -1.0, 1.0]), [-1.0, 1.0]);
        assert_eq!(
            Plot::parse("param(sqrt(-1),t,0,1)")
                .unwrap()
                .auto_bounds([2.0, 4.0]),
            [-1.0, 1.0, -1.0, 1.0]
        );
        let constants = Plot::parse("param(3,-2,0,1)")
            .unwrap()
            .auto_bounds([0.0, 1.0]);
        assert!(
            constants[0] < 3.0 && constants[1] > 3.0 && constants[2] < -2.0 && constants[3] > -2.0
        );
    }

    #[test]
    fn circles_are_round_connected_and_closed_on_pixel_grid() {
        for source in ["polar(1,0,2*pi)", "param(cos(t),sin(t),0,2*pi)"] {
            let plot = Plot::parse(source).unwrap();
            let bounds = plot.auto_bounds(crate::X_RANGE);
            for size in [(70, 28), (240, 224), (560, 224), (100, 300)] {
                let canvas = plot.render(bounds, size);
                let pixels: Vec<Vec<_>> = canvas
                    .pixels()
                    .chunks_exact(size.0 as usize)
                    .map(|r| r.to_vec())
                    .collect();
                assert_eq!(components(mask(pixels)), 1, "{source} {size:?}");
                let points: Vec<_> = canvas
                    .pixels()
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| ink(**p))
                    .map(|(i, _)| ((i % size.0 as usize) as i32, (i / size.0 as usize) as i32))
                    .collect();
                let width = points.iter().map(|p| p.0).max().unwrap()
                    - points.iter().map(|p| p.0).min().unwrap();
                let height = points.iter().map(|p| p.1).max().unwrap()
                    - points.iter().map(|p| p.1).min().unwrap();
                assert!((width - height).abs() <= 2, "{size:?}: {width} x {height}");
                let view = View::new(equal_bounds(bounds, size), size);
                let scale_x = (view.right - view.left) / (view.bounds[1] - view.bounds[0]);
                let scale_y = (view.bottom - view.top) / (view.bounds[3] - view.bounds[2]);
                assert!((scale_x - scale_y).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn geometric_gaps_poles_and_two_dimensional_clipping() {
        for source in [
            "param(t,1/(t-0.0037),-2,2)",
            "param(1/(t-0.0037),t,-2,2)",
            "param(t,sqrt(t^2-1),-2,2)",
        ] {
            let plot = Plot::parse(source).unwrap();
            let canvas = plot.render([-2.0, 2.0, -2.0, 2.0], (240, 224));
            let pixels = canvas
                .pixels()
                .chunks_exact(240)
                .map(|r| r.to_vec())
                .collect();
            assert_eq!(components(mask(pixels)), 2, "{source}");
        }
        let size = (240, 224);
        let view = View::new([-1.0, 1.0, -1.0, 1.0], size);
        for (a, b) in [
            ((-100.0, 100.0), (400.0, 100.0)),
            ((100.0, -100.0), (100.0, 400.0)),
            ((-100.0, -100.0), (400.0, 400.0)),
        ] {
            let mut stroke = Stroke::new(size);
            clipped_line(&mut stroke, a, b, &view);
            assert!(stroke.coverage.iter().any(|v| *v > 0.0));
            for (i, v) in stroke.coverage.iter().enumerate() {
                if *v > 0.0 {
                    let (x, y) = ((i % 240) as f64, (i / 240) as f64);
                    assert!(x >= view.left && x <= view.right && y >= view.top && y <= view.bottom);
                }
            }
        }
    }

    #[test]
    fn mixed_sampling_domain_survives_survey_aspect_and_cache() {
        let mut plot = Plot::parse("x; polar(10,0,2*pi)").unwrap();
        let samples = std::rc::Rc::new(RefCell::new(Vec::new()));
        let recorded = samples.clone();
        plot.functions[0] = Box::new(move |x| {
            recorded.borrow_mut().push(x);
            x
        });
        let bounds = plot.auto_bounds([2.0, 4.0]);
        samples.borrow_mut().clear();
        let first = plot.image(bounds, (560, 224));
        assert!(!samples.borrow().is_empty());
        assert!(samples.borrow().iter().all(|x| (2.0..=4.0).contains(x)));
        let count = samples.borrow().len();
        assert!(Arc::ptr_eq(&first, &plot.image(bounds, (560, 224))));
        assert_eq!(samples.borrow().len(), count);
        let resized = plot.image(bounds, (100, 300));
        assert!(!Arc::ptr_eq(&first, &resized));
        assert!(samples.borrow().iter().all(|x| (2.0..=4.0).contains(x)));
        // Same survey bounds, different domain: the cache must not reuse the old image.
        let next_bounds = plot.auto_bounds([3.0, 5.0]);
        assert_eq!(bounds, next_bounds);
        samples.borrow_mut().clear();
        assert!(!Arc::ptr_eq(&resized, &plot.image(next_bounds, (100, 300))));
        assert!(samples.borrow().iter().all(|x| (3.0..=5.0).contains(x)));
        samples.borrow_mut().clear();
        plot.image([-1.0, 1.0, -20.0, 20.0], (560, 224));
        assert!(samples.borrow().iter().all(|x| (-1.0..=1.0).contains(x)));
    }

    #[test]
    fn parametric_work_is_bounded_and_ranges_are_respected() {
        let calls = std::cell::Cell::new(0);
        let view = View::new([-1.0, 1.0, -1.0, 1.0], (70, 28));
        let mut stroke = Stroke::new((70, 28));
        draw_parametric(
            &mut stroke,
            &|t| {
                assert!((2.0..=4.0).contains(&t));
                calls.set(calls.get() + 1);
                (t, (t * 1e9).sin() * 1e6)
            },
            [2.0, 4.0],
            &view,
        );
        assert!(calls.get() <= 1 + 69 * 129);
    }

    #[test]
    fn parses_and_binds_only_x() {
        for source in [
            "",
            " ",
            "x;",
            ";x",
            "x;;x",
            "x+y",
            "y",
            "sin(",
            "unknown(x)",
            "x;x;x;x;x;x;x",
        ] {
            assert!(Plot::parse(source).is_err(), "{source:?}");
        }
        let plot = Plot::parse(" sin(x); 1/x; sqrt(x); x^2; pi; 2 ").unwrap();
        assert_eq!(plot.functions.len(), 6);
        assert_eq!((plot.functions[3])(3.0), 9.0);
    }
}
