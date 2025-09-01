use render::area::Size;
use render::chunk::Chunk;
use render::{Draw, DrawErr};

use crate::{node::Node, Element};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

#[derive(Debug, Clone)]
pub struct Document {
    root: Option<Node>,
    // title: String,
    elements: HashMap<String, Rc<RefCell<Element>>>,
}

impl Document {
    pub fn new() -> Self {
        Self {
            root: None,
            // title: String::new(),
            elements: HashMap::new(),
        }
    }

    pub fn set_root(&mut self, root: Element) {
        self.root = Some(Node::Element(Rc::new(RefCell::new(root))));
    }

    pub fn register(&mut self, id: String, element: Rc<RefCell<Element>>) {
        self.elements.insert(id, element);
    }

    pub fn get_element_by_id(&self, id: &str) -> Option<Weak<RefCell<Element>>> {
        if let Some(elm) = self.elements.get(id) {
            return Some(Rc::downgrade(elm));
        }

        None
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Draw for Document {
    fn draw(&mut self, chunk: Chunk) -> Result<Size, DrawErr> {
        if let Some(root) = self.root.as_mut() {
            root.draw(chunk)
        } else {
            Err(DrawErr)
        }
    }
}
