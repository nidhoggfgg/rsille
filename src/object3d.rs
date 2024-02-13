//! Object in 3D
//!
//! ## Example
//!
//! draw a line
//! ```
//! use rsille::{object3d::Object3D, Canvas};
//! let mut canvas = Canvas::new();
//! let mut object = Object3D::new();
//! let points = [(-10.0, -10.0, -10.0), (10.0, 10.0, 10.0)];
//! let sides = [(0, 1)]; // connect these 2 point
//! object.add_points(&points);
//! object.add_sides(&sides).unwrap();
//! canvas.paint(&object, 0.0, 0.0).unwrap();
//! println!("{}", canvas.frame());
//! ```

use crate::{
    canvas::Paint,
    utils::{check_zoom, mean, RsilleErr, MIN_DIFFERENCE},
    Canvas,
};

#[cfg(feature = "color")]
use crate::color::TermColor;
#[cfg(feature = "color")]
use std::collections::HashMap;

/// # A paintable Object in 3D
///
/// make object easy and easy to do something like rotate, zoom and more
/// also, it support colorful output
///

#[derive(Debug, Clone)]
pub struct Object3D {
    origin_vertices: Vec<Point3D>,
    zoomed_vertices: Option<Vec<Point3D>>,
    center: Point3D,

    #[cfg(not(feature = "color"))]
    sides: Vec<(usize, usize)>,

    #[cfg(feature = "color")]
    sides: HashMap<(usize, usize), TermColor>,
}

impl Object3D {
    /// construct a new Object3D, with no vertices and sides
    pub fn new() -> Self {
        Self {
            origin_vertices: Vec::new(),
            zoomed_vertices: None,
            center: Point3D::new(0.0, 0.0, 0.0),

            #[cfg(not(feature = "color"))]
            sides: Vec::new(),

            #[cfg(feature = "color")]
            sides: HashMap::new(),
        }
    }

    /// return the vertices
    pub fn vertices(&self) -> Vec<(f64, f64, f64)> {
        self.origin_vertices.iter().map(|p| p.get()).collect()
    }

    /// add vertices to object
    pub fn add_points(&mut self, points: &[(f64, f64, f64)]) {
        for p in points {
            self.origin_vertices.push(Point3D::from(*p));
        }
        self.calc_center();
    }

    /// return the sides
    pub fn sides(&self) -> Vec<(usize, usize)> {
        #[cfg(not(feature = "color"))]
        return self.sides.clone();

        #[cfg(feature = "color")]
        return self.sides.keys().cloned().collect();
    }

    /// add sides to object
    pub fn add_sides(&mut self, sides: &[(usize, usize)]) -> Result<(), RsilleErr> {
        let vn = self.origin_vertices.len();
        for side in sides {
            if vn <= side.0 || vn <= side.1 {
                return Err(RsilleErr::new("wrong add sides!".to_string()));
            }

            #[cfg(not(feature = "color"))]
            self.sides.push(*side);

            #[cfg(feature = "color")]
            self.sides.insert(*side, TermColor::None);
        }
        Ok(())
    }

    /// add sides and color of those sides
    #[cfg(feature = "color")]
    pub fn add_sides_colorful(
        &mut self,
        sides: &[((usize, usize), TermColor)],
    ) -> Result<(), RsilleErr> {
        let vn = self.origin_vertices.len();
        for (side, color) in sides {
            if vn <= side.0 || vn <= side.1 {
                return Err(RsilleErr::new("wrong add sides!".to_string()));
            }
            self.sides.insert(*side, *color);
        }
        Ok(())
    }

    /// set the color of the side
    #[cfg(feature = "color")]
    pub fn set_side_colorful(&mut self, side: (usize, usize), color: TermColor) {
        if self.sides.contains_key(&side) {
            self.sides.insert(side, color);
        }
    }

    /// rotate the whole object, about [rotate]
    ///
    /// [rotate]: <https://en.wikipedia.org/wiki/Rotation_matrix>
    pub fn rotate(&mut self, angle: (f64, f64, f64)) {
        for p in &mut self.origin_vertices {
            p.rotate(angle);
        }
    }

    /// rotate the whole object, similar to [`rotate`]
    /// normaly, the rotate won't grow the error of f64.
    /// if the error is growing, use this function for return a new Object3D is useful
    ///
    /// [`rotate`]: struct.Object3D.html#method.rotate
    pub fn rotate_new(&self, angle: (f64, f64, f64)) -> Self {
        let mut obj = self.clone();
        obj.rotate(angle);
        obj
    }

