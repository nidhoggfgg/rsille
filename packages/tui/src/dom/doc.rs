use std::{cell::RefCell, collections::HashMap, rc::Rc};

use render::{Draw, DrawErr, Update};

use crate::{dom::Node, Widget};

pub struct Document {
    root: Option<Rc<RefCell<dyn Node>>>,
    nodes: HashMap<String, Rc<RefCell<dyn Node>>>,
}

impl Document {
    pub fn new() -> Self {
        Self {
            root: None,
            nodes: HashMap::new(),
        }
    }

    pub fn get_node_by_id(&self, id: &str) -> Option<Rc<RefCell<dyn Node>>> {
        self.nodes.get(id).cloned()
    }

    pub fn add_element<T>(&mut self, element: T)
    where
        T: Widget + 'static,
    {
        let node = Rc::new(RefCell::new(element));
        let id = node.borrow().id();
        if self.root.is_none() {
            self.root = Some(node.clone());
        }
        self.nodes.insert(id, node);
    }

    pub fn with_node_as<T, F, R>(&self, id: &str, f: F) -> Result<R, DrawErr>
    where
        T: 'static,
        F: FnOnce(&mut T) -> Result<R, DrawErr>,
    {
        let node = self.get_node_by_id(id).ok_or(DrawErr)?;

        let mut bindging = node.borrow_mut();
        let node_any = bindging.as_any_node_mut();

        let target_node = node_any.downcast_mut::<T>().ok_or(DrawErr)?;

        f(target_node)
    }
}

impl Draw for Document {
    fn draw(&mut self, chunk: render::chunk::Chunk) -> Result<(), DrawErr> {
        if let Some(root) = &self.root {
            root.borrow_mut().draw(chunk)
        } else {
            Ok(())
        }
    }
}

impl Update for Document {
    fn on_events(&mut self, events: &[term::event::Event]) -> Result<(), DrawErr> {
        if let Some(root) = &self.root {
            root.borrow_mut().on_events(events)
        } else {
            Ok(())
        }
    }

    fn update(&mut self) -> Result<bool, DrawErr> {
        if let Some(root) = &self.root {
            root.borrow_mut().update()
        } else {
            Ok(false)
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}
