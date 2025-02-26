mod div;
mod text;

pub use div::Div;
pub use text::Text;

use crate::{attr::Attr, DrawErr, DrawUpdate, Stylized};

pub trait Widget: DrawUpdate {
    fn show(&mut self) -> Result<Vec<Stylized>, DrawErr>;
    fn get_attr(&self) -> &Attr;
    fn set_attr(&mut self, attr: Attr);
}
