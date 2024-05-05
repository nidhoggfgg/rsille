use crossterm::style::Color;
use glam::Affine3A;

use crate::{
    canvas::Paint, utils::{mean_f32, RsilleErr, MIN_DIFFERENCE}, Canvas
};

use super::math::glm::{Pos3, Vec3};

/// A paintable Object in 3D
///
/// Make object and easy to do something like rotate, zoom and more
/// also, it support colorful output
///
/// ## Example
///
/// make a cube and rotate it endlessly
/// ```no_run
/// use rsille::{extra::Object3D, Animation};
/// let cube = Object3D::cube(30.0);
/// let mut anime = Animation::new();
/// anime.push(cube, |cube| {
///     cube.rotate((1.0, 2.0, 3.0));
///     false
/// }, (0, 0));
/// anime.run();
/// ```
/// also try to not paint at *(0,0)*, at other location like *(30, -30)*
#[derive(Debug, Clone)]
pub struct Object3D {
    pub vertices: Vec<Pos3>,
    pub sides: Vec<Side>,
    center: Pos3,
    radians: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Side {
    pub side: (usize, usize),
    pub color: Option<Color>
}

impl Object3D {
    /// Construct a new Object3D, with no vertices and sides
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            sides: Vec::new(),
            center: Pos3::ZERO,
            radians: false
        }
    }

    /// Add vertices to object
    pub fn add_points(&mut self, points: &[Pos3]) {
        for p in points {
            self.vertices.push(*p);
        }
        self.calc_center();
    }

    /// Add sides to object
    ///
    /// For example, there is 3 vertices in the object,
    /// you want to connect the first and the second, then it's `[0, 1]`.
    ///
    /// Return an error if the index is out of range, like only 3 vertices but you want to connect `[0, 4]`
    pub fn add_sides(&mut self, sides: &[(usize, usize)]) -> Result<(), RsilleErr> {
        let vn = self.vertices.len();
        for side in sides {
            if vn <= side.0 || vn <= side.1 {
                return Err(RsilleErr::new("Wrong sides!".to_string()));
            }

            self.sides.push(Side { side: *side, color: None });
        }
        Ok(())
    }

    /// Add sides and color of those sides
    ///
    /// Take a look at [`add_sides`](struct.Object3D.html#method.add_sides) for more information
    pub fn add_sides_colorful(
        &mut self,
        sides: &[((usize, usize), Color)],
    ) -> Result<(), RsilleErr> {
        let vn = self.vertices.len();
        for (side, color) in sides {
            if vn <= side.0 || vn <= side.1 {
                return Err(RsilleErr::new("Wrong sides!".to_string()));
            }
            self.sides.push(Side { side: *side, color: None });
        }
        Ok(())
    }

    /// Set the color of the side
    ///
    /// If there isn't the side, it will do nothing
    // pub fn set_side_colorful(&mut self, side: (usize, usize), color: Color) {
    //     if self.sides.contains_key(&side) {
    //         self.sides.insert(side, Some(color));
    //     }
    // }

    /// Return the vertices
    pub fn vertices(&self) -> &[Pos3] {
        &self.vertices
    }

    /// Return the sides
    // pub fn sides(&self) -> Vec<(usize, usize)> {
    //     self.sides.keys().cloned().collect()
    // }

    /// Rotate the whole object
    ///
    /// Normaly, the rotate won't grow the error of f64.
    /// If the error is growing, use [`rotate_new`](struct.Object3D.html#method.rotate_new)
    pub fn rotate(&mut self, angle: Vec3) {
        let angle =  if self.radians {
            angle
        } else {
            Vec3::new(angle.x.to_radians(), angle.y.to_radians(), angle.z.to_radians())
        };
        self.vertices.iter_mut().for_each(|p| p.rotate(angle));
    }

    /// Rotate the whole object
    ///
    /// Similar to [`rotate`](struct.Object3D.html#method.rotate) but will return a new Object3D
    // pub fn rotate_new(&self, angle: Vec3) -> Self {
    //     let mut obj = self.clone();
    //     obj.rotate(angle);
    //     obj
    // }

    /// Zoom the whole object
    ///
    /// * `factor` - magnification of zoom, must bigger than 0.001
    ///
    /// Because of the implementation, it won't grow the error of f64 in most time.
    /// But if after many times call it, the error is grow, consider use [`zoom_new`](struct.Object3D.html#method.zoom_new)
    // pub fn scale(&mut self, factor: f64) {
    //     check_zoom(factor);
    //     let mut vertices = self.vertices.clone();
    //     vertices.into_iter().for_each(|v| v.scale()
    //         .iter_mut()
    //         .for_each(|v| v.zoom(self.center, factor));
    //     self.affined = Some(vertices);
    // }

    /// Zoom the whole object
    ///
    /// * `factor` - magnification of zoom, must bigger than 0.001
    ///
    /// Instead of change the original object, it returns a new one.
    /// It can forbide the growing of error of f64
    // pub fn zoom_new(&self, factor: f64) -> Self {
    //     check_zoom(factor);
    //     let mut points = self.vertices.clone();
    //     points.iter_mut().for_each(|v| v.zoom(self.center, factor));
    //     Self {
    //         vertices: points,
    //         affined: None,
    //         sides: self.sides.clone(),
    //         center: self.center,
    //     }
    // }

    fn calc_center(&mut self) {
        let (mut xs, mut ys, mut zs) = (Vec::new(), Vec::new(), Vec::new());
        for p in &self.vertices {
            xs.push(p.x);
            ys.push(p.y);
            zs.push(p.z);
        }
        let mean = Pos3::new(mean_f32(&xs), mean_f32(&ys), mean_f32(&zs));

        // forbide the lost of f64
        if mean.distance(self.center) < MIN_DIFFERENCE as f32 {
            return;
        } else {
            self.center = mean;
        }
    }
}

