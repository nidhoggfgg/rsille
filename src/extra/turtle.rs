// TODO: remove the extra things for animation in Turtle

use std::f64::consts::PI;

use crate::{
    canvas::Paint,
    utils::{RsilleErr, MIN_DIFFERENCE},
    Canvas,
};

use crate::color::Color;

/// The turtle impl of braille code in Rust
///
/// All the api is similar to turtle in Python
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
/// just paint it
/// ```
/// use rsille::{extra::Turtle, Canvas};
/// let mut canvas = Canvas::new();
/// let mut t = Turtle::new();
/// let mut length = 1.0;
/// for _ in 0..150 {
///     t.forward(length);
///     t.right(10.0);
///     length += 0.05;
/// }
/// canvas.paint(&t, 50.0, 50.0).unwrap();
/// canvas.print();
/// ```
///
/// or a animation
/// ```no_run
/// use rsille::{extra::Turtle, Animation};
/// let mut anime = Animation::new();
/// let mut t = Turtle::new();
/// let mut length = 1.0;
/// for _ in 0..150 {
///     t.forward(length);
///     t.right(10.0);
///     length += 0.05;
/// }
/// t.anime();
/// anime.push(t, move |t| t.update(), (50.0, 50.0));
/// anime.run();
/// ```
///
/// ## NOTE:
///
/// There isn't position or heading function,
/// because the position and heading can't know until paint it!
/// When you call forward or circle or any other method, it will only record the procedure.
/// Then in the paint function, it will do those procedures and paint the canvas.
#[derive(Debug, Clone, PartialEq)]
pub struct Turtle {
    procedures: Vec<Procedure>,
    anime_proc: Option<Vec<Procedure>>,
    anime_step: f64,
    frame_count: usize,
}

impl Turtle {
    /// return a new Turtle
    pub fn new() -> Self {
        Self {
            procedures: Vec::new(),
            anime_proc: None,
            anime_step: 10.0,
            frame_count: 0,
        }
    }

    /// pen up, the turtle won't draw when moving
    pub fn penup(&mut self) {
        self.add_procedure(Procedure::PenUp);
    }

    /// alias: [`penup`](struct.Turtle.html#method.penup)
    pub fn up(&mut self) {
        self.penup();
    }

    /// pen down, the turtle will draw when moving
    pub fn pendown(&mut self) {
        self.add_procedure(Procedure::PenDown);
    }

    /// alias: [`pendown`](struct.Turtle.html#method.pendown)
    pub fn down(&mut self) {
        self.pendown();
    }

    /// Move the turtle forward by the specified distance, in the direction the turtle is headed.
    /// * `step` - the distance
    pub fn forward<T>(&mut self, step: T) where T: Into<f64> {
        self.add_procedure(Procedure::Forward(step.into()));
    }

    /// alias: [`forward`](struct.Turtle.html#method.forward)
    pub fn fd<T>(&mut self, step: T) where T: Into<f64> {
        self.forward(step);
    }

    /// Move the turtle backward by distance, opposite to the direction the turtle is headed.
    /// It won't change the turtle’s heading.
    /// * `step` - the distance
    pub fn backward<T>(&mut self, step: T) where T: Into<f64> {
        self.forward(-step.into());
    }

    /// alias: [`backward`](struct.Turtle.html#method.backward)
    pub fn back<T>(&mut self, step: T) where T: Into<f64> {
        self.backward(step);
    }

    /// alias: [`backward`](struct.Turtle.html#method.backward)
    pub fn bk<T>(&mut self, step: T) where T: Into<f64> {
        self.backward(step);
    }

    /// Turn turtle right by angle units.
    /// * `angle` - the degree
    pub fn right<T>(&mut self, angle: T) where T: Into<f64> {
        self.add_procedure(Procedure::Right(angle.into()));
    }

    /// alias: [`right`](struct.Turtle.html#method.right)
    pub fn rt<T>(&mut self, angle: T) where T: Into<f64> {
        self.right(angle);
    }

    /// Turn turtle left by angle units.
    /// * `angle` - the degree
    pub fn left<T>(&mut self, angle: T) where T: Into<f64> {
        self.right(-angle.into());
    }

    /// alias: [`left`](struct.Turtle.html#method.left)
    pub fn lt<T>(&mut self, angle: T) where T: Into<f64> {
        self.left(angle);
    }

    /// Draw a circle with given radius.
    /// The center is radius units left of the turtle;
    /// * `extent` – an angle – determines which part of the circle is drawn. If extent is not a full circle, one endpoint of the arc is the current pen position.
    /// * `radius` - Draw the arc in counterclockwise direction if radius is positive, otherwise in clockwise direction. Finally the direction of the turtle is changed by the amount of extent.
    pub fn circle<T>(&mut self, radius: T, extent: T) where T: Into<f64> {
        self.add_procedure(Procedure::Circle(radius.into(), extent.into(), 100));
    }

    /// Draw a circle with given radius.
    /// The center is radius units left of the turtle;
    /// * `extent` – an angle – determines which part of the circle is drawn. If extent is not a full circle, one endpoint of the arc is the current pen position.
    /// * `radius` - Draw the arc in counterclockwise direction if radius is positive, otherwise in clockwise direction. Finally the direction of the turtle is changed by the amount of extent.
    /// * `steps` - Suggest 100. As the circle is approximated by an inscribed regular polygon, steps determines the number of steps to use.
    ///
    /// suggest use [`circle`](struct.Turtle.html#method.circle)
    pub fn circle_with_steps<T>(&mut self, radius: T, extent: T, steps: usize) where T: Into<f64> {
        self.add_procedure(Procedure::Circle(radius.into(), extent.into(), steps));
    }

