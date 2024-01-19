use crate::Canvas;

#[derive(Debug, Clone, PartialEq)]
pub struct Turtle {
    home_x: f64,
    home_y: f64,
    x: f64,
    y: f64,
    heading: f64,
    pen_on: bool,
    canvas: Canvas,
}

impl Turtle {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            home_x: x,
            home_y: y,
            x,
            y,
            heading: 0.0,
            pen_on: true,
            canvas: Canvas::new(),
        }
    }

    pub fn frame(&mut self) -> String {
        self.canvas.frame()
    }

    pub fn get_lines(&mut self) -> Vec<String> {
        self.canvas.get_lines()
    }

    pub fn penup(&mut self) {
        self.pen_on = false;
    }

    pub fn up(&mut self) {
        self.penup();
    }

    pub fn pendown(&mut self) {
        self.pen_on = true;
    }

    pub fn down(&mut self) {
        self.pendown();
    }

    pub fn forward(&mut self, step: f64) {
        let (sr, cr) = self.heading.to_radians().sin_cos();
        let x = self.x + cr * step;
        let y = self.y + sr * step;
        let prev_brush = self.pen_on;
        self.pen_on = true;
        self.goto(x, y);
        self.pen_on = prev_brush;
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
        self.heading += angle;
    }

    pub fn rt(&mut self, angle: f64) {
        self.right(angle);
    }

    pub fn left(&mut self, angle: f64) {
        self.heading -= angle;
    }

    pub fn lt(&mut self, angle: f64) {
        self.left(angle);
    }

    pub fn circle(&mut self, raduis: f64, extent: f64, steps: usize) {
        let angle = extent / steps as f64;
        for _ in 0..steps {
            self.forward(2.0 * raduis * (angle / 2.0).to_radians().sin());
            self.left(angle);
        }
    }

    pub fn goto(&mut self, x: f64, y: f64) {
        if self.pen_on {
            self.canvas.line(self.x, self.y, x, y);
        }

        self.x = x;
        self.y = y;
    }

    pub fn teleport(&mut self, x: f64, y: f64) {
        self.x = x;
        self.y = y;
    }

    pub fn home(&mut self) {
        self.x = self.home_x;
        self.y = self.home_y;
    }

    pub fn position(&self) -> (f64, f64) {
        (self.x, self.y)
    }

    pub fn pos(&self) -> (f64, f64) {
        self.position()
    }

    pub fn heading(&self) -> f64 {
        self.heading
    }
}