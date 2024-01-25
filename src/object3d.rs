use crate::{canvas::Paint, utils::{check_zoom, mean, MIN_DIFFERENCE}, Canvas};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point3D {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn from(xyz: (f64, f64, f64)) -> Self {
        Self {
            x: xyz.0,
            y: xyz.1,
            z: xyz.2,
        }
    }

    pub fn get(&self) -> (f64, f64, f64) {
        (self.x, self.y, self.z)
    }

    // see https://en.wikipedia.org/wiki/Rotation_matrix for more information
    pub fn rotate(&mut self, anlge_x: f64, anlge_y: f64, angle_z: f64) {
        // let (x, y, z) = (self.x, self.y, self.z);
        // let (sx, cx) = anlge_x.to_radians().sin_cos();
        // let (sy, cy) = anlge_y.to_radians().sin_cos();
        // let (sz, cz) = angle_z.to_radians().sin_cos();
        // let (t1, t2, t3) = (
        //     x * cy + y * sx * sy + z * cx * sy,
        //     y * cx - z * sx,
        //     y * sx + z * cx,
        // );
        // self.x = cz * t1 - sz * t2;
        // self.y = sz * t1 + cz * t2;
        // self.z = cz * t3 - sy * x;

        self.rotate_x(anlge_x);
        self.rotate_y(anlge_y);
        self.rotate_z(angle_z);
    }

    pub fn rotate_new(&self, anlge_x: f64, anlge_y: f64, angle_z: f64) -> Self {
        let mut point = *self; // = self.clone()
        point.rotate(anlge_x, anlge_y, angle_z);
        point
    }

    pub fn zoom(&mut self, center: Self, factor: f64) {
        check_zoom(factor);
        self.x = (self.x - center.x) * factor;
        self.y = (self.y - center.y) * factor;
        self.z = (self.z - center.z) * factor;
    }

    pub fn zoom_new(&self, center: Self, factor: f64) -> Point3D {
        check_zoom(factor);
        let dx = (self.x - center.x) * factor;
        let dy = (self.y - center.y) * factor;
        let dz = (self.z - center.y) * factor;

        Self {
            x: dx,
            y: dy,
            z: dz,
        }
    }

    pub fn rotate_x(&mut self, angle: f64) {
        let (s, c) = angle.to_radians().sin_cos();
        let (y, z) = (self.y, self.z);
        self.y = y * c - z * s;
        self.z = y * s + z * c;
    }

    pub fn rotate_y(&mut self, angle: f64) {
        let (s, c) = angle.to_radians().sin_cos();
        let (x, z) = (self.x, self.z);
        self.x = x * c + z * s;
        self.z = -x * s + z * c;
    }

    pub fn rotate_z(&mut self, anlge: f64) {
        let (s, c) = anlge.to_radians().sin_cos();
        let (x, y) = (self.x, self.y);
        self.x = x * c - y * s;
        self.y = x * s + y * c;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Object3D {
    origin_vertices: Vec<Point3D>,
    zoomed_vertices: Option<Vec<Point3D>>,
    sides: Vec<(usize, usize)>,
    center: Point3D,
}

impl Object3D {
    pub fn new() -> Self {
        Self {
            origin_vertices: Vec::new(),
            zoomed_vertices: None,
            sides: Vec::new(),
            center: Point3D::new(0.0, 0.0, 0.0),
        }
    }

    // pub fn from(points: &[(f64, f64, f64)]) -> Self {
    //     let mut vertices = Vec::new();
    //     for p in points {
    //         let point = Point3D::new(p.0, p.1, p.2);
    //         vertices.push(point);
    //     }
    //     let mut obj = Self {
    //         origin_vertices: vertices,
    //         zoomed_vertices: None,
    //         sides: Vec::new(),
    //         center: Point3D::new(0.0, 0.0, 0.0),
    //     };
    //     obj.calc_center();
    //     obj
    // }

    pub fn vertices(&self) -> Vec<(f64, f64, f64)> {
        self.origin_vertices.iter().map(|p| p.get()).collect()
    }

    pub fn add_points(&mut self, points: &[(f64, f64, f64)]) {
        for p in points {
            self.origin_vertices.push(Point3D::from(*p));
        }
        self.calc_center();
    }

    pub fn sides(&self) -> Vec<(usize, usize)> {
        self.sides.clone()
    }

    pub fn add_sides(&mut self, sides: &[(usize, usize)]) {
        let vn = self.origin_vertices.len();
        for side in sides {
            if vn <= side.0 || vn <= side.1 {
                panic!("wrong add sides!");
            }

            self.sides.push(*side);
        }
    }

    pub fn rotate(&mut self, angle_x: f64, angle_y: f64, angle_z: f64) {
        for p in &mut self.origin_vertices {
            p.rotate(angle_x, angle_y, angle_z);
        }
    }

    pub fn rotate_new(&self, angle_x: f64, angle_y: f64, angle_z: f64) -> Self {
        let mut obj = self.clone();
        obj.rotate(angle_x, angle_y, angle_z);
        obj
    }

    pub fn zoom(&mut self, factor: f64) {
        check_zoom(factor);
        let mut vertices = self.origin_vertices.clone();
        vertices.iter_mut().for_each(|v| v.zoom(self.center, factor));
        self.zoomed_vertices = Some(vertices);
    }

    pub fn zoom_new(&self, factor: f64) -> Self {
        check_zoom(factor);
        let mut points: Vec<Point3D> = self.origin_vertices.clone();
        points.iter_mut().for_each(|v| v.zoom(self.center, factor));
        Self {
            origin_vertices: points,
            zoomed_vertices: None,
            sides: self.sides.clone(),
            center: self.center,
        }
    }

    fn calc_center(&mut self) {
        let (mut xs, mut ys, mut zs) = (Vec::new(), Vec::new(), Vec::new());
        for p in &self.origin_vertices {
            xs.push(p.x);
            ys.push(p.y);
            zs.push(p.z);
        }
        let (mut mx, mut my, mut mz) = (mean(&xs), mean(&ys), mean(&zs));

        // forbide the lost of f64
        if (mx - self.center.x) < MIN_DIFFERENCE {
            mx = self.center.x;
        }
        if (my - self.center.y) < MIN_DIFFERENCE {
            my = self.center.y;
        }
        if (mz - self.center.z) < MIN_DIFFERENCE {
            mz = self.center.z;
        }
        self.center = Point3D::new(mx, my, mz);
    }
}

impl Paint for Object3D {
    fn paint(&self, canvas: &mut Canvas, x: f64, y: f64) {
        let points = if let Some(p) = &self.zoomed_vertices {
            p
        } else {
            &self.origin_vertices
        };
        for s in &self.sides {
            let (v1, v2) = (points[s.0], points[s.1]);
            let (x1, y1) = (x + v1.x, y + v1.z);
            let (x2, y2) = (x + v2.x, y + v2.z);
            canvas.line(x1, y1, x2, y2);
        }
    }
}
