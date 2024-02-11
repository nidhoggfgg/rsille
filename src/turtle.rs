use crate::{canvas::Paint, utils::RsilleErr, Canvas};

#[cfg(feature = "color")]
use crate::color::TermColor;

/// # The turtle impl of braille code in Rust
///
/// all the api is similar to turtle in Python
///
/// ## Features
///
/// - Basic turtle movement: forward, backward, turn left, turn right.
/// - Pen control: pen up, pen down.
/// - Drawing shapes: circles.
/// - Position and Color.
///
/// ## Example
///
/// ```rust
/// use rsille::{Turtle, Canvas};
///
/// let mut canvas = Canvas::new();
/// let mut t = Turtle::new();
/// let mut length = 1.0;
/// for i in 0..150 {
///     t.forward(length);
///     t.right(10.0);
///     length += 0.05;
/// }
/// canvas.paint(&t, 30.0, 30.0).unwrap();
/// println!("{}", canvas.frame());
///
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Turtle {
    procedures: Vec<Procedure>,
}

impl Turtle {
    /// return a new Turtle
    pub fn new() -> Self {
        Self {
            procedures: Vec::new(),
        }
    }

    /// pen up, the turtle won't draw when moving
    pub fn penup(&mut self) {
        self.add_procedure(Procedure::PenUp);
    }

    /// equal to [`penup`](struct.Turtle.html#method.penup)
    pub fn up(&mut self) {
        self.penup();
    }

    /// pen down, the turtle will draw when moving
    pub fn pendown(&mut self) {
        self.add_procedure(Procedure::PenDown);
    }

    /// equal to [`pendown`](struct.Turtle.html#method.pendown)
    pub fn down(&mut self) {
        self.pendown();
    }

    /// Move the turtle forward by the specified distance, in the direction the turtle is headed.
    /// * `step` - the distance
    pub fn forward(&mut self, step: f64) {
        self.add_procedure(Procedure::Forward(step));
    }

    /// equal to [`forward`](struct.Turtle.html#method.forward)
    pub fn fd(&mut self, step: f64) {
        self.forward(step);
    }

    /// Move the turtle backward by distance, opposite to the direction the turtle is headed. Do not change the turtle’s heading.
    /// * `step` - the distance
    pub fn backward(&mut self, step: f64) {
        self.forward(-step);
    }

    /// equal to [`backward`](struct.Turtle.html#method.backward)
    pub fn back(&mut self, step: f64) {
        self.backward(step);
    }

    /// equal to [`backward`](struct.Turtle.html#method.backward)
    pub fn bk(&mut self, step: f64) {
        self.backward(step);
    }

    /// Turn turtle right by angle units.
    /// * `angle` - the degree
    pub fn right(&mut self, angle: f64) {
        self.add_procedure(Procedure::Right(angle));
    }

    /// equal to [`right`](struct.Turtle.html#method.right)
    pub fn rt(&mut self, angle: f64) {
        self.right(angle);
    }

    /// Turn turtle left by angle units.
    /// * `angle` - the degree
    pub fn left(&mut self, angle: f64) {
        self.right(-angle);
    }

    /// equal to [`left`](struct.Turtle.html#method.left)
    pub fn lt(&mut self, angle: f64) {
        self.left(angle);
    }

    /// Draw a circle with given radius.
    /// The center is radius units left of the turtle;
    /// * `extent` – an angle – determines which part of the circle is drawn. If extent is not a full circle, one endpoint of the arc is the current pen position.
    /// * `radius` - Draw the arc in counterclockwise direction if radius is positive, otherwise in clockwise direction. Finally the direction of the turtle is changed by the amount of extent.
    /// * `steps` - Suggest 100. As the circle is approximated by an inscribed regular polygon, steps determines the number of steps to use.
    pub fn circle(&mut self, radius: f64, extent: f64, steps: usize) {
        self.add_procedure(Procedure::Circle(radius, extent, steps));
    }

    /// Move turtle to an absolute position. If the pen is down, draw line.
    /// * `x` - the position of x
    /// * `y` - the position of y
    pub fn goto(&mut self, x: f64, y: f64) {
        self.add_procedure(Procedure::Goto(x, y));
    }

    /// Move turtle to an absolute position and a line will not be drawn.
    /// * `x` - the position of x
    /// * `y` - the position of y
    pub fn teleport(&mut self, x: f64, y: f64) {
        self.add_procedure(Procedure::Teleport(x, y));
    }

    /// Move turtle to the origin – coordinates (0,0) – and set its heading to its start-orientation
    pub fn home(&mut self) {
        self.add_procedure(Procedure::Home);
    }

