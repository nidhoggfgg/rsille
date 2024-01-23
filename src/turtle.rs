use crate::{canvas::Draw, Canvas};

#[derive(Debug, Clone, PartialEq)]
pub struct Turtle {
    procedures: Vec<Procedure>,
}

impl Turtle {
    pub fn new() -> Self {
        Self {
            procedures: Vec::new(),
        }
    }

    pub fn penup(&mut self) {
        self.add_procedure(Procedure::PenUp);
    }

    pub fn up(&mut self) {
        self.penup();
    }

    pub fn pendown(&mut self) {
        self.add_procedure(Procedure::PenDown);
    }

    pub fn down(&mut self) {
        self.pendown();
    }

    pub fn forward(&mut self, step: f64) {
        self.add_procedure(Procedure::Forward(step));
        // let (sr, cr) = self.heading.to_radians().sin_cos();
        // let x = self.x + cr * step;
        // let y = self.y + sr * step;
        // let prev_brush = self.pen_on;
        // self.pen_on = true;
        // self.goto(x, y);
        // self.pen_on = prev_brush;
    }

    pub fn fd(&mut self, step: f64) {
        self.forward(step);
    }

    pub fn backward(&mut self, step: f64) {
        self.forward(-step);
    }

    pub fn back(&mut self, step: f64) {
        self.backward(step);
    }

    pub fn bk(&mut self, step: f64) {
        self.backward(step);
    }

    pub fn right(&mut self, angle: f64) {
        self.add_procedure(Procedure::Right(angle));
    }

    pub fn rt(&mut self, angle: f64) {
        self.right(angle);
    }

    pub fn left(&mut self, angle: f64) {
        self.right(-angle);
    }

    pub fn lt(&mut self, angle: f64) {
        self.left(angle);
    }

    pub fn circle(&mut self, radius: f64, extent: f64, steps: usize) {
        self.add_procedure(Procedure::Circle(radius, extent, steps));
        // let angle = extent / steps as f64;
        // for _ in 0..steps {
        //     self.forward(2.0 * radius * (angle / 2.0).to_radians().sin());
        //     self.left(angle);
        // }
    }

    pub fn goto(&mut self, x: f64, y: f64) {
        self.add_procedure(Procedure::Goto(x, y));
        // if self.pen_on {
        //     self.canvas.line(self.x, self.y, x, y);
        // }

        // self.x = x;
        // self.y = y;
    }

    pub fn teleport(&mut self, x: f64, y: f64) {
        self.add_procedure(Procedure::Teleport(x, y));
        // self.x = x;
        // self.y = y;
    }

    pub fn home(&mut self) {
        self.add_procedure(Procedure::Home);
        // self.x = self.home_x;
        // self.y = self.home_y;
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
}

// using closure is worse
fn forward(canvas: &mut Canvas, x: f64, y: f64, heading: f64, step: f64) -> (f64, f64) {
    let (sr, cr) = heading.to_radians().sin_cos();
    let tx = x + cr * step;
    let ty = y + sr * step;
    canvas.line(x, y, tx, ty);
    (tx, ty)
}

impl Draw for Turtle {
    fn draw(&self, canvas: &mut Canvas, x: f64, y: f64) {
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
                    (x, y) = forward(canvas, x, y, heading, *step);
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
                        canvas.line(x, y, *tx, *ty);
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
                            2.0 * radius * (angle / 2.0).to_radians().sin(),
                        );
                        heading -= angle;
                    }
                }
            }
        }
    }
}
