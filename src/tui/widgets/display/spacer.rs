//! Spacer widget — occupies space without rendering

use crate::tui::event::Event;
use crate::tui::layout::Constraints;
use crate::tui::widget::{EventCtx, RenderCtx, Widget};

/// Spacer widget that occupies space but renders nothing.
#[derive(Debug, Clone)]
pub struct Spacer<M = ()> {
    constraints: Constraints,
    widget_key: Option<String>,
    _phantom: std::marker::PhantomData<M>,
}

impl<M> Spacer<M> {
    pub fn new() -> Self {
        Self {
            constraints: Constraints::content(),
            widget_key: None,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn fixed(mut self, width: u16, height: u16) -> Self {
        self.constraints = Constraints::fixed(width, height);
        self
    }

    pub fn min(mut self, width: u16, height: u16) -> Self {
        self.constraints = Constraints::min(width, height);
        self
    }

    pub fn width(mut self, width: u16) -> Self {
        self.constraints.min_width = width;
        self.constraints.max_width = Some(width);
        self
    }

    pub fn height(mut self, height: u16) -> Self {
        self.constraints.min_height = height;
        self.constraints.max_height = Some(height);
        self
    }

    pub fn flex(mut self, flex: f32) -> Self {
        self.constraints.flex = Some(flex);
        self
    }

    pub fn fill(mut self) -> Self {
        self.constraints = Constraints::fill();
        self
    }
}

impl<M> Default for Spacer<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> Widget<M> for Spacer<M> {
    fn render(&self, _chunk: &mut crate::render::chunk::Chunk, _ctx: &RenderCtx) {}

    fn handle_event(&self, _event: &Event, _ctx: &mut EventCtx<M>) {}

    fn constraints(&self) -> Constraints {
        self.constraints
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

/// Create a new spacer widget.
pub fn spacer<M>() -> Spacer<M> {
    Spacer::new()
}
