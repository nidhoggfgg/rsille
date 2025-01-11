use term::crossterm::event::{Event, KeyEvent, MouseEvent};

use crate::{Draw, Update};

pub enum EventKind {
    Key,
    Mouse,
    Resize,
    Focus,
}

pub struct Interactive<T> {
    component: T,
    key_handler: Option<Box<dyn FnMut(&mut T, KeyEvent) + Send + Sync>>,
    mouse_handler: Option<Box<dyn FnMut(&mut T, MouseEvent) + Send + Sync>>,
    resize_handler: Option<Box<dyn FnMut(&mut T, (u16, u16)) + Send + Sync>>,
    focus_handler: Option<Box<dyn FnMut(&mut T, bool) + Send + Sync>>,
}

impl<T> Interactive<T> {
    pub fn new(component: T) -> Self {
        Self {
            component,
            key_handler: None,
            mouse_handler: None,
            resize_handler: None,
            focus_handler: None,
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

    pub fn register_resize_event<F>(&mut self, handler: F)
    where
        F: FnMut(&mut T, (u16, u16)) + Send + Sync + 'static,
    {
        self.resize_handler = Some(Box::new(handler));
    }

    pub fn register_focus_event<F>(&mut self, handler: F)
    where
        F: FnMut(&mut T, bool) + Send + Sync + 'static,
    {
        self.focus_handler = Some(Box::new(handler));
    }
}

impl<T> Update for Interactive<T>
where
    T: Update,
{
    fn update(&mut self, events: &[Event]) -> Result<bool, crate::DrawErr> {
        let mut changed = false;
        if self.component.update(events)? {
            changed = true;
        }

        for event in events {
            match event {
                Event::Key(key) => {
                    if let Some(handler) = &mut self.key_handler {
                        (handler)(&mut self.component, *key);
                    }
                }
                Event::Mouse(mouse) => {
                    if let Some(handler) = &mut self.mouse_handler {
                        (handler)(&mut self.component, *mouse);
                    }
                }
                _ => {}
            }
        }

        Ok(changed)
    }
}

impl<T> Draw for Interactive<T>
where
    T: Draw,
{
    fn draw(&self) -> Result<Vec<crate::style::Stylized>, crate::DrawErr> {
        self.component.draw()
    }

    fn size(&self) -> Option<(u32, u32)> {
        self.component.size()
    }
}
