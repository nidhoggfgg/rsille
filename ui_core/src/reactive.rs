use async_trait::async_trait;
use tokio::sync::watch;

use crate::{style::Stylized, traits::Draw, DrawErr, DrawUpdate, Update};

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
    pub fn watch(mut self, watcher: watch::Receiver<S>, func: F) -> Self {
        self.watchers.push(Watcher { watcher, func });
        self
    }
}

impl<T, S, F> Draw for Reactive<T, S, F>
where
    T: Draw,
    S: Clone + Send + Sync,
    F: FnMut(&mut T, &S) + 'static,
{
    fn draw(&self) -> Result<Vec<Stylized>, DrawErr> {
        self.component.draw()
    }

    fn size(&self) -> Option<(u32, u32)> {
        self.component.size()
    }
}

#[async_trait]
impl<T, S, F> Update for Reactive<T, S, F>
where
    T: Update,
    S: Clone + Send + Sync,
    F: FnMut(&mut T, &S) + Send + 'static,
{
    async fn update(&mut self) -> Result<bool, DrawErr> {
        self.component.update().await?;
        let mut changed = false;
        for watcher in self.watchers.iter_mut() {
            match watcher.watcher.changed().await {
                Ok(()) => {
                    (watcher.func)(&mut self.component, &watcher.watcher.borrow());
                    changed = true;
                }
                Err(_) => return Err(DrawErr),
            }
        }
        Ok(changed)
    }
}

impl<T, S, F> DrawUpdate for Reactive<T, S, F>
where
    T: DrawUpdate,
    S: Clone + Send + Sync,
    F: FnMut(&mut T, &S) + Send + 'static,
{
}

struct Watcher<S, F> {
    watcher: watch::Receiver<S>,
    func: F,
}
