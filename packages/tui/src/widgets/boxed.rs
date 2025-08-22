use render::{chunk::Chunk, Draw, DrawErr};

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
    fn draw(&mut self, _chunk: Chunk) -> Result<(), DrawErr> {
        todo!()
    }
}
