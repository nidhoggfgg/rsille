use render::{
    area::{Area, Position, Size},
    chunk::Chunk,
    Draw, DrawErr, Update,
};
use term::event::Event;

use crate::{
    attr::{Attr, SetAttr},
    border::Border,
    Widget,
};

#[allow(unused)]
pub struct Grid<const M: usize, const N: usize> {
    children: [[Box<dyn Widget + Send + Sync>; M]; N],
    border: Border,
    has_border: bool,
    attr: Attr,
}

impl<const M: usize, const N: usize> Grid<M, N> {
    pub fn new(children: [[Box<dyn Widget + Send + Sync>; M]; N], border: Border) -> Self {
        Self {
            children,
            border,
            has_border: true,
            attr: Attr::default(),
        }
    }

    pub fn without_border(children: [[Box<dyn Widget + Send + Sync>; M]; N]) -> Self {
        Self {
            children,
            border: Border::default(),
            has_border: false,
            attr: Attr::default(),
        }
    }

    pub fn with_simple_border(children: [[Box<dyn Widget + Send + Sync>; M]; N]) -> Self {
        Self {
            children,
            border: Border::simple(),
            has_border: true,
            attr: Attr::default(),
        }
    }
}

impl<const M: usize, const N: usize> Draw for Grid<M, N> {
    fn draw(&mut self, mut chunk: Chunk) -> Result<(), DrawErr> {
        let (m, n) = (M as u16, N as u16);

        if m == 0 || n == 0 {
            return Ok(());
        }

        let mut size = chunk.area().size();

        if self.has_border {
            if size.width < m + 1 || size.height < n + 1 {
                return Ok(());
            }

            size.width -= m + 1;
            size.height -= n + 1;
        }

        if size.width < m || size.height < n {
            return Ok(());
        }

        let (widths, heights) = size.split_mxn(m, n);

        let mut offset = Position::default();
        if self.has_border {
            offset.right(1);
            offset.down(1);
        }

        for (i, height) in heights.iter().enumerate() {
            for (j, width) in widths.iter().enumerate() {
                let pos = chunk.area().pos() + offset;
                let area = Area::new(pos, (*width, *height).into());
                self.children[i][j].draw(chunk.from_area(area)?)?;

                offset.right(*width);
                if self.has_border {
                    offset.right(1);
                }
            }
            offset.reset_x();
            offset.down(*height);
            if self.has_border {
                offset.down(1);
                offset.right(1);
            }
        }

        self.border.draw(chunk, &widths, &heights)?;

        Ok(())
    }
}

impl<const N: usize, const M: usize> Update for Grid<N, M> {
    fn on_events(&mut self, _events: &[Event]) -> Result<(), DrawErr> {
        todo!()
    }

    fn update(&mut self) -> Result<bool, DrawErr> {
        todo!()
    }
}

impl<const N: usize, const M: usize> Widget for Grid<N, M> {
    fn get_attr(&self) -> &Attr {
        &self.attr
    }

    fn set_attr(&mut self, attr: SetAttr) {
        self.attr.set(attr);
    }

    fn size(&self) -> Size {
        todo!()
    }
}