    /// zoom the whole object
    /// because of the implementation, it won't grow the error of f64 in most time
    /// but if after many times call it, the error is grow, consider use [`zoom_new`]
    /// * `factor` - magnification of zoom, must biger than 0.001
    ///
    /// [`zoom_new`]: struct.Object3D.html#method.zoom_new
    pub fn zoom(&mut self, factor: f64) {
        check_zoom(factor);
        let mut vertices = self.origin_vertices.clone();
        vertices
            .iter_mut()
            .for_each(|v| v.zoom(self.center, factor));
        self.zoomed_vertices = Some(vertices);
    }

    /// zoom the whole object
    /// * `factor` - magnification of zoom, must biger than 0.001
    ///
    /// instead of change the original object, it returns a new one
    /// it can forbide the growing of error of f64
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
    fn paint(&self, canvas: &mut Canvas, x: f64, y: f64) -> Result<(), RsilleErr> {
        let points = if let Some(p) = &self.zoomed_vertices {
            p
        } else {
            &self.origin_vertices
        };

        #[cfg(not(feature = "color"))]
        for side in &self.sides {
            let (v1, v2) = (points[side.0], points[side.1]);
            let xy1 = (x + v1.x, y + v1.z);
            let xy2 = (x + v2.x, y + v2.z);
            canvas.line(xy1, xy2);
        }

        #[cfg(feature = "color")]
        for (side, color) in &self.sides {
            let (v1, v2) = (points[side.0], points[side.1]);
            let xy1 = (x + v1.x, y + v1.z);
            let xy2 = (x + v2.x, y + v2.z);
            canvas.line_colorful(xy1, xy2, *color);
        }

        Ok(())
    }
}

/// point in 3D
///
/// support rotate and zoom, not paintable
#[derive(Debug, Clone, Copy)]
struct Point3D {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) z: f64,
}

#[allow(unused)]
impl Point3D {
    /// construct a new point
    fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// similar to new, but use a tuple
    fn from(xyz: (f64, f64, f64)) -> Self {
        Self {
            x: xyz.0,
            y: xyz.1,
            z: xyz.2,
        }
    }

    /// get the coordinate
    fn get(&self) -> (f64, f64, f64) {
        (self.x, self.y, self.z)
    }

    /// rotate the point, see [wiki] for more information
    ///
    /// [wiki]: <https://en.wikipedia.org/wiki/Rotation_matrix>
    fn rotate(&mut self, angle: (f64, f64, f64)) {
        self.rotate_x(angle.0);
        self.rotate_y(angle.1);
        self.rotate_z(angle.2);

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
    }

    /// similar to [rotate] but don't change the original point and return the rotated point
    ///
    /// [rotate]: struct.Point3D.html#method.rotate
    fn rotate_new(&self, angle: (f64, f64, f64)) -> Self {
        let mut point = *self;
        point.rotate(angle);
        point
    }

    /// Zoom coordinate of the point with the center
    ///
    /// It will change the coordinate, so don't call it many times, precision errors of f64 are cumulative!
    ///
    /// In most time, you shouldn't use it, just use the [`zoom`](struct.Object3D.html#method.zoom) in Object3D
    fn zoom(&mut self, center: Self, factor: f64) {
        check_zoom(factor);
        self.x = (self.x - center.x) * factor;
        self.y = (self.y - center.y) * factor;
        self.z = (self.z - center.z) * factor;
    }

    /// Zoom coordinate of the point with the center and return a new point
    ///
    /// It won't change the original point, so the precision error isn't matter
    fn zoom_new(&self, center: Self, factor: f64) -> Point3D {
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

    /// rotate by an angle about the x axis
    fn rotate_x(&mut self, angle: f64) {
        let (s, c) = angle.to_radians().sin_cos();
        let (y, z) = (self.y, self.z);
        self.y = y * c - z * s;
        self.z = y * s + z * c;
    }

    /// rotate by an angle about the y axis
    fn rotate_y(&mut self, angle: f64) {
        let (s, c) = angle.to_radians().sin_cos();
        let (x, z) = (self.x, self.z);
        self.x = x * c + z * s;
        self.z = -x * s + z * c;
    }

    /// rotate by an angle about the z axis
    fn rotate_z(&mut self, anlge: f64) {
        let (s, c) = anlge.to_radians().sin_cos();
        let (x, y) = (self.x, self.y);
        self.x = x * c - y * s;
        self.y = x * s + y * c;
    }
}
