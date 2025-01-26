use term::event::Event;

use crate::{style::Stylized, Draw, DrawErr, Update};

#[derive(Clone)]
pub struct Animative<T, F> {
    component: T,
    anime_fn: F,
}

impl<T, F> Animative<T, F> {
    pub fn new(component: T, anime_fn: F) -> Self {
        Self {
            component,
            anime_fn,
        }
    }
}

impl<T, F> Draw for Animative<T, F>
where
    T: Draw,
    F: Fn(&mut T) + 'static,
{
    fn draw(&mut self) -> Result<Vec<Stylized>, DrawErr> {
        self.component.draw()
    }

    fn size(&self) -> Option<(u16, u16)> {
        self.component.size()
    }
}

impl<T, F> Update for Animative<T, F>
where
    T: Update,
    F: Fn(&mut T) + Send + Sync + 'static,
{
    fn update(&mut self) -> Result<bool, DrawErr> {
        (self.anime_fn)(&mut self.component);
        self.component.update()
    }

    fn on_events(&mut self, events: &[Event]) -> Result<(), DrawErr> {
        self.component.on_events(events)
    }
}