impl Paint for Object3D {
    fn paint<T>(&self, canvas: &mut Canvas, x: T, y: T)
    where
        T: Into<f64>,
    {
        let (x, y) = (x.into(), y.into());
        let points = &self.vertices;

        for side in &self.sides {
            let (v1, v2) = (points[side.side.0], points[side.side.1]);
            let xy1 = (x + v1.x as f64, y + v1.z as f64);
            let xy2 = (x + v2.x as f64, y + v2.z as f64);
            if let Some(color) = side.color {
                canvas.line_colorful(xy1, xy2, color);
            } else {
                canvas.line(xy1, xy2);
            }
        }
    }
}

impl Object3D {
    #[cfg(feature = "obj-rs")]
    pub fn from_obj(path: &str, f: f32) -> Result<Self, RsilleErr> {
        use std::fs::File;
        use std::io::BufReader;
        use obj::{load_obj, Obj};

        use crate::to_rsille_err;

        let input = BufReader::new(File::open(path).map_err(to_rsille_err)?);
        let dome: Obj = load_obj(input).map_err(to_rsille_err)?;
        let mut sides = Vec::new();
        for is in dome.indices.windows(2) {
            sides.push(Side {
                side: (is[0].into(), is[1].into()),
                color: None
            })
        }
        let vs = dome.vertices.iter().map(|v| {
            let p = v.position;
            Pos3::new(p[0] * f, p[1] * f, p[2] * f)
        }).collect();
        Ok(Self {
            vertices: vs,
            sides,
            center: Pos3::ZERO,
            radians: false
        })
    }

    /// Make a cube
    pub fn cube<T>(side_len: T) -> Object3D
    where
        T: Into<f64>,
    {
        let side_len = side_len.into();
        let mut object = Object3D::new();
        #[rustfmt::skip]
        // the vertices of cube
        let a = [
            (-1, -1, -1),
            (-1, -1,  1),
            (-1,  1, -1),
            ( 1, -1, -1),
            (-1,  1,  1),
            ( 1, -1,  1),
            ( 1,  1, -1),
            ( 1,  1,  1),
        ];
        let mut points = Vec::new();
        for i in a {
            let x = side_len as f32 / 2.0 * i.0 as f32;
            let y = side_len as f32 / 2.0 * i.1 as f32;
            let z = side_len as f32 / 2.0 * i.2 as f32;
            points.push(Pos3::new(x, y, z));
        }
        object.add_points(&points);
        object
            .add_sides(&[
                (0, 1),
                (1, 4),
                (4, 2),
                (2, 0),
                (3, 5),
                (5, 7),
                (7, 6),
                (6, 3),
                (1, 5),
                (4, 7),
                (2, 6),
                (0, 3),
            ])
            .unwrap();
        object
    }
}
trait Affine {
    fn rotate(&mut self, angle: Vec3);
    fn scale(&mut self, factor: Vec3);
    fn rotate_x(&mut self, angle: f32);
    fn rotate_y(&mut self, angle: f32);
    fn rotate_z(&mut self, angle: f32);
}

impl Affine for Pos3 {
    fn rotate(&mut self, angle: Vec3) {
        self.rotate_x(angle.x);
        self.rotate_y(angle.y);
        self.rotate_z(angle.z);
    }

