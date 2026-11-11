//! Bounded sampling and an opaque, two-sided orthographic software renderer.
use super::{
    bind, equal_bounds, interpolate, RasterCanvas, Rgb, View, AXIS, BACKGROUND, COLORS, GRID,
};

type Point = [f64; 3];
const GRID_STEPS: usize = 64;
const CURVE_STEPS: usize = 2048;

pub(super) enum Object {
    Surface {
        sample: Box<dyn Fn(f64, f64) -> f64>,
        x: [f64; 2],
        y: [f64; 2],
    },
    Curve {
        sample: Box<dyn Fn(f64) -> Point>,
        range: [f64; 2],
    },
}

pub(super) fn parse(source: &str) -> Result<Option<Object>, String> {
    let name = source
        .split(|c: char| !c.is_ascii_alphanumeric())
        .next()
        .unwrap_or("");
    if name != "surface" && name != "param3" {
        return Ok(None);
    }
    let body = source[name.len()..]
        .trim()
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| format!("Expected {name}(... )"))?;
    let (mut depth, mut start) = (0usize, 0);
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
    if args.len() != 5 || args.iter().any(|a| a.is_empty()) {
        return Err(format!(
            "Expected {}",
            if name == "surface" {
                "surface(z_expr, xmin, xmax, ymin, ymax)"
            } else {
                "param3(x_expr, y_expr, z_expr, tmin, tmax)"
            }
        ));
    }
    let range = |i: usize| -> Result<[f64; 2], String> {
        let a =
            meval::eval_str(args[i]).map_err(|e| format!("Invalid constant range bound: {e}"))?;
        let b = meval::eval_str(args[i + 1])
            .map_err(|e| format!("Invalid constant range bound: {e}"))?;
        if !a.is_finite() || !b.is_finite() || a >= b || !(b - a).is_finite() {
            return Err("Range bounds must be finite, increasing, with a finite span".into());
        }
        Ok([a, b])
    };
    Ok(Some(if name == "surface" {
        let x = range(1)?;
        let y = range(3)?;
        let expression = args[0].parse::<meval::Expr>().map_err(|e| e.to_string())?;
        let sample = Box::new(
            expression
                .bind2("x", "y")
                .map_err(|e| format!("z(x,y): {e}"))?,
        );
        Object::Surface { sample, x, y }
    } else {
        let range = range(3)?;
        let x = bind(args[0], "t").map_err(|e| format!("x(t): {e}"))?;
        let y = bind(args[1], "t").map_err(|e| format!("y(t): {e}"))?;
        let z = bind(args[2], "t").map_err(|e| format!("z(t): {e}"))?;
        Object::Curve {
            sample: Box::new(move |t| [x(t), y(t), z(t)]),
            range,
        }
    }))
}

