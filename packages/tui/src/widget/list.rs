//! List widget - scrollable list
//! Full implementation in Phase 7

use super::*;

#[derive(Debug)]
pub struct List;

impl Widget for List {
    type Message = ();

    fn render(&self, _chunk: &mut render::chunk::Chunk, _area: Area) {
        // Implemented in Phase 7
    }

    fn handle_event(&mut self, _event: &Event) -> EventResult<()> {
        EventResult::Ignored
    }

    fn constraints(&self) -> Constraints {
        Constraints::content()
    }
}