    fn scale(&mut self, scale: Vec3) {
        *self = Affine3A::from_scale(scale.into()).transform_point3a(*self);
    }

    fn rotate_x(&mut self, angle: f32) {
        *self = Affine3A::from_rotation_x(angle).transform_point3a(*self);
    }

    fn rotate_y(&mut self, angle: f32) {
        *self = Affine3A::from_rotation_y(angle).transform_point3a(*self);
    }

    fn rotate_z(&mut self, angle: f32) {
        *self = Affine3A::from_rotation_z(angle).transform_point3a(*self);
    }
}

// point in 3D
//
// support rotate and zoom, not paintable
// #[derive(Debug, Clone, Copy)]
// struct Point3D {
//     pub(crate) x: f64,
//     pub(crate) y: f64,
//     pub(crate) z: f64,
// }

// #[allow(unused)]
// impl Point3D {
//     /// construct a new point
//     fn new(x: f64, y: f64, z: f64) -> Self {
//         Self { x, y, z }
//     }

//     /// similar to new, but use a tuple
//     fn from(xyz: (f64, f64, f64)) -> Self {
//         Self {
//             x: xyz.0,
//             y: xyz.1,
//             z: xyz.2,
//         }
//     }

//     /// get the coordinate
//     fn get(&self) -> (f64, f64, f64) {
//         (self.x, self.y, self.z)
//     }

//     /// rotate the point, see [wiki] for more information
//     ///
//     /// [wiki]: <https://en.wikipedia.org/wiki/Rotation_matrix>
//     fn rotate(&mut self, angle: (f64, f64, f64)) {
//         self.rotate_x(angle.0);
//         self.rotate_y(angle.1);
//         self.rotate_z(angle.2);

//         // let (x, y, z) = (self.x, self.y, self.z);
//         // let (sx, cx) = anlge_x.to_radians().sin_cos();
//         // let (sy, cy) = anlge_y.to_radians().sin_cos();
//         // let (sz, cz) = angle_z.to_radians().sin_cos();
//         // let (t1, t2, t3) = (
//         //     x * cy + y * sx * sy + z * cx * sy,
//         //     y * cx - z * sx,
//         //     y * sx + z * cx,
//         // );
//         // self.x = cz * t1 - sz * t2;
//         // self.y = sz * t1 + cz * t2;
//         // self.z = cz * t3 - sy * x;
//     }

//     /// similar to [rotate] but don't change the original point and return the rotated point
//     ///
//     /// [rotate]: struct.Point3D.html#method.rotate
//     fn rotate_new(&self, angle: (f64, f64, f64)) -> Self {
//         let mut point = *self;
//         point.rotate(angle);
//         point
//     }

//     /// Zoom coordinate of the point with the center
//     ///
//     /// It will change the coordinate, so don't call it many times, precision errors of f64 are cumulative!
//     ///
//     /// In most time, you shouldn't use it, just use the [`zoom`](struct.Object3D.html#method.zoom) in Object3D
//     fn zoom(&mut self, center: Self, factor: f64) {
//         check_zoom(factor);
//         self.x = (self.x - center.x) * factor;
//         self.y = (self.y - center.y) * factor;
//         self.z = (self.z - center.z) * factor;
//     }

//     /// Zoom coordinate of the point with the center and return a new point
//     ///
//     /// It won't change the original point, so the precision error isn't matter
//     fn zoom_new(&self, center: Self, factor: f64) -> Point3D {
//         check_zoom(factor);
//         let dx = (self.x - center.x) * factor;
//         let dy = (self.y - center.y) * factor;
//         let dz = (self.z - center.y) * factor;

//         Self {
//             x: dx,
//             y: dy,
//             z: dz,
//         }
//     }

//     /// rotate by an angle about the x axis
//     fn rotate_x(&mut self, angle: f64) {
//         let (s, c) = angle.to_radians().sin_cos();
//         let (y, z) = (self.y, self.z);
//         self.y = y * c - z * s;
//         self.z = y * s + z * c;
//     }

//     /// rotate by an angle about the y axis
//     fn rotate_y(&mut self, angle: f64) {
//         let (s, c) = angle.to_radians().sin_cos();
//         let (x, z) = (self.x, self.z);
//         self.x = x * c + z * s;
//         self.z = -x * s + z * c;
//     }

//     /// rotate by an angle about the z axis
//     fn rotate_z(&mut self, anlge: f64) {
//         let (s, c) = anlge.to_radians().sin_cos();
//         let (x, y) = (self.x, self.y);
//         self.x = x * c - y * s;
//         self.y = x * s + y * c;
//     }
// }