struct Mesh {
    points: Vec<Point>,
    surface: bool,
    color: Rgb,
}
pub(super) struct Scene {
    meshes: Vec<Mesh>,
    low: Point,
    high: Point,
}
fn finite(p: Point) -> bool {
    p.iter().all(|v| v.is_finite())
}
fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: Point, b: Point) -> Point {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
// Orthonormal camera basis: z is up, looking from positive x/y/z.
fn project(p: Point) -> Point {
    let r = 2.0f64.sqrt();
    let s = 6.0f64.sqrt();
    [
        (p[0] - p[1]) / r,
        (-p[0] - p[1] + 2.0 * p[2]) / s,
        (p[0] + p[1] + p[2]) / 3.0f64.sqrt(),
    ]
}

fn rotate(p: Point, angle: f64) -> Point {
    let (s, c) = angle.sin_cos();
    [c * p[0] - s * p[1], s * p[0] + c * p[1], p[2]]
}

impl Scene {
    pub(super) fn new(objects: Vec<Object>) -> Self {
        let mut meshes = Vec::new();
        for (object, color) in objects.into_iter().zip(COLORS) {
            let mut points = Vec::new();
            let surface = matches!(object, Object::Surface { .. });
            match object {
                Object::Surface { sample, x, y } => {
                    for j in 0..=GRID_STEPS {
                        let y = interpolate(y[0], y[1], j as f64 / GRID_STEPS as f64);
                        for i in 0..=GRID_STEPS {
                            let x = interpolate(x[0], x[1], i as f64 / GRID_STEPS as f64);
                            points.push([x, y, sample(x, y)]);
                        }
                    }
                }
                Object::Curve { sample, range } => {
                    for i in 0..=CURVE_STEPS {
                        points.push(sample(interpolate(
                            range[0],
                            range[1],
                            i as f64 / CURVE_STEPS as f64,
                        )));
                    }
                }
            }
            meshes.push(Mesh {
                points,
                surface,
                color,
            });
        }
        // One common normalization prevents overflow without distorting world units.
        let magnitude = meshes
            .iter()
            .flat_map(|m| &m.points)
            .filter(|p| finite(**p))
            .flat_map(|p| p.iter())
            .fold(0.0f64, |a, v| a.max(v.abs()));
        let magnitude = if magnitude > 0.0 { magnitude } else { 1.0 };
        let mut low = [f64::INFINITY; 3];
        let mut high = [f64::NEG_INFINITY; 3];
        for mesh in &mut meshes {
            for p in &mut mesh.points {
                if !finite(*p) {
                    *p = [f64::NAN; 3];
                    continue;
                }
                for k in 0..3 {
                    p[k] /= magnitude;
                    low[k] = low[k].min(p[k]);
                    high[k] = high[k].max(p[k]);
                }
            }
        }
        if !finite(low) {
            low = [-1.0; 3];
            high = [1.0; 3];
        }
        let center = std::array::from_fn::<_, 3, _>(|k| (low[k] + high[k]) * 0.5);
        let span = (0..3).map(|k| high[k] - low[k]).fold(0.0f64, f64::max);
        let span = if span > 0.0 { span } else { 1.0 };
        for mesh in &mut meshes {
            for p in &mut mesh.points {
                for k in 0..3 {
                    p[k] = (p[k] - center[k]) / span;
                }
            }
        }
        for k in 0..3 {
            low[k] = (low[k] - center[k]) / span;
            high[k] = (high[k] - center[k]) / span;
        }
        Self { meshes, low, high }
    }

    fn corners(&self) -> [Point; 8] {
        std::array::from_fn(|i| {
            std::array::from_fn(|k| {
                if i & (1 << k) == 0 {
                    self.low[k]
                } else {
                    self.high[k]
                }
            })
        })
    }

    pub(super) fn bounds(&self) -> [f64; 4] {
        // Envelope of the entire z-axis orbit, not just the current view.
        let radius = self.low[0]
            .abs()
            .max(self.high[0].abs())
            .hypot(self.low[1].abs().max(self.high[1].abs()));
        let vertical = (2.0 / 3.0f64).sqrt();
        let tilt = radius / 3.0f64.sqrt();
        let corners = [
            [-radius, vertical * self.low[2] - tilt],
            [radius, vertical * self.high[2] + tilt],
        ];
        let mut bounds = [0.0; 4];
        for k in 0..2 {
            let lo = corners.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min);
            let hi = corners
                .iter()
                .map(|p| p[k])
                .fold(f64::NEG_INFINITY, f64::max);
            let pad = ((hi - lo) * 0.08).max(0.05);
            bounds[2 * k] = lo - pad;
            bounds[2 * k + 1] = hi + pad;
        }
        bounds
    }

    pub(super) fn render(&self, bounds: [f64; 4], size: (u32, u32)) -> RasterCanvas {
        self.render_at(bounds, size, 0.0)
    }

    pub(super) fn render_at(&self, bounds: [f64; 4], size: (u32, u32), angle: f64) -> RasterCanvas {
        let view = View::new(equal_bounds(bounds, size), size);
        let screen = |p| {
            let p = project(rotate(p, angle));
            [
                view.left
                    + super::fraction(p[0], view.bounds[0], view.bounds[1])
                        * (view.right - view.left),
                view.y(p[1]),
                p[2],
            ]
        };
        let mut target = Target::new(size);
        let corners = self.corners().map(screen);
        for i in 0..8 {
            for k in 0..3 {
                if i & (1 << k) == 0 {
                    target.line(corners[i], corners[i | (1 << k)], GRID);
                }
            }
        }
        // Three brighter edges indicate the positive world-axis directions.
        for k in 0..3 {
            target.line(corners[0], corners[1 << k], AXIS);
        }
        for mesh in &self.meshes {
            let points: Vec<_> = mesh.points.iter().copied().map(screen).collect();
            if mesh.surface {
                for j in 0..GRID_STEPS {
                    for i in 0..GRID_STEPS {
                        let a = j * (GRID_STEPS + 1) + i;
                        let b = a + 1;
                        let c = a + GRID_STEPS + 1;
                        for indices in [[a, b, c], [b, c + 1, c]] {
                            let world = indices.map(|n| mesh.points[n]);
                            if !world.iter().all(|p| finite(*p)) {
                                continue;
                            }
                            let normal = cross(sub(world[1], world[0]), sub(world[2], world[0]));
                            let length = dot(normal, normal).sqrt();
                            if length == 0.0 {
                                continue;
                            }
                            let light =
                                (dot(normal, [0.3, -0.4, 0.8660254037844386]) / length).abs();
                            let shade = 0.35 + 0.65 * light;
                            let Rgb(r, g, b) = mesh.color;
                            target.triangle(
                                indices.map(|n| points[n]),
                                Rgb(
                                    (r as f64 * shade) as u8,
                                    (g as f64 * shade) as u8,
                                    (b as f64 * shade) as u8,
                                ),
                            );
                        }
                    }
                }
            } else {
                for pair in points.windows(2) {
                    target.line(pair[0], pair[1], mesh.color);
                }
            }
        }
        target.finish()
    }
}

