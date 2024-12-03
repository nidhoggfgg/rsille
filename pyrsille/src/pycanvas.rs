use canvas::canvas;
use pyo3::prelude::*;

#[pyclass]
pub struct Canvas {
    canvas: canvas::Canvas,
}

#[pymethods]
impl Canvas {
    #[new]
    fn new() -> Self {
        Self {
            canvas: canvas::Canvas::new(),
        }
    }

    fn set(&mut self, x: f64, y: f64) {
        self.canvas.set(x, y);
    }

    fn line(&mut self, xy1: (f64, f64), xy2: (f64, f64)) {
        self.canvas.line(xy1, xy2);
    }

    fn print(&mut self) {
        self.canvas.print();
    }
}