    /// Set the color of turtle
    #[cfg(feature = "color")]
    pub fn colorful(&mut self, color: TermColor) {
        self.add_procedure(Procedure::Color(color));
    }

    /// equal to [`colorful`](struct.Turtle.html#method.colorful)
    #[cfg(feature = "color")]
    pub fn color(&mut self, color: TermColor) {
        self.colorful(color);
    }

    // pub fn position(&self) -> (f64, f64) {
    //     (self.x, self.y)
    // }

    // pub fn pos(&self) -> (f64, f64) {
    //     self.position()
    // }

    // pub fn heading(&self) -> f64 {
    //     self.heading
    // }

    fn add_procedure(&mut self, p: Procedure) {
        self.procedures.push(p);
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum Procedure {
    PenDown,
    PenUp,
    Forward(f64), // step
    // Backward(f64), == - Forward
    Right(f64), // angle
    // Left(f64), == - Right
    Teleport(f64, f64), // (x, y)
    Home,
    Goto(f64, f64),          // (x, y)
    Circle(f64, f64, usize), // (radius, extent, steps)

    #[cfg(feature = "color")]
    Color(TermColor),
}

#[cfg(not(feature = "color"))]
fn forward(canvas: &mut Canvas, x: f64, y: f64, heading: f64, pen: bool, step: f64) -> (f64, f64) {
    let (sr, cr) = heading.to_radians().sin_cos();
    let txy = (x + cr * step, y + sr * step);
    if pen {
        canvas.line((x, y), txy);
    }
    txy
}

#[cfg(feature = "color")]
fn forward(
    canvas: &mut Canvas,
    x: f64,
    y: f64,
    heading: f64,
    pen: bool,
    step: f64,
    color: TermColor,
) -> (f64, f64) {
    let (sr, cr) = heading.to_radians().sin_cos();
    let txy = (x + cr * step, y + sr * step);
    if pen {
        canvas.line_colorful((x, y), txy, color);
    }
    txy
}

#[cfg(not(feature = "color"))]
impl Paint for Turtle {
    fn paint(&self, canvas: &mut Canvas, x: f64, y: f64) -> Result<(), RsilleErr> {
        use Procedure::*;
        let (home_x, home_y) = (x, y);
        let (mut pen, mut heading, mut x, mut y) = (true, 0.0, x, y);

        for p in &self.procedures {
            match p {
                PenDown => {
                    pen = true;
                }
                PenUp => {
                    pen = false;
                }
                Forward(step) => {
                    (x, y) = forward(canvas, x, y, heading, pen, *step);
                }
                Right(angle) => {
                    heading += angle;
                }
                Teleport(tx, ty) => {
                    x = *tx;
                    y = *ty;
                }
                Home => {
                    (x, y) = (home_x, home_y);
                }
                Goto(tx, ty) => {
                    if pen {
                        canvas.line((x, y), (*tx, *ty));
                    }
                    x = *tx;
                    y = *ty;
                }
                Circle(radius, extent, steps) => {
                    let angle = extent / *steps as f64;
                    for _ in 0..*steps {
                        (x, y) = forward(
                            canvas,
                            x,
                            y,
                            heading,
                            pen,
                            2.0 * radius * (angle / 2.0).to_radians().sin(),
                        );
                        heading -= angle;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(feature = "color")]
impl Paint for Turtle {
    fn paint(&self, canvas: &mut Canvas, x: f64, y: f64) -> Result<(), RsilleErr> {
        use Procedure::*;
        let (home_x, home_y) = (x, y);
        let (mut pen, mut heading, mut x, mut y) = (true, 0.0, x, y);
        let mut color = TermColor::None;

        for p in &self.procedures {
            match p {
                PenDown => {
                    pen = true;
                }
                PenUp => {
                    pen = false;
                }
                Forward(step) => {
                    (x, y) = forward(canvas, x, y, heading, pen, *step, color);
                }
                Right(angle) => {
                    heading += angle;
                }
                Teleport(tx, ty) => {
                    x = *tx;
                    y = *ty;
                }
                Home => {
                    (x, y) = (home_x, home_y);
                }
                Goto(tx, ty) => {
                    if pen {
                        canvas.line_colorful((x, y), (*tx, *ty), color);
                    }
                    x = *tx;
                    y = *ty;
                }
                Circle(radius, extent, steps) => {
                    let angle = extent / *steps as f64;
                    for _ in 0..*steps {
                        (x, y) = forward(
                            canvas,
                            x,
                            y,
                            heading,
                            pen,
                            2.0 * radius * (angle / 2.0).to_radians().sin(),
                            color,
                        );
                        heading -= angle;
                    }
                }
                Color(c) => {
                    color = *c;
                }
            }
        }
        Ok(())
    }
}