struct Target {
    canvas: RasterCanvas,
    depth: Vec<f64>,
    strokes: Vec<Vec<(f64, f64, Rgb)>>,
    width: usize,
    height: usize,
}
impl Target {
    fn new(size: (u32, u32)) -> Self {
        let canvas =
            RasterCanvas::new(size.0, size.1, BACKGROUND).expect("graph allocation failed");
        let depth = vec![f64::NEG_INFINITY; canvas.pixels().len()];
        Self {
            strokes: vec![Vec::new(); depth.len()],
            canvas,
            depth,
            width: size.0 as usize,
            height: size.1 as usize,
        }
    }
    fn pixel(&mut self, x: i32, y: i32, z: f64, color: Rgb) {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height || !z.is_finite()
        {
            return;
        }
        let i = y as usize * self.width + x as usize;
        if z >= self.depth[i] {
            self.depth[i] = z;
            self.canvas.set_pixel(x, y, color);
        }
    }
    fn triangle(&mut self, p: [Point; 3], color: Rgb) {
        if !p.iter().all(|p| finite(*p)) {
            return;
        }
        let edge = |a: Point, b: Point, x: f64, y: f64| {
            (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])
        };
        let area = edge(p[0], p[1], p[2][0], p[2][1]);
        if !area.is_finite() || area.abs() < 1e-12 {
            return;
        }
        let min = |k| {
            p.iter()
                .map(|p| p[k])
                .fold(f64::INFINITY, f64::min)
                .floor()
                .max(0.0) as usize
        };
        let max = |k, limit| {
            p.iter()
                .map(|p| p[k])
                .fold(f64::NEG_INFINITY, f64::max)
                .ceil()
                .max(0.0)
                .min(limit as f64) as usize
        };
        for y in min(1)..max(1, self.height) {
            for x in min(0)..max(0, self.width) {
                let a = edge(p[1], p[2], x as f64 + 0.5, y as f64 + 0.5) / area;
                let b = edge(p[2], p[0], x as f64 + 0.5, y as f64 + 0.5) / area;
                let c = 1.0 - a - b;
                if a >= -1e-10 && b >= -1e-10 && c >= -1e-10 {
                    self.pixel(
                        x as i32,
                        y as i32,
                        a * p[0][2] + b * p[1][2] + c * p[2][2],
                        color,
                    );
                }
            }
        }
    }
    fn line(&mut self, a: Point, b: Point, color: Rgb) {
        if !finite(a) || !finite(b) || self.width == 0 || self.height == 0 {
            return;
        }
        let d = sub(b, a);
        if !finite(d) {
            return;
        }
        let (mut lo, mut hi) = (0.0f64, 1.0f64);
        for (k, max) in [(0, self.width), (1, self.height)] {
            if d[k] == 0.0 {
                if a[k] < -1.25 || a[k] > max as f64 + 1.25 {
                    return;
                }
            } else {
                let u = (-1.25 - a[k]) / d[k];
                let v = (max as f64 + 1.25 - a[k]) / d[k];
                lo = lo.max(u.min(v));
                hi = hi.min(u.max(v));
            }
        }
        if lo > hi {
            return;
        }
        let start: Point = std::array::from_fn(|k| a[k] + d[k] * lo);
        let end: Point = std::array::from_fn(|k| a[k] + d[k] * hi);
        let min = |k: usize| (start[k].min(end[k]) - 1.25).floor().max(0.0) as usize;
        let max = |k: usize, limit: usize| {
            (start[k].max(end[k]) + 1.25)
                .ceil()
                .max(0.0)
                .min(limit as f64) as usize
        };
        let length = d[0].hypot(d[1]);
        for y in min(1)..max(1, self.height) {
            for x in min(0)..max(0, self.width) {
                let t = if length > 0.0 {
                    (((x as f64 + 0.5 - a[0]) * (d[0] / length)
                        + (y as f64 + 0.5 - a[1]) * (d[1] / length))
                        / length)
                        .clamp(lo, hi)
                } else {
                    hi
                };
                let distance =
                    (x as f64 + 0.5 - a[0] - d[0] * t).hypot(y as f64 + 0.5 - a[1] - d[1] * t);
                // Radius .75 plus a one-pixel antialiasing ramp, with round caps.
                let coverage = (1.25 - distance).clamp(0.0, 1.0);
                if coverage > 0.0 {
                    self.strokes[y * self.width + x].push((a[2] + d[2] * t, coverage, color));
                }
            }
        }
    }

    fn finish(mut self) -> RasterCanvas {
        for (i, fragments) in self.strokes.iter().enumerate() {
            // Resolve against the completed surface depth buffer, independent of
            // submission order. Union each curve's coverage before blending joins.
            let mut layers: Vec<(f64, f64, Rgb)> = Vec::new();
            for &(z, coverage, color) in fragments {
                if !z.is_finite() || z < self.depth[i] {
                    continue;
                }
                if let Some(layer) = layers.iter_mut().find(|layer| layer.2 == color) {
                    layer.0 = layer.0.max(z);
                    layer.1 = layer.1.max(coverage);
                } else {
                    layers.push((z, coverage, color));
                }
            }
            layers.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut pixel = self.canvas.pixels()[i];
            for (_, coverage, Rgb(r, g, b)) in layers {
                let blend =
                    |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * coverage).round() as u8;
                pixel = Rgb(blend(pixel.0, r), blend(pixel.1, g), blend(pixel.2, b));
            }
            self.canvas
                .set_pixel((i % self.width) as i32, (i / self.width) as i32, pixel);
        }
        self.canvas
    }
}

#[cfg(test)]
mod tests {
    use super::super::Plot;
    use super::*;
    use std::sync::Arc;

    #[test]
    fn syntax_ranges_and_dimensions() {
        for source in [
            "surface(max(x,y),-pi,pi,-2,2)",
            "param3(cos(t),sin(t),t,0,2*pi)",
            "surface(0,0,1,0,1);param3(t,t,t,0,1)",
        ] {
            assert!(Plot::parse(source).is_ok(), "{source}");
        }
        for source in [
            "surface(t,0,1,0,1)",
            "param3(x,t,t,0,1)",
            "surface(x,0,0,0,1)",
            "surface(x,0,1,2,1)",
            "param3(t,t,t,0,1/0)",
            "surface(x,x,1,0,1)",
            "surface(x,0,1,0)",
            "param3(t,,t,0,1)",
            "surface(max(x,y),0,1,0,1))",
        ] {
            assert!(Plot::parse(source).is_err(), "{source}");
        }
        for source in ["x;surface(x+y,0,1,0,1)", "param3(t,t,t,0,1);polar(1,0,pi)"] {
            assert!(Plot::parse(source).err().unwrap().contains("Cannot mix 2D"));
        }
        let six = ["surface(0,0,1,0,1)"; 6].join(";");
        assert!(Plot::parse(&six).is_ok());
        assert!(Plot::parse(&(six + ";surface(0,0,1,0,1)")).is_err());
    }

    #[test]
    fn camera_preserves_lengths_and_sampling_is_bounded() {
        let p = [2.0, -3.0, 4.0];
        assert!((dot(project(p), project(p)) - dot(p, p)).abs() < 1e-12);
        let calls = std::rc::Rc::new(std::cell::Cell::new(0));
        let count = calls.clone();
        let scene = Scene::new(vec![Object::Surface {
            sample: Box::new(move |x, y| {
                assert!((2.0..=4.0).contains(&x) && (-3.0..=1.0).contains(&y));
                count.set(count.get() + 1);
                x + y
            }),
            x: [2.0, 4.0],
            y: [-3.0, 1.0],
        }]);
        assert_eq!(calls.get(), (GRID_STEPS + 1).pow(2));
        assert_eq!(scene.meshes[0].points.len(), calls.get());
        scene.render(scene.bounds(), (80, 60));
        assert_eq!(calls.get(), (GRID_STEPS + 1).pow(2));
    }

