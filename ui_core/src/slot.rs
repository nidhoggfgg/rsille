use async_trait::async_trait;

use crate::{attr::Attr, style::Stylized, traits::Draw, DrawErr, DrawUpdate, Update};

pub struct Slot {
    pub attr: Attr,
    pub thing: Box<dyn DrawUpdate>,
}

impl Draw for Slot {
    fn draw(&self) -> Result<Vec<Stylized>, DrawErr> {
        self.thing.draw()
    }

    fn size(&self) -> Option<(u32, u32)> {
        self.thing.size()
    }
}

#[async_trait]
impl Update for Slot {
    async fn update(&mut self) -> Result<bool, DrawErr> {
        self.thing.update().await
    }
}
