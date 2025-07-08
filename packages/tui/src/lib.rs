use render::DrawUpdate;

pub mod attr;
pub mod composite;
pub mod widgets;

pub trait Widget: DrawUpdate {
    fn get_attr(&self) -> &attr::Attr;
    fn set_attr(&mut self, attr: attr::SetAttr);
}
