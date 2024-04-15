use crate::Canvas;

/// The box outside the object
///
/// It include many `char` for making a **box**.
/// But it *not* include the functions for making box!
///
/// As shown in the figure below, the abbreviation is:  
/// `l`: left, `r`: right, `t`: top, `b`: bottom, `e`: edge, `c`: cross
///
/// ```text
/// lt    te   tc       rt
///   ┌────────┰────────┓          lt: ┌ rt: ┓
///   │        ┆        ┃          lb: ╰ rb: ╝
///   │        ┆sv      ┃
///   │        ┆        ┃          te: ─ re: ┃  sv: ┆
/// lc├╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┦rc ┄┄┄┄┄> be: ═ le: │  sh: ╌
///   │   cross┆   sh   ┃
/// le│        ┆        ┃re        tc: ┰ rc: ┦
///   │        ┆        ┃          bc: ┸ lc: ├  cross: ┼
///   ╰════════┸════════╝
/// lb        bc   be    rb
/// ```
///
/// In general, you don't need to manually set up your own.
/// Some useful constructor methods can directly generate specific styles of borders.
pub struct Decor {
    /// left top corner
    pub lt: char,
    /// right top corner
    pub rt: char,
    /// right top corner
    pub rb: char,
    /// left bottom corner
    pub lb: char,

    /// top edge
    pub te: char,
    /// right edge
    pub re: char,
    /// bottom edge
    pub be: char,
    /// left edge
    pub le: char,

    /// top cross
    pub tc: char,
    /// right cross
    pub rc: char,
    /// bottom cross
    pub bc: char,
    /// left cross
    pub lc: char,
    /// the cross
    pub cross: char,

    /// horizontal segmetation
    pub sh: char,
    /// vertical segmetation
    pub sv: char,
}

impl Decor {
    /// Return those corner char
    pub fn get_corner(&self) -> (char, char, char, char) {
        (self.lt, self.rt, self.rb, self.lb)
    }

    /// Return those edge char
    pub fn get_edge(&self) -> (char, char, char, char) {
        (self.te, self.le, self.be, self.re)
    }

    /// Return those cross char
    pub fn get_cross(&self) -> (char, char, char, char, char) {
        (self.tc, self.rc, self.bc, self.lc, self.cross)
    }
}

#[rustfmt::skip]
impl Decor {
    /// The regular style
    /// 
    /// ```text
    /// ┌───┬───┐
    /// │   │   │
    /// ├───┼───┤
    /// │   │   │
    /// └───┴───┘
    /// ```
    pub fn simple() -> Self {
        Decor {
            lt: '┌', rt: '┐',
            lb: '└', rb: '┘',

                te: '─', 
            le: '│', re: '│',
                be: '─',

                        tc: '┬',
            lc: '├', cross: '┼', rc: '┤',
                        bc: '┴',

            sh: '─', sv: '│',
        }
    }

    /// The bold style
    /// 
    /// ```text
    /// ┏━━━┳━━━┓
    /// ┃   ┃   ┃
    /// ┣━━━╋━━━┫
    /// ┃   ┃   ┃
    /// ┗━━━┻━━━┛
    /// ```
    pub fn bold() -> Self {
        Decor {
            lt: '┏', rt: '┓',
            lb: '┗', rb: '┛',

                te: '━', 
            le: '┃', re: '┃',
                be: '━',

                        tc: '┳',
            lc: '┣', cross: '╋', rc: '┫',
                        bc: '┻',

            sh: '━', sv: '┃',
        }
    }

    /// The style for plot
    /// 
    /// ```text
    /// ╭═══╤═══╗
    /// │   ╎   ║
    /// ├┄┄┄┼┄┄┄╢
    /// │   ╎   ║
    /// ╰───┴───╜
    /// ```
    pub fn plot() -> Self {
        Decor {
            lt: '╭', rt: '╗',
            lb: '╰', rb: '╜',

                te: '═', 
            le: '│', re: '║',
                be: '─',

                        tc: '╤',
            lc: '├', cross: '┼', rc: '╢',
                        bc: '┴',

            sh: '┄', sv: '╎',
        }
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
