use crate::Canvas;

#[rustfmt::skip]
pub struct Decor {
    pub lt: char, pub rt: char, pub rb: char, pub lb: char,
    pub te: char, pub le: char, pub be: char, pub re: char,
    pub tc: char, pub lc: char, pub bc: char, pub rc: char, pub cross: char,
    pub sh: char, pub sv: char,
}

#[rustfmt::skip]
impl Decor {
    pub fn new() -> Self {
        Decor {
            lt: '╭', rt: '╗',
            lb: '╰', rb: '╜',

                te: '═', 
            le: '│', re: '║',
                be: '─',

                        tc: '╤',
            lc: '├', cross: '┼', rc: '╢',
                        bc: '┴',

            sh: '┄', sv: '┄',
        }
    }

    pub fn get_corner(&self) -> (char, char, char, char) {
        (self.lt, self.rt, self.rb, self.lb)
    }

    pub fn get_edge(&self) -> (char, char, char, char) {
        (self.te, self.le, self.be, self.re)
    }

    pub fn get_cross(&self) -> (char, char, char, char, char) {
        (self.tc, self.rc, self.bc, self.lc, self.cross)
    }
}

pub(crate) fn draw_box(canvas: &mut Canvas, start: (f64, f64), end: (f64, f64), decor: &Decor) {
    let (lt, rt, rb, lb) = decor.get_corner();
    let (te, le, be, re) = decor.get_edge();
    let (start_x, start_y) = start;
    let (end_x, end_y) = end;
    canvas.line_any((start_x, start_y), (end_x, start_y), be, None);
    canvas.line_any((start_x, start_y), (start_x, end_y), le, None);
    canvas.line_any((end_x, end_y), (start_x, end_y), te, None);
    canvas.line_any((end_x, end_y), (end_x, start_y), re, None);
    canvas.put(start_x, start_y, lb, None);
    canvas.put(start_x, end_y, lt, None);
    canvas.put(end_x, start_y, rb, None);
    canvas.put(end_x, end_y, rt, None);
}

