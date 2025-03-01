use render::{style::Stylized, Draw, DrawChunk, DrawErr, Update};
use term::event::Event;

use crate::{attr::Attr, widgets::Widget};

#[derive(Clone)]
pub struct Animative<T, F> {
    component: T,
    anime_fn: F,
}

impl<T, F> Animative<T, F>
where
    F: Fn(&mut T) + 'static,
{
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
    fn draw(&mut self) -> Result<DrawChunk, DrawErr> {
        self.component.draw()
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

impl<T, F> Widget for Animative<T, F>
where
    T: Widget,
    F: Fn(&mut T) + Send + Sync + 'static,
{
    fn get_attr(&self) -> &Attr {
        self.component.get_attr()
    }

    fn set_attr(&mut self, attr: Attr) {
        self.component.set_attr(attr);
    }

    fn show(&mut self) -> Result<Vec<Stylized>, DrawErr> {
        self.component.show()
    }
}
