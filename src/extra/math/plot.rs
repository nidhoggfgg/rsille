use crate::Paint;

pub struct Plot<F> {
    fny: F,
    step: f64,
    range: (f64, f64),
    scale: (f64, f64),
    show_axis: bool,
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
            show_axis: true
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
        let (mut minx, mut maxx) = (px, end);
        let (mut miny, mut maxy) = ((self.fny)(px), (self.fny)(px));
        loop {
            if px > end {
                break;
            }
            let py = (self.fny)(px);
            canvas.set(x + px * sx, y + py * sy);
            px += self.step;
            if minx > px {
                minx = px;
            }
            if maxx < px {
                maxx = px;
            }
            if miny > py {
                miny = py;
            }
            if maxy < py {
                maxy = py;
            }
        }
        if self.show_axis {
            canvas.line((x + minx * sx, 0.0), (x + maxx * sx, 0.0));
            canvas.line((0.0, y + miny * sy), (0.0, y + maxy * sy));
        }
        Ok(())
    }
}
