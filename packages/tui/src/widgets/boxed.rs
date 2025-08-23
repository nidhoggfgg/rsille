use render::{chunk::Chunk, style::Stylized, Draw, DrawErr};

use crate::border::Border;

#[allow(unused)]
pub struct Boxed<T> {
    inner: T,
    border: Border,
}

impl<T> Boxed<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            border: Border::simple(),
        }
    }

    pub fn set_border(&mut self, border: Border) {
        self.border = border;
    }

    pub fn get_border(&self) -> &Border {
        &self.border
    }
}

impl<T> Draw for Boxed<T>
where
    T: Draw,
{
    fn draw(&mut self, mut chunk: Chunk) -> Result<(), DrawErr> {
        let inner_chunk = chunk.shrink(1, 1, 1, 1)?;
        self.inner.draw(inner_chunk)?;

        let area = chunk.area();
        let size = area.size;

        chunk.set(0, 0, Stylized::raw(self.border.lt))?;
        chunk.set(size.width - 1, 0, Stylized::raw(self.border.rt))?;
        chunk.set(0, size.height - 1, Stylized::raw(self.border.lb))?;
        chunk.set(size.width - 1, size.height - 1, Stylized::raw(self.border.rb))?;

        for x in 1..size.width - 1 {
            chunk.set(x, 0, Stylized::raw(self.border.te))?;
            chunk.set(x, size.height - 1, Stylized::raw(self.border.be))?;
        }
        for y in 1..size.height - 1 {
            chunk.set(0, y, Stylized::raw(self.border.le))?;
            chunk.set(size.width - 1, y, Stylized::raw(self.border.re))?;
        }

        Ok(())
    }
}
