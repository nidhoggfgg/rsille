use render::{chunk::Chunk, style::Stylized, DrawErr};

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
#[derive(Debug, Clone, PartialEq, Eq, Default, Copy, Hash, PartialOrd, Ord)]
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

impl Border {
    pub fn draw(&self, mut chunk: Chunk, widths: &[u16], heights: &[u16]) -> Result<(), DrawErr> {
        if widths.is_empty() || heights.is_empty() {
            return Ok(());
        }

        let chunk_size = chunk.area().size();

        let total_width = widths.iter().sum::<u16>() + widths.len() as u16 + 1;
        let total_height = heights.iter().sum::<u16>() + heights.len() as u16 + 1;

        if total_width > chunk_size.width || total_height > chunk_size.height {
            return Err(DrawErr);
        }

        chunk.set_forced(0, 0, Stylized::raw(self.lt))?;
        chunk.set_forced(total_width - 1, 0, Stylized::raw(self.rt))?;
        chunk.set_forced(0, total_height - 1, Stylized::raw(self.lb))?;
        chunk.set_forced(total_width - 1, total_height - 1, Stylized::raw(self.rb))?;

        let mut current_x = 0;
        for &width in widths {
            for x in 1..=width {
                chunk.set_forced(current_x + x, 0, Stylized::raw(self.te))?;
                chunk.set_forced(current_x + x, total_height - 1, Stylized::raw(self.be))?;
            }
            if current_x > 0 {
                chunk.set_forced(current_x, 0, Stylized::raw(self.tc))?;
                chunk.set_forced(current_x, total_height - 1, Stylized::raw(self.bc))?;
            }

            current_x += width + 1;
        }

        let mut current_x = 0;
        let mut current_y = 0;
        for &height in heights {
            for &width in widths {
                if current_y > 0 {
                    if current_x == 0 {
                        chunk.set_forced(current_x, current_y, Stylized::raw(self.lc))?;
                    } else {
                        chunk.set_forced(current_x, current_y, Stylized::raw(self.cross))?;
                    }
                    for x in 1..=width {
                        chunk.set_forced(current_x + x, current_y, Stylized::raw(self.sh))?;
                    }
                }
                for y in 1..=height {
                    if current_x == 0 {
                        chunk.set_forced(current_x, current_y + y, Stylized::raw(self.le))?;
                    } else {
                        chunk.set_forced(current_x, current_y + y, Stylized::raw(self.sv))?;
                    }
                }
                current_x += width + 1;
            }

            for y in 1..=height {
                chunk.set_forced(current_x, current_y + y, Stylized::raw(self.re))?;
            }
            if current_y > 0 {
                chunk.set_forced(current_x, current_y, Stylized::raw(self.rc))?;
            }

            current_x = 0;
            current_y += height + 1;
        }

        Ok(())
    }
}
