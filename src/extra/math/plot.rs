use std::iter::zip;

use super::figure::Plotable;

#[derive(Debug)]
pub struct Plot<F> {
    fny: F,
    step: f64,
    range: (f64, f64),
}

impl<F> Plot<F>
where
    F: Fn(f64) -> f64,
{
    pub fn new<T>(f: F, range: (T, T)) -> Self
    where
        T: Into<f64>,
    {
        let range = (range.0.into(), range.1.into());
        Self {
            fny: f,
            step: 0.08,
            range,
        }
    }

    pub fn set_step(&mut self, step: f64) {
        self.step = step;
    }
}

impl<F> Plotable for Plot<F> where F: Fn(f64) -> f64 {
    fn plot(&self) -> (Vec<f64>, Vec<f64>) {
        let mut x = self.range.0;
        let end = self.range.1;
        let (mut xs, mut ys) = (Vec::new(), Vec::new());
        loop {
            if x > end && (x - end).abs() > 0.01 {
                break;
            }
            xs.push(x);
            ys.push((self.fny)(x));
            x += self.step;
        }
        (xs, ys)
    }
}
