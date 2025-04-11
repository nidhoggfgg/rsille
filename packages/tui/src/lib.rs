mod utils;

use attr::Attr;
use render::DrawUpdate;

pub mod attr;
pub mod composite;
pub mod widgets;

pub use utils::calc_render_area;
pub use utils::draw_boxes;

pub trait Widget: DrawUpdate {
    fn get_attr(&self) -> &Attr;
    fn set_attr(&mut self, attr: Attr);
}
