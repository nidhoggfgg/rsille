use render::{Draw, DrawChunk, DrawErr, Update};
use term::event::Event;
use tokio::sync::watch;

use crate::Widget;

#[derive(Clone)]
pub struct Reactive<T, S, F> {
    component: T,
    watchers: Vec<Watcher<S, F>>,
}

impl<T, S, F> Reactive<T, S, F>
where
    T: Draw,
    S: Clone + Send + Sync,
    F: FnMut(&mut T, &S),
{
    #[must_use]
    #[inline]
    pub fn new(component: T) -> Self {
        Self {
            component,
            watchers: vec![],
        }
    }

    #[must_use]
    #[inline]
    pub fn watch(&mut self, value: S, func: F) -> watch::Sender<S> {
        let (sender, receiver) = watch::channel(value);
        self.watchers.push(Watcher {
            _sender: sender.clone(),
            receiver,
            func,
        });
        sender
    }
}

impl<T, S, F> Draw for Reactive<T, S, F>
where
    T: Draw,
    S: Clone + Send + Sync,
    F: FnMut(&mut T, &S) + 'static,
{
    fn draw(&mut self) -> Result<DrawChunk, DrawErr> {
        self.component.draw()
    }
}

impl<T, S, F> Update for Reactive<T, S, F>
where
    T: Update,
    S: Clone + Send + Sync,
    F: FnMut(&mut T, &S) + Send + 'static,
{
    fn on_events(&mut self, events: &[Event]) -> Result<(), DrawErr> {
        self.component.on_events(events)
    }

    fn update(&mut self) -> Result<bool, DrawErr> {
        self.component.update()?;
        let mut changed = false;
        for watcher in self.watchers.iter_mut() {
            match watcher.receiver.has_changed() {
                Ok(true) => {
                    (watcher.func)(&mut self.component, &watcher.receiver.borrow());
                    changed = true;
                }
                Ok(false) => {}
                Err(_) => return Err(DrawErr),
            }
        }
        Ok(changed)
    }
}

impl<T, S, F> Widget for Reactive<T, S, F>
where
    T: Widget,
    S: Clone + Send + Sync,
    F: FnMut(&mut T, &S) + Send + 'static,
{
    fn get_attr(&self) -> &crate::attr::Attr {
        self.component.get_attr()
    }

    fn set_attr(&mut self, attr: crate::attr::Attr) {
        self.component.set_attr(attr);
    }
}

// for hold on to the sender and receiver
#[derive(Clone)]
pub struct Watcher<S, F> {
    _sender: watch::Sender<S>,
    receiver: watch::Receiver<S>,
    func: F,
}
