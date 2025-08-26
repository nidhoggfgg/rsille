use render::{area::Size, DrawUpdate};

pub mod attr;
pub mod border;
pub mod composite;
pub mod layout;
pub mod widgets;

pub trait Widget: DrawUpdate {
    fn get_attr(&self) -> &attr::Attr;
    fn set_attr(&mut self, attr: attr::SetAttr);
    fn size(&self) -> Size;
}
