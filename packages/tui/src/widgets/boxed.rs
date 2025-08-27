use render::{area::Size, chunk::Chunk, style::Stylized, Draw, DrawErr, Update};

use crate::{
    attr::{Attr, SetAttr},
    border::Border,
    Widget,
};

#[allow(unused)]
pub struct Boxed<T> {
    inner: T,
    border: Border,
    attr: Attr,
}

impl<T> Boxed<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            border: Border::simple(),
            attr: Attr::default(),
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
        let size = area.size();

        chunk.set_forced(0, 0, Stylized::raw(self.border.lt))?;
        chunk.set_forced(size.width - 1, 0, Stylized::raw(self.border.rt))?;
        chunk.set_forced(0, size.height - 1, Stylized::raw(self.border.lb))?;
        chunk.set_forced(
            size.width - 1,
            size.height - 1,
            Stylized::raw(self.border.rb),
        )?;

        for x in 1..size.width - 1 {
            chunk.set_forced(x, 0, Stylized::raw(self.border.te))?;
            chunk.set_forced(x, size.height - 1, Stylized::raw(self.border.be))?;
        }
        for y in 1..size.height - 1 {
            chunk.set_forced(0, y, Stylized::raw(self.border.le))?;
            chunk.set_forced(size.width - 1, y, Stylized::raw(self.border.re))?;
        }

        Ok(())
    }
}

impl<T> Update for Boxed<T>
where
    T: Update,
{
    fn on_events(&mut self, events: &[term::event::Event]) -> Result<(), DrawErr> {
        self.inner.on_events(events)
    }

    fn update(&mut self) -> Result<bool, DrawErr> {
        self.inner.update()
    }
}

impl<T> Widget for Boxed<T>
where
    T: Widget,
{
    fn set_attr(&mut self, attr: SetAttr) {
        self.attr.set(attr);
    }

    fn get_attr(&self) -> &Attr {
        &self.attr
    }

    fn size(&self) -> Size {
        self.inner.size()
    }

    fn id(&self) -> String {
        self.attr.id.clone()
    }
}
