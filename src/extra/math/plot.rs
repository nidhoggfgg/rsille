use crate::Paint;

pub struct Plot<F> {
    fny: F,
    step: f64,
    range: (f64, f64),
    scale: (f64, f64),
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
            panic!("")
        }
        Self {
            fny: f,
            range: (start, end),
            step: 0.01,
            scale: (10.0, 10.0),
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
        let (mut px, end) = self.range;
        let (sx, sy) = self.scale;
        loop {
            if px > end {
                break;
            }
            let py = (self.fny)(px);
            canvas.set(x + px * sx, y + py * sy);
            px += self.step;
        }
        Ok(())
    }
}
