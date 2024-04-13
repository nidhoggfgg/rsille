use std::iter::zip;

use crate::{Canvas, Paint};

pub trait Plotable {
    fn plot(&self) -> (Vec<f64>, Vec<f64>);
}

pub struct Figure {
    xs: Vec<f64>,
    ys: Vec<f64>,
    scale: (f64, f64),
    show_axis: bool,
    boxed: bool,
    padding: f64,
}

impl Figure {
    pub fn new() -> Self {
        Self {
            xs: Vec::new(),
            ys: Vec::new(),
            scale: (10.0, 10.0),
            show_axis: true,
            boxed: true,
            padding: 10.0,
        }
    }

    pub fn plot<P>(&mut self, p: &P)
    where P: Plotable {
        let (xs, ys) = p.plot();
        self.xs.extend(&xs);
        self.ys.extend(&ys);
    }
}

impl Paint for Figure {
    fn paint<T>(&self, canvas: &mut Canvas, x: T, y: T) -> Result<(), crate::utils::RsilleErr>
    where
        T: Into<f64>,
    {
        let (x, y) = (x.into(), y.into());
        let pad = self.padding;
        let (sx, sy) = self.scale;
        for (px, py) in zip(&self.xs, &self.ys) {
            canvas.set(x + px * sx, y + py * sy);
        }
        let minx = self.xs.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let maxx = self.xs.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let miny = self.ys.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let maxy = self.ys.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let (start_x, start_y) = (x + minx * sx - pad, y + miny * sy - pad);
        let (end_x, end_y) = (x + maxx * sx + pad, y + maxy * sy + pad);
        canvas.line_any((start_x, start_y), (end_x, start_y), '─', None);
        canvas.line_any((start_x, start_y), (start_x, end_y), '│', None);
        canvas.line_any((end_x, end_y), (start_x, end_y), '─', None);
        canvas.line_any((end_x, end_y), (end_x, start_y), '│', None);
        canvas.put(start_x, start_y, '└', None);
        canvas.put(start_x, end_y, '┌', None);
        canvas.put(end_x, start_y, '┘', None);
        canvas.put(end_x, end_y, '┐', None);
        Ok(())
    }
}
