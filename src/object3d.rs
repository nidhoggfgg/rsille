use crate::Canvas;

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
    pub fn rotate_xyz(&mut self, anlge_x: f64, anlge_y: f64, angle_z: f64) {
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
    vertices: Vec<Point3D>,
    sides: Vec<(usize, usize)>,
    canvas: Canvas,
}

impl Object3D {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            sides: Vec::new(),
            canvas: Canvas::new(),
        }
    }

    pub fn from(points: &[(f64, f64, f64)]) -> Self {
        let mut vertices = Vec::new();
        for p in points {
            let point = Point3D::new(p.0, p.1, p.2);
            vertices.push(point);
        }
        Self {
            vertices,
            sides: Vec::new(),
            canvas: Canvas::new(),
        }
    }

    pub fn draw(&mut self, move_x: f64, move_z: f64) -> String {
        self.canvas.clear();
        self.draw_impl(move_x, move_z);
        self.canvas.draw()
    }

    pub fn lines(&mut self, move_x: f64, move_z: f64) -> Vec<String> {
        self.canvas.clear();
        self.draw_impl(move_x, move_z);
        self.canvas.lines()
    }

    pub fn sides(&self) -> Vec<(f64, f64, f64)> {
        self.vertices.iter().map(|p| p.get()).collect()
    }

    pub fn add_points(&mut self, points: &[(f64, f64, f64)]) {
        for p in points {
            self.vertices.push(Point3D::from(*p));
        }
    }

    pub fn add_sides(&mut self, sides: &[(usize, usize)]) {
        let vn = self.vertices.len();
        for side in sides {
            if vn <= side.0 || vn <= side.1 {
                panic!()
            }

            self.sides.push(*side);
        }
    }

    pub fn rotate_xyz(&mut self, angle_x: f64, angle_y: f64, angle_z: f64) {
        for p in &mut self.vertices {
            p.rotate_xyz(angle_x, angle_y, angle_z);
        }
    }

    fn draw_impl(&mut self, move_x: f64, move_z: f64) {
        for s in &self.sides {
            let (v1, v2) = (self.vertices[s.0], self.vertices[s.1]);
            let (x1, y1) = (move_x + v1.x, move_z + v1.z);
            let (x2, y2) = (move_x + v2.x, move_z + v2.z);
            self.canvas.line(x1, y1, x2, y2);
        }

        // it's .... bad, add argument move_x, move_z works much better
        // if self.vertices.is_empty() {
        //     return;
        // }
        // let px = self
        //     .vertices
        //     .iter()
        //     .min_by_key(|&p| p.x.partial_cmp(&p.x).unwrap_or(std::cmp::Ordering::Equal))
        //     .unwrap()
        //     .x
        //     .abs()
        //     .round() + 1.0;
        // let py = self
        //     .vertices
        //     .iter()
        //     .min_by_key(|&p| p.y.partial_cmp(&p.y).unwrap_or(std::cmp::Ordering::Equal))
        //     .unwrap()
        //     .y
        //     .abs()
        //     .round() + 1.0;
        // let pz = self
        //     .vertices
        //     .iter()
        //     .min_by_key(|&p| p.z.partial_cmp(&p.z).unwrap_or(std::cmp::Ordering::Equal))
        //     .unwrap()
        //     .z
        //     .abs()
        //     .round() + 1.0;
        // for s in &self.sides {
        //     let (v1, v2) = (self.vertices[s.0], self.vertices[s.1]);
        //     self.canvas.line(px*2.0 + v1.x, pz * 2.0 + v1.z, px * 2.0 + v2.x, pz * 2.0 + v2.z);
        // }
    }
}
