use term::crossterm::event::Event;
use tokio::sync::watch;

use crate::{style::Stylized, traits::Draw, DrawErr, Update};

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
    fn draw(&mut self) -> Result<Vec<Stylized>, DrawErr> {
        self.component.draw()
    }

    fn size(&self) -> Option<(u16, u16)> {
        self.component.size()
    }
}

impl<T, S, F> Update for Reactive<T, S, F>
where
    T: Update,
    S: Clone + Send + Sync,
    F: FnMut(&mut T, &S) + Send + 'static,
{
    fn update(&mut self, events: &[Event]) -> Result<bool, DrawErr> {
        self.component.update(events)?;
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

// for hold on to the sender and receiver
#[derive(Clone)]
struct Watcher<S, F> {
    _sender: watch::Sender<S>,
    receiver: watch::Receiver<S>,
    func: F,
}
