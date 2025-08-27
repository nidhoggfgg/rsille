use std::any::Any;

use crate::Widget;

pub trait Node: Widget {
    fn as_any_node(&self) -> &dyn Any;
    fn as_any_node_mut(&mut self) -> &mut dyn Any;
}

impl<T: Widget + 'static> Node for T {
    fn as_any_node(&self) -> &dyn Any {
        self
    }
    fn as_any_node_mut(&mut self) -> &mut dyn Any {
        self
    }
}
