use std::iter::zip;

use crate::{
    decor::{draw_box, Decor},
    utils::MIN_DIFFERENCE,
    Canvas, Paint,
};

/// A help macro for plot functions easy
///
/// example:
/// ```
/// use rsille::figure;
/// figure!((|x| x.sin(), (0, 5)), (|x| x.cos(), (0, 5)));
/// ```
#[macro_export]
macro_rules! figure {
    ($(($f: expr, ($start:expr, $end:expr))),*) => {
        use rsille::{extra::math::{Figure, Plot}, Canvas};
        let mut canvas = Canvas::new();
        let mut fig = Figure::new();
        $(
            let f: fn(f64) -> f64 = $f;
            let plot = Plot::new($f, ($start, $end));
            fig.plot(&plot);
        )*
        canvas.paint(&fig, 0, 0);
        canvas.print();
    };
}

/// Impl this for plot on [`Figure`](struct.Figure.html)
pub trait Plotable {
    /// Plot on the figure
    fn plot(&self) -> (Vec<f64>, Vec<f64>);
}

/// The figure
///
/// It's a figure can paint many functions on it.
/// All the math function all automatically draw on the right place.
///
/// ## Example
///
/// Draw the `y=sin(x)` and `y=cos(x)`
/// ```
/// use rsille::{extra::math::{Figure, Plot}, Canvas};
///
/// let mut canvas = Canvas::new();
/// let mut figure = Figure::new();
/// let p1 = Plot::new(|x| x.sin(), (0, 10));
/// let p2 = Plot::new(|x| x.cos(), (0, 10));
/// figure.plot(&p1);
/// figure.plot(&p2);
/// canvas.paint(&figure, 0, 0).unwrap();
/// canvas.print();
/// ```
pub struct Figure {
    xs: Vec<f64>,
    ys: Vec<f64>,
    scale: (f64, f64),
    show_axis: bool,
    boxed: bool,
    padding: f64,
    decor: Decor,
}

impl Figure {
    /// Make a new figure
    pub fn new() -> Self {
        Self {
            xs: Vec::new(),
            ys: Vec::new(),
            scale: (12.0, 12.0),
            show_axis: true,
            boxed: true,
            padding: 10.0,
            decor: Decor::plot(),
        }
    }

    pub fn with_scale(mut self, x: f64, y: f64) -> Self {
        self.scale = (x, y);
        self
    }

    pub fn clear(&mut self) {
        self.xs = Vec::new();
        self.ys = Vec::new();
    }

    /// Plot a thing
    ///
    /// It can plot something impl [`Plotable`](trait.Plotable.html)
    pub fn plot<P>(&mut self, p: &P)
    where
        P: Plotable,
    {
        let (xs, ys) = p.plot();
        self.xs.extend(&xs);
        self.ys.extend(&ys);
    }
}

impl Paint for Figure {
    fn paint<T>(&self, canvas: &mut Canvas, x: T, y: T)
    where
        T: Into<f64>,
    {
        let minx = self.xs.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let miny = self.ys.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let (x, y) = (x.into(), y.into());
        let pad = self.padding;
        let (sx, sy) = self.scale;
        for (px, py) in zip(&self.xs, &self.ys) {
            canvas.set(x + (px - minx) * sx, y + (py - miny) * sy);
        }
        if self.boxed || self.show_axis {
            let maxx = self.xs.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let maxy = self.ys.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let start = (x - pad, y - pad);
            let end = (x + (maxx - minx) * sx + pad, y + (maxy - miny) * sy + pad);
            if self.boxed {
                draw_box(canvas, start, end, &self.decor);
            }

            if self.show_axis {
                let (lc, bc) = (self.decor.lc, self.decor.bc);
                let (sx, sy) = ((sx / 2.0).floor() * 2.0, (sy / 4.0).floor() * 4.0);
                let xlabels = gen_label((minx * sx, maxx * sx), sx, minx);
                for (raw, label) in xlabels {
                    let (px, _) = floor(raw);
                    canvas.put(x + label, start.1, bc, None);
                    canvas.put_text(x + label, start.1 - 4.0, &px, None);
                }
                let ylabels = gen_label((miny * sy, maxy * sy), sy, miny);
                for (raw, label) in ylabels {
                    let (py, l) = floor(raw);
                    canvas.put(x + start.0, y + label, lc, None);
                    canvas.put_text(x + start.0 - 2.0 * l as f64, y + label, &py, None);
                }
            }
        }
    }
}

fn gen_label(range: (f64, f64), step: f64, v: f64) -> Vec<(f64, f64)> {
    let mut labels = Vec::new();
    let mut v = v;
    let (mut label, max) = (range.0, range.1);
    let mut p = 0.0;
    loop {
        if label > max && (label - max).abs() > 0.01 {
            break;
        }
        if label.abs() - 0.0 < MIN_DIFFERENCE {
            label = 0.0;
        }
        if p == 0.0 && label > 0.0 {
            p = 1.0;
        }
        labels.push((v, label - range.0 + p));
        v += 1.0;
        label += step;
    }
    labels
}

fn floor(v: f64) -> (String, usize) {
    let a = format!("{v:.2}");
    let len = a.len();
    (a, len)
}
