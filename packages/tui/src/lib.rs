use render::{area::Size, DrawUpdate};

use crate::dom::Document;

pub mod attr;
pub mod border;
// pub mod composite;
pub mod dom;
pub mod layout;
pub mod widgets;

pub trait Widget: DrawUpdate {
    fn get_attr(&self) -> &attr::Attr;
    fn set_attr(&mut self, attr: attr::SetAttr);
    fn size(&self) -> Size;
    fn id(&self) -> String;

    fn register_widget(self, doc: &mut Document)
    where
        Self: Sized + 'static,
    {
        doc.add_element(self);
    }
}
