use std::iter::zip;

use crate::Paint;

pub struct Plot<F> {
    fny: F,
    step: f64,
    range: (f64, f64),
    scale: (f64, f64),
    show_axis: bool,
    boxed: bool,
    padding: f64,
}

impl<F> Plot<F>
where
    F: Fn(f64) -> f64,
{
    pub fn new<T>(f: F, range: (T, T)) -> Self
    where
        T: Into<f64>,
    {
        let (start, end) = (range.0.into(), range.1.into());
        if start > end {
            panic!("The start must greater than end")
        }
        Self {
            fny: f,
            range: (start, end),
            step: 0.01,
            scale: (10.0, 10.0),
            show_axis: true,
            boxed: true,
            padding: 10.0
        }
    }
}

impl<F> Paint for Plot<F>
where
    F: Fn(f64) -> f64 + Send + 'static,
{
    fn paint<T>(
        &self,
        canvas: &mut crate::Canvas,
        x: T,
        y: T,
    ) -> Result<(), crate::utils::RsilleErr>
    where
        T: Into<f64>,
    {
        let (x, y) = (x.into(), y.into());
        let (start, end) = self.range;
        let mut px = start;
        let (sx, sy) = self.scale;
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        // the lost of f64 isn't a big deal, loop 101 or 100 times won't be a problem
        loop {
            if px > end {
                break;
            }
            let py = (self.fny)(px);
            xs.push(px);
            ys.push(py);
            px += self.step;
        }
        for (px, py) in zip(&xs, &ys) {
            canvas.set(x + px * sx, y + py * sy)
        }
        if self.show_axis {
            let pad = self.padding;
            let miny = ys.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let maxy = ys.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let (start_x, start_y) = (x + start * sx - pad, y + miny * sy - pad);
            let (end_x, end_y) = (x + end * sx + pad, y + maxy * sy + pad);
            canvas.line_any((start_x, start_y), (end_x, start_y), '─', None);
            canvas.line_any((start_x, start_y), (start_x, end_y), '│', None);
            canvas.line_any((end_x, end_y), (start_x, end_y), '─', None);
            canvas.line_any((end_x, end_y), (end_x, start_y), '│', None);
            canvas.put(start_x, start_y, '└', None);
            canvas.put(start_x, end_y, '┌', None);
            canvas.put(end_x, start_y, '┘', None);
            canvas.put(end_x, end_y, '┐', None);
        }
        Ok(())
    }
}
