use term::event::{Event, KeyEvent, MouseEvent};

use crate::{Draw, Update};

pub type KeyHandler<T> = Box<dyn FnMut(&mut T, KeyEvent) + Send + Sync>;
pub type MouseHandler<T> = Box<dyn FnMut(&mut T, MouseEvent) + Send + Sync>;

pub struct Interactive<T> {
    component: T,
    key_handler: Option<KeyHandler<T>>,
    mouse_handler: Option<MouseHandler<T>>,
}

impl<T> Interactive<T> {
    pub fn new(component: T) -> Self {
        Self {
            component,
            key_handler: None,
            mouse_handler: None,
        }
    }

    pub fn register_key_event<F>(&mut self, handler: F)
    where
        F: FnMut(&mut T, KeyEvent) + Send + Sync + 'static,
    {
        self.key_handler = Some(Box::new(handler));
    }

    pub fn register_mouse_event<F>(&mut self, handler: F)
    where
        F: FnMut(&mut T, MouseEvent) + Send + Sync + 'static,
    {
        self.mouse_handler = Some(Box::new(handler));
    }
}

impl<T> Update for Interactive<T>
where
    T: Update,
{
    fn on_events(&mut self, events: &[Event]) -> Result<(), crate::DrawErr> {
        self.component.on_events(events)
    }

    fn update(&mut self) -> Result<bool, crate::DrawErr> {
        self.component.update()
    }
}

impl<T> Draw for Interactive<T>
where
    T: Draw,
{
    fn draw(&mut self) -> Result<Vec<crate::style::Stylized>, crate::DrawErr> {
        self.component.draw()
    }

    fn size(&self) -> Option<(u16, u16)> {
        self.component.size()
    }
}
