use term::crossterm::event::{Event, KeyEvent, MouseEvent};

use crate::{Draw, Update};

pub type KeyHandler<T> = Box<dyn FnMut(&mut T, KeyEvent) + Send + Sync>;
pub type MouseHandler<T> = Box<dyn FnMut(&mut T, MouseEvent) + Send + Sync>;
pub type ResizeHandler<T> = Box<dyn FnMut(&mut T, (u16, u16)) + Send + Sync>;
pub type FocusHandler<T> = Box<dyn FnMut(&mut T, bool) + Send + Sync>;

pub struct Interactive<T> {
    component: T,
    key_handler: Option<KeyHandler<T>>,
    mouse_handler: Option<MouseHandler<T>>,
    resize_handler: Option<ResizeHandler<T>>,
    focus_handler: Option<FocusHandler<T>>,
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
            changed |= match event {
                Event::Key(key) => self
                    .key_handler
                    .as_mut()
                    .map(|h| h(&mut self.component, *key))
                    .is_some(),
                Event::Mouse(mouse) => self
                    .mouse_handler
                    .as_mut()
                    .map(|h| h(&mut self.component, *mouse))
                    .is_some(),
                Event::Resize(width, height) => self
                    .resize_handler
                    .as_mut()
                    .map(|h| h(&mut self.component, (*width, *height)))
                    .is_some(),
                Event::FocusGained => self
                    .focus_handler
                    .as_mut()
                    .map(|h| h(&mut self.component, true))
                    .is_some(),
                Event::FocusLost => self
                    .focus_handler
                    .as_mut()
                    .map(|h| h(&mut self.component, false))
                    .is_some(),
                _ => false,
            }
        }

        Ok(changed)
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
