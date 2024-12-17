use std::collections::HashMap;

use crate::style::Stylized;

pub struct DrawErr;

pub trait Draw: Send + 'static {
    fn draw(&self) -> Vec<Stylized>;
    fn size(&self) -> (u32, u32);
}

pub struct Panel {
    size: (u32, u32),
    boxes: HashMap<String, DrawBox>,
}

impl Panel {
    fn push<T>(&mut self, name: &str, thing: T, pos: (u32, u32))
    where
        T: Draw,
    {
        let draw_box = DrawBox {
            pos,
            z_index: 0,
            thing: Box::new(thing),
        };
        self.boxes.insert(name.to_owned(), draw_box);
    }
}

impl Draw for Panel {
    fn draw(&self) -> Vec<Stylized> {
        todo!()
    }

    fn size(&self) -> (u32, u32) {
        todo!()
    }
}

// wrapped for
struct DrawBox {
    pub pos: (u32, u32),
    pub z_index: u32,
    thing: Box<dyn Draw>,
}

impl Draw for DrawBox {
    fn draw(&self) -> Vec<Stylized> {
        self.thing.draw()
    }

    fn size(&self) -> (u32, u32) {
        self.thing.size()
    }
}
