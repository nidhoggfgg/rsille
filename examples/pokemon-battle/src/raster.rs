//! Depth-tested, perspective CPU rasterizer. All models are solid geometry.
use crate::math::V3;

pub type Rgb = [f32; 3];

#[derive(Clone, Copy)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: -0.15,
            pitch: 0.22,
            distance: 8.6,
        }
    }
}

#[derive(Default)]
pub struct Raster {
    pub width: usize,
    pub height: usize,
    view_height: usize,
    pub pixels: Vec<Rgb>,
    depth: Vec<f32>,
    eye: V3,
    right: V3,
    up: V3,
    forward: V3,
    focal: f32,
}

#[derive(Clone, Copy)]
struct Screen {
    x: f32,
    y: f32,
    z: f32,
}

impl Raster {
    pub fn begin(&mut self, width: usize, height: usize, view_height: usize, camera: Camera) {
        self.width = width;
        self.height = height;
        self.view_height = view_height.max(1);
        self.pixels.resize(width * height, [0.0; 3]);
        self.depth.resize(width * height, f32::INFINITY);
        self.depth.fill(f32::INFINITY);
        let target = V3(0.0, 1.13, 0.0);
        self.eye = target
            + V3(
                camera.yaw.sin() * camera.pitch.cos(),
                camera.pitch.sin(),
                camera.yaw.cos() * camera.pitch.cos(),
            ) * camera.distance;
        self.forward = (target - self.eye).unit();
        self.right = self.forward.cross(V3::Y).unit();
        self.up = self.right.cross(self.forward).unit();
        self.focal = (self.view_height as f32 * 1.48).min(width as f32 * 1.03);
        for y in 0..height {
            for x in 0..width {
                let u = (x as f32 + 0.5 - width as f32 * 0.5) / self.focal;
                let v = (self.view_height as f32 * 0.46 - y as f32 - 0.5) / self.focal;
                let ray = self.forward + self.right * u + self.up * v;
                let index = y * width + x;
                if ray.1 < -0.0001 {
                    // This parameter is camera-space Z, matching perspective triangle depth.
                    let depth = -self.eye.1 / ray.1;
                    let p = self.eye + ray * depth;
                    self.pixels[index] =
                        crate::environment::fog(crate::environment::grass_color(p.0, p.2), depth);
                    self.depth[index] = depth;
                } else {
                    self.pixels[index] = crate::environment::sky(ray);
                }
            }
        }
    }

    pub fn face(&mut self, p: [V3; 3], base: Rgb, emissive: bool) {
        let normal = (p[1] - p[0]).cross(p[2] - p[0]).unit();
        let center = (p[0] + p[1] + p[2]) / 3.0;
        let rgb = if emissive {
            base
        } else {
            // Two-sided lighting also covers the undersides of the thin tail and ears.
            let n = if normal.dot(self.eye - center) < 0.0 {
                -normal
            } else {
                normal
            };
            let light = V3(-0.5, 0.9, 0.8).unit();
            let diffuse = n.dot(light).max(0.0);
            let rim = n.dot(V3(0.7, 0.35, -0.8).unit()).max(0.0);
            let spec = n
                .dot((light + (self.eye - center).unit()).unit())
                .max(0.0)
                .powi(32)
                * 0.12;
            std::array::from_fn(|i| {
                (base[i] * (0.44 + diffuse * 0.65) + [0.03, 0.12, 0.21][i] * rim + spec).min(1.0)
            })
        };
        self.triangle(
            p,
            crate::environment::fog(rgb, (center - self.eye).length()),
            false,
        );
    }
    fn project(&self, p: V3) -> Option<Screen> {
        let v = p - self.eye;
        let z = v.dot(self.forward);
        if z < 0.1 {
            return None;
        }
        Some(Screen {
            x: self.width as f32 * 0.5 + v.dot(self.right) * self.focal / z,
            y: self.view_height as f32 * 0.46 - v.dot(self.up) * self.focal / z,
            z,
        })
    }

    pub fn terrain(&mut self, points: [V3; 3]) {
        self.triangle(points, [0.0; 3], true);
    }

