use crate::Canvas;

#[derive(Debug, Clone, PartialEq)]
pub struct Turtle {
    x: f64,
    y: f64,
    rotation: f64,
    brush_on: bool,
    canvas: Canvas,
}

impl Turtle {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            rotation: 0.0,
            brush_on: true,
            canvas: Canvas::new(),
        }
    }

    pub fn frame(&mut self) -> String {
        self.canvas.frame()
    }

    pub fn get_lines(&mut self) -> Vec<String> {
        self.canvas.get_lines()
    }

    pub fn up(&mut self) {
        self.brush_on = false;
    }

    pub fn down(&mut self) {
        self.brush_on = true;
    }

    pub fn forward(&mut self, step: f64) {
        let (sr, cr) = self.rotation.to_radians().sin_cos();
        let x = self.x + cr * step;
        let y = self.y + sr * step;
        let prev_brush = self.brush_on;
        self.brush_on = true;
        self.move_to(x, y);
        self.brush_on = prev_brush;
    }

    pub fn move_to(&mut self, x: f64, y: f64) {
        if self.brush_on {
            self.canvas.line(self.x, self.y, x, y);
        }

        self.x = x;
        self.y = y;
    }

    pub fn go_to(&mut self, x: f64, y: f64) {
        self.x = x;
        self.y = y;
    }

    pub fn right(&mut self, angle: f64) {
        self.rotation += angle;
    }

    pub fn left(&mut self, angle: f64) {
        self.rotation -= angle;
    }

    pub fn back(&mut self, step: f64) {
        self.forward(-step)
    }
}