    #[test]
    fn depth_interpolates_triangles_and_tests_curves_in_any_order() {
        let triangle = [[2.0, 2.0, 0.0], [18.0, 2.0, 0.0], [2.0, 18.0, 0.0]];
        for reverse in [false, true] {
            let mut target = Target::new((20, 20));
            if reverse {
                target.line([3.0, 5.0, -1.0], [12.0, 5.0, -1.0], COLORS[1]);
            }
            target.triangle(triangle, COLORS[0]);
            target.line([3.0, 5.0, -1.0], [12.0, 5.0, -1.0], COLORS[1]);
            assert_eq!(target.canvas.pixels()[5 * 20 + 5], COLORS[0]);
            target.line([3.0, 5.5, 1.0], [12.0, 5.5, 1.0], COLORS[2]);
            target.triangle(triangle, COLORS[0]);
            assert_eq!(target.finish().pixels()[5 * 20 + 5], COLORS[2]);
        }
        let mut target = Target::new((20, 20));
        target.triangle(
            [[2.0, 2.0, 0.0], [18.0, 2.0, 16.0], [2.0, 18.0, 0.0]],
            COLORS[0],
        );
        assert!((target.depth[5 * 20 + 5] - 3.5).abs() < 1e-10);
    }

    #[test]
    fn crossing_surfaces_use_per_pixel_depth_not_draw_order() {
        let a = [[2.0, 2.0, 0.0], [18.0, 2.0, 16.0], [2.0, 18.0, 0.0]];
        let b = [[2.0, 2.0, 8.0], [18.0, 2.0, 8.0], [2.0, 18.0, 8.0]];
        let mut first = Target::new((20, 20));
        first.triangle(a, COLORS[0]);
        first.triangle(b, COLORS[1]);
        let mut second = Target::new((20, 20));
        second.triangle(b, COLORS[1]);
        second.triangle(a, COLORS[0]);
        assert_eq!(first.canvas.pixels(), second.canvas.pixels());
        assert_eq!(first.canvas.pixels()[3 * 20 + 4], COLORS[1]);
        assert_eq!(first.canvas.pixels()[3 * 20 + 12], COLORS[0]);
    }

    #[test]
    fn curves_are_sampled_once_and_nonfinite_vertices_leave_gaps() {
        let calls = std::rc::Rc::new(std::cell::Cell::new(0));
        let count = calls.clone();
        let scene = Scene::new(vec![Object::Curve {
            sample: Box::new(move |t| {
                assert!((-2.0..=3.0).contains(&t));
                count.set(count.get() + 1);
                [t, t * 2.0, 0.0]
            }),
            range: [-2.0, 3.0],
        }]);
        assert_eq!(calls.get(), CURVE_STEPS + 1);
        let points = &scene.meshes[0].points;
        let delta = sub(points[CURVE_STEPS], points[0]);
        assert!((delta[1] / delta[0] - 2.0).abs() < 1e-12);
        scene.render(scene.bounds(), (100, 80));
        scene.render(scene.bounds(), (50, 40));
        assert_eq!(calls.get(), CURVE_STEPS + 1);

        let mut target = Target::new((20, 20));
        target.triangle(
            [[2.0, 2.0, 0.0], [18.0, 2.0, f64::NAN], [2.0, 18.0, 0.0]],
            COLORS[0],
        );
        target.line([2.0, 2.0, 0.0], [18.0, 18.0, f64::INFINITY], COLORS[1]);
        assert!(target.canvas.pixels().iter().all(|p| *p == BACKGROUND));
        target.line([-10000.0, 10.5, 0.0], [10000.0, 10.5, 0.0], COLORS[2]);
        let canvas = target.finish();
        assert!((0..20).all(|x| canvas.pixels()[10 * 20 + x] == COLORS[2]));
    }