    /// Move turtle to an absolute position. If the pen is down, draw line.
    /// * `x` - the position of x
    /// * `y` - the position of y
    pub fn goto<T>(&mut self, x: T, y: T) where T : Into<f64>{
        self.add_procedure(Procedure::Goto(x.into(), y.into()));
    }

    /// Move turtle to an absolute position and a line will not be drawn.
    /// * `x` - the position of x
    /// * `y` - the position of y
    pub fn teleport<T>(&mut self, x: T, y: T) where T: Into<f64> {
        self.add_procedure(Procedure::Teleport(x.into(), y.into()));
    }

    /// Move turtle to the origin – coordinates (0,0) – and set its heading to its start-orientation
    pub fn home(&mut self) {
        self.add_procedure(Procedure::Home);
    }

    /// Set the color of turtle
    pub fn colorful(&mut self, color: Color) {
        self.add_procedure(Procedure::Colorful(color));
    }

    /// alias: [`colorful`](struct.Turtle.html#method.colorful)
    pub fn color(&mut self, color: Color) {
        self.colorful(color);
    }

    /// Build the Turtle for animation
    ///
    /// If you don't need the animation, then don't call this method
    pub fn anime(&mut self) {
        use Procedure::*;
        let mut anime_proc = Vec::new();
        let astep = self.anime_step;
        for p in &self.procedures {
            match p {
                PenDown | PenUp | Right(_) | Teleport(_, _) | Home => {
                    anime_proc.push(*p);
                }
                Forward(step) => {
                    let mut step = *step;
                    while step > astep {
                        anime_proc.push(Forward(astep));
                        step -= astep;
                    }
                    // forbide the lost of f64
                    if (step - astep).abs() < MIN_DIFFERENCE {
                        anime_proc.push(Forward(astep));
                    } else {
                        anime_proc.push(Forward(step));
                    }
                }
                Goto(_, _) => {
                    // FIXME: make the goto anime
                    anime_proc.push(*p);
                }
                Circle(radius, extent, steps) => {
                    let mut extent = *extent;
                    if PI * radius * extent <= 180.0 * astep {
                        anime_proc.push(*p);
                    } else {
                        let e = 180.0 * astep / (PI * radius);
                        while extent > e {
                            anime_proc.push(Circle(*radius, e, *steps));
                            extent -= e;
                        }
                        if (extent - e).abs() > MIN_DIFFERENCE {
                            anime_proc.push(Circle(*radius, extent, *steps));
                        }
                    }
                }
                _ => anime_proc.push(*p),
            }
        }
        self.anime_proc = Some(anime_proc);
    }

    /// Generate the next frame of the animation
    ///
    /// return true if the animation is over
    ///
    /// If you don't need the animation, don't need to call this method.
    /// And you should call [`anime`](struct.Turtle.html#method.anime) first
    pub fn update(&mut self) -> bool {
        use Procedure::*;
        if let Some(procs) = &self.anime_proc {
            if self.frame_count >= procs.len() {
                return true;
            }
            while let Some(p) = procs.get(self.frame_count) {
                match p {
                    Forward(_) | Goto(_, _) | Circle(_, _, _) => {
                        self.frame_count += 1;
                        break;
                    }
                    _ => {
                        self.frame_count += 1;
                    }
                }
            }
        }
        false
    }

    /// Set the step of the animation
    ///
    /// The default of Turtle is 10.0, if you want to change it, you can call this method
    ///
    /// For example:
    /// * *forward(100.0) -> forward(10.0) * 10*
    /// * but *forward(5.0) -> forward(5.0)*
    ///
    /// Beacuse like *forward(5.0), right(10.0), forward(5.0)* can't fold to *forward(10.0), right(10.0)*
    pub fn set_anime_step<T>(&mut self, step: T) where T: Into<f64> {
        self.anime_step = step.into();
    }

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
    Colorful(Color),
}

fn forward(
    canvas: &mut Canvas,
    x: f64,
    y: f64,
    heading: f64,
    pen: bool,
    step: f64,
    color: Color,
) -> (f64, f64) {
    let (sr, cr) = heading.to_radians().sin_cos();
    let txy = (x + cr * step, y + sr * step);
    if pen {
        canvas.line_colorful((x, y), txy, color);
    }
    txy
}

impl Paint for Turtle {
    fn paint<T>(&self, canvas: &mut Canvas, x: T, y: T) -> Result<(), RsilleErr>
    where
        T: Into<f64>,
    {
        use Procedure::*;
        let (x, y) = (x.into(), y.into());
        let (home_x, home_y) = (x, y);
        let (mut pen, mut heading, mut x, mut y) = (true, 0.0, x, y);
        let mut color = Color::Reset;
        let procs = if let Some(procs) = &self.anime_proc {
            &procs[0..self.frame_count]
        } else {
            &self.procedures
        };

        for p in procs {
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
                    heading -= angle;
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
                Colorful(c) => {
                    color = *c;
                }
            }
        }
        Ok(())
    }
}
