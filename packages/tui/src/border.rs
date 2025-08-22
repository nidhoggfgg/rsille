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
pub struct Border {
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

impl Border {
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
impl Border {
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
        Border {
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
        Border {
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
        Border {
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