    #[test]
    fn round_strokes_union_joins_and_preserve_gaps_and_occlusion() {
        let draw = |split: bool| {
            let mut target = Target::new((32, 24));
            let a = [3.25, 4.75, 1.0];
            let b = [27.25, 18.75, 1.0];
            if split {
                for i in 0..100 {
                    let p = |t: f64| std::array::from_fn(|k| interpolate(a[k], b[k], t));
                    target.line(p(i as f64 / 100.0), p((i + 1) as f64 / 100.0), COLORS[0]);
                }
            } else {
                target.line(a, b, COLORS[0]);
            }
            target.finish()
        };
        let whole = draw(false);
        assert_eq!(whole.pixels(), draw(true).pixels());
        assert!(whole
            .pixels()
            .iter()
            .any(|p| *p != BACKGROUND && *p != COLORS[0]));
        let mut target = Target::new((32, 24));
        for pair in [[2.0, 10.0], [20.0, 29.0]] {
            target.line([pair[0], 12.5, -1.0], [pair[1], 12.5, -1.0], COLORS[0]);
        }
        target.line([10.0, 12.5, -1.0], [f64::NAN; 3], COLORS[0]);
        target.triangle(
            [[1.0, 1.0, 0.0], [15.0, 1.0, 0.0], [1.0, 23.0, 0.0]],
            COLORS[1],
        );
        let canvas = target.finish();
        assert_eq!(canvas.pixels()[12 * 32 + 3], COLORS[1]);
        assert_eq!(canvas.pixels()[12 * 32 + 15], BACKGROUND);
        assert_eq!(canvas.pixels()[12 * 32 + 24], COLORS[0]);
    }

    #[test]
    fn rotation_cache_and_framing_are_stable() {
        let plot = Plot::parse("param3(3*cos(t)+8,sin(t),t,0,4)").unwrap();
        assert!(plot.is_spatial());
        let bounds = plot.auto_bounds([0.0, 1.0]);
        let size = (120, 90);
        let zero = plot.image(bounds, size);
        assert!(Arc::ptr_eq(&zero, &plot.image_at(bounds, size, 0.0)));
        let turned = plot.image_at(bounds, size, 0.7);
        assert_ne!(zero.pixels(), turned.pixels());
        assert!(Arc::ptr_eq(&turned, &plot.image_at(bounds, size, 0.7)));
        assert_eq!(zero.pixels(), plot.image(bounds, size).pixels());
        let scene = plot.spatial.as_ref().unwrap();
        for step in 0..360 {
            let angle = (step as f64).to_radians();
            for corner in scene.corners() {
                let rotated = rotate(corner, angle);
                assert_eq!(rotated[2], corner[2]);
                assert!((dot(rotated, rotated) - dot(corner, corner)).abs() < 1e-12);
                let p = project(rotated);
                assert!(p[0] > bounds[0] && p[0] < bounds[1]);
                assert!(p[1] > bounds[2] && p[1] < bounds[3]);
            }
        }
        let flat = Plot::parse("sin(x)").unwrap();
        assert!(!flat.is_spatial());
        let a = flat.image(bounds, size);
        assert!(Arc::ptr_eq(&a, &flat.image_at(bounds, size, 1.0)));
    }

    #[test]
    fn render_cache_resize_gaps_and_extreme_values() {
        let plot =
            Plot::parse("surface(sin(x)*cos(y),-pi,pi,-pi,pi);param3(cos(t),sin(t),t/4,0,2*pi)")
                .unwrap();
        let bounds = plot.auto_bounds([-10.0, 10.0]);
        assert_eq!(bounds, plot.auto_bounds([0.0, 1.0]));
        let a = plot.image(bounds, (180, 120));
        assert!(Arc::ptr_eq(&a, &plot.image(bounds, (180, 120))));
        assert!(
            a.pixels()
                .iter()
                .filter(|p| **p != BACKGROUND && **p != GRID && **p != AXIS)
                .count()
                > 200
        );
        let b = plot.image(bounds, (90, 60));
        assert!(!Arc::ptr_eq(&a, &b));
        let mut changed = bounds;
        changed[1] += 0.2;
        let c = plot.image(changed, (90, 60));
        assert!(!Arc::ptr_eq(&b, &c));
        assert_ne!(b.pixels(), c.pixels());
        for source in [
            "surface(sqrt(-1),0,1,0,1)",
            "surface(1e308,0,1,0,1)",
            "param3(t,1/t,t,-1,1)",
        ] {
            let plot = Plot::parse(source).unwrap();
            let bounds = plot.auto_bounds([0.0, 1.0]);
            assert!(bounds.iter().all(|v| v.is_finite()));
            plot.image(bounds, (32, 24));
        }
        let scene = Scene::new(vec![parse("surface(sqrt(-1),0,1,0,1)").unwrap().unwrap()]);
        assert!(scene.meshes[0].points.iter().all(|p| !finite(*p)));
    }
}
