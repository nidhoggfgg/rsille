use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use render::{Draw, DrawChunk, DrawErr, Update};
use term::event::Event;

use crate::{draw_boxes, Widget};

use super::slot::Slot;

pub struct Panel {
    pub boxes: Vec<Slot>,
    pub size: (u16, u16),
    // internal cache
    cache: Vec<Option<DrawChunk>>,
}

impl Panel {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            size: (width, height),
            boxes: Vec::new(),
            cache: Vec::new(),
        }
    }

    pub fn push<T>(&mut self, thing: T)
    where
        T: Widget + Send + Sync + 'static,
    {
        let slot = Slot {
            thing: Box::new(thing),
            updated: false,
        };
        self.boxes.push(slot);
        self.cache.push(None);
    }

    pub fn refresh_cache(&mut self) -> Result<(), DrawErr> {
        let cache = self
            .boxes
            .par_iter_mut()
            .enumerate()
            .map(|(i, b)| {
                if b.updated || self.cache[i].is_none() {
                    (i, Some(b.draw()))
                } else {
                    (i, None)
                }
            })
            .collect::<Vec<_>>();
        for (i, d) in cache {
            if let Some(d) = d {
                let d = d?;
                self.cache[i] = Some(d);
            }
        }
        Ok(())
    }
}

impl Draw for Panel {
    fn draw(&mut self) -> Result<render::DrawChunk, render::DrawErr> {
        let mut boxes = Vec::new();
        for (i, cache) in self.cache.iter().enumerate() {
            if let Some(cache) = cache {
                boxes.push((cache, self.boxes[i].get_attr()));
            }
        }
        draw_boxes(&boxes, self.size)
    }
}

impl Update for Panel {
    fn on_events(&mut self, _events: &[Event]) -> Result<(), DrawErr> {
        // dispatch event
        // todo!()
        Ok(())
    }

    fn update(&mut self) -> Result<bool, DrawErr> {
        let changes = self
            .boxes
            .par_iter_mut()
            .map(|b| b.update())
            .collect::<Result<Vec<_>, DrawErr>>()?;

        let changed = changes.iter().any(|x| *x);

        Ok(changed)
    }
}
