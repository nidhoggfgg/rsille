mod div;
mod text;

pub use div::Div;
use render::{Draw, DrawErr, DrawUpdate};
use term::style::Stylized;
pub use text::Text;

use crate::attr::Attr;

pub trait Widget: DrawUpdate {
    fn show(&mut self) -> Result<Vec<Stylized>, DrawErr>;
    fn get_attr(&self) -> &Attr;
    fn set_attr(&mut self, attr: Attr);
}

pub fn wrap_with_attr<T>(thing: &mut T, _attr: &Attr) -> Result<Vec<Stylized>, DrawErr>
where
    T: Draw,
{
    let _data = thing.draw()?;

    todo!()
}