    fn triangle(&mut self, points: [V3; 3], rgb: Rgb, terrain: bool) {
        let [Some(a), Some(b), Some(c)] = points.map(|p| self.project(p)) else {
            return;
        };
        let edge = |a: Screen, b: Screen, x: f32, y: f32| {
            (x - a.x) * (b.y - a.y) - (y - a.y) * (b.x - a.x)
        };
        let area = edge(a, b, c.x, c.y);
        if area.abs() < 1e-6 {
            return;
        }
        let minx = a.x.min(b.x).min(c.x).floor().max(0.0) as usize;
        let maxx = a.x.max(b.x).max(c.x).ceil().max(0.0).min(self.width as f32) as usize;
        let miny = a.y.min(b.y).min(c.y).floor().max(0.0) as usize;
        let maxy =
            a.y.max(b.y)
                .max(c.y)
                .ceil()
                .max(0.0)
                .min(self.height as f32) as usize;
        for y in miny..maxy {
            for x in minx..maxx {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let wa = edge(b, c, px, py) / area;
                let wb = edge(c, a, px, py) / area;
                let wc = 1.0 - wa - wb;
                if wa < -1e-5 || wb < -1e-5 || wc < -1e-5 {
                    continue;
                }
                let z = 1.0 / (wa / a.z + wb / b.z + wc / c.z);
                let i = y * self.width + x;
                if z < self.depth[i] {
                    self.depth[i] = z;
                    self.pixels[i] = if terrain {
                        let p = (points[0] * (wa / a.z)
                            + points[1] * (wb / b.z)
                            + points[2] * (wc / c.z))
                            * z;
                        crate::environment::fog(crate::environment::grass_color(p.0, p.2), z)
                    } else {
                        rgb
                    };
                }
            }
        }
    }

    pub fn line(&mut self, a: V3, b: V3, color: Rgb) {
        let (Some(a), Some(b)) = (self.project(a), self.project(b)) else {
            return;
        };
        let steps = (b.x - a.x).abs().max((b.y - a.y).abs()).ceil() as usize;
        // Bound work even if a future scene supplies an offscreen segment.
        if steps > 4 * (self.width + self.height) {
            return;
        }
        for i in 0..=steps {
            let t = i as f32 / steps.max(1) as f32;
            let x = (a.x + (b.x - a.x) * t).round() as i32;
            let y = (a.y + (b.y - a.y) * t).round() as i32;
            if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
                continue;
            }
            let z = 1.0 / ((1.0 - t) / a.z + t / b.z);
            let k = y as usize * self.width + x as usize;
            if z < self.depth[k] + 0.012 {
                self.pixels[k] = color;
                self.depth[k] = z;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearer_geometry_occludes_regardless_of_submission_order() {
        let mut raster = Raster::default();
        let camera = Camera {
            yaw: 0.0,
            pitch: 0.0,
            distance: 10.0,
        };
        let triangle = |z| [V3(-1.0, 0.0, z), V3(1.0, 0.0, z), V3(0.0, 2.0, z)];
        raster.begin(80, 60, 60, camera);
        raster.face(triangle(0.0), [1.0, 0.0, 0.0], true);
        raster.face(triangle(1.0), [0.0, 1.0, 0.0], true);
        let a = raster.pixels.clone();
        raster.begin(80, 60, 60, camera);
        raster.face(triangle(1.0), [0.0, 1.0, 0.0], true);
        raster.face(triangle(0.0), [1.0, 0.0, 0.0], true);
        assert_eq!(a, raster.pixels);
        assert_eq!(raster.pixels[27 * 80 + 40], [0.0, 1.0, 0.0]);
    }
    #[test]
    fn continuous_ground_occludes_buried_geometry_from_each_direction() {
        for yaw in [-2.0, 0.0, 2.0] {
            let mut raster = Raster::default();
            raster.begin(
                100,
                70,
                70,
                Camera {
                    yaw,
                    ..Camera::default()
                },
            );
            let before = raster.pixels.clone();
            raster.face(
                [
                    V3(-2.0, -1.0, -1.0),
                    V3(2.0, -1.0, -1.0),
                    V3(0.0, -1.0, 2.0),
                ],
                [1.0, 0.0, 1.0],
                true,
            );
            assert_eq!(
                raster.pixels, before,
                "ground depth must occlude submerged triangles"
            );
        }
    }
}
