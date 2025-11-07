//! ProgressBar widget - status indication
//! Full implementation in Phase 7

use super::*;

#[derive(Debug)]
pub struct ProgressBar;

impl Widget for ProgressBar {
    type Message = ();

    fn render(&self, _chunk: &mut render::chunk::Chunk) {
        // Implemented in Phase 7
    }

    fn handle_event(&mut self, _event: &Event) -> EventResult<()> {
        EventResult::Ignored
    }

    fn constraints(&self) -> Constraints {
        Constraints::content()
    }
}
