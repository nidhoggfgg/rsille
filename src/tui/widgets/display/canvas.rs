//! Canvas widget — adapts the braille canvas crate into the TUI layout system.

use std::fmt;

use crate::render::Draw;

use crate::tui::event::Event;
use crate::tui::layout::Constraints;
use crate::tui::widget::{EventCtx, RenderCtx, Widget};

/// Dimensions passed to a [`CanvasWidget`] draw callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasContext {
    cell_width: u16,
    cell_height: u16,
}

impl CanvasContext {
    pub fn new(cell_width: u16, cell_height: u16) -> Self {
        Self {
            cell_width,
            cell_height,
        }
    }

    /// Width of the widget area in terminal cells.
    pub fn cell_width(&self) -> u16 {
        self.cell_width
    }

    /// Height of the widget area in terminal cells.
    pub fn cell_height(&self) -> u16 {
        self.cell_height
    }

    /// Width of the drawable braille grid in dot coordinates.
    pub fn dot_width(&self) -> i32 {
        self.cell_width as i32 * 2
    }

    /// Height of the drawable braille grid in dot coordinates.
    pub fn dot_height(&self) -> i32 {
        self.cell_height as i32 * 4
    }
}

/// Widget for drawing into a fixed-size braille canvas.
///
/// The draw callback receives a fresh [`crate::canvas::Canvas`] every render.
/// The widget sets the canvas bounds to the allocated terminal-cell area before
/// calling the callback, so drawing can use dot coordinates from `0..dot_width`
/// and `0..dot_height` without manually sizing the canvas.
pub struct CanvasWidget<M = (), F = fn(&mut crate::canvas::Canvas, CanvasContext)> {
    draw: F,
    constraints: Constraints,
    widget_key: Option<String>,
    _phantom: std::marker::PhantomData<M>,
}

impl<M, F> fmt::Debug for CanvasWidget<M, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CanvasWidget")
            .field("constraints", &self.constraints)
            .field("widget_key", &self.widget_key)
            .finish_non_exhaustive()
    }
}

impl<M, F> CanvasWidget<M, F>
where
    F: Fn(&mut crate::canvas::Canvas, CanvasContext),
{
    pub fn new(draw: F) -> Self {
        Self {
            draw,
            constraints: Constraints {
                min_width: 1,
                max_width: None,
                min_height: 1,
                max_height: None,
                flex: Some(1.0),
            },
            widget_key: None,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn constraints(mut self, constraints: Constraints) -> Self {
        self.constraints = constraints;
        self
    }

    pub fn min_size(mut self, width: u16, height: u16) -> Self {
        self.constraints.min_width = width;
        self.constraints.min_height = height;
        self
    }

    pub fn fixed(mut self, width: u16, height: u16) -> Self {
        self.constraints = Constraints::fixed(width, height);
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
}

impl<M, F> Widget<M> for CanvasWidget<M, F>
where
    F: Fn(&mut crate::canvas::Canvas, CanvasContext),
{
    fn render(&self, chunk: &mut crate::render::chunk::Chunk, _ctx: &RenderCtx) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        let canvas_ctx = CanvasContext::new(area.width(), area.height());
        let mut canvas = crate::canvas::Canvas::new();
        canvas.set_bound(
            (0, area.width().saturating_sub(1) as i32),
            (0, area.height().saturating_sub(1) as i32),
        );
        canvas.fixed_bound(true);

        (self.draw)(&mut canvas, canvas_ctx);

        if let Ok(child) = chunk.from_area(area) {
            let _ = canvas.draw(child);
        }
    }

    fn handle_event(&self, _event: &Event, _ctx: &mut EventCtx<M>) {}

    fn constraints(&self) -> Constraints {
        self.constraints
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

/// Create a canvas widget from a drawing callback.
pub fn canvas<M, F>(draw: F) -> CanvasWidget<M, F>
where
    F: Fn(&mut crate::canvas::Canvas, CanvasContext),
{
    CanvasWidget::new(draw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::area::Area;
    use crate::render::buffer::Buffer;
    use crate::render::chunk::Chunk;
    use crate::render::style::Style as RenderStyle;
    use crate::tui::animation::AnimationStore;
    use crate::tui::style::Theme;
    use crate::tui::widget::{RenderCtx, WidgetPath, WidgetStore};
    use crossterm::style::{Color, Colors};
    use std::cell::RefCell;
    use std::collections::HashMap;

    #[test]
    fn canvas_widget_passes_cell_and_dot_dimensions() {
        let seen = RefCell::new(None);
        let widget = canvas::<(), _>(|canvas, ctx| {
            *seen.borrow_mut() = Some(ctx);
            canvas.set(0, 0);
        });

        render_widget(&widget, 5, 3);

        assert_eq!(*seen.borrow(), Some(CanvasContext::new(5, 3)));
        let ctx = seen.borrow().unwrap();
        assert_eq!(ctx.dot_width(), 10);
        assert_eq!(ctx.dot_height(), 12);
    }

    #[test]
    fn canvas_widget_draws_into_chunk() {
        let widget = canvas::<(), _>(|canvas, _ctx| {
            canvas.set(0, 0);
        });

        let buffer = render_widget(&widget, 2, 1);

        assert_ne!(cell_char(&buffer, 0, 0), Some(' '));
    }

    #[test]
    fn canvas_widget_preserves_canvas_colors() {
        let widget = canvas::<(), _>(|canvas, _ctx| {
            canvas.set_colorful(
                0,
                0,
                Colors {
                    foreground: Some(Color::Red),
                    background: None,
                },
            );
        });

        let buffer = render_widget(&widget, 2, 1);
        let style = buffer.content()[0].content.style;

        assert_eq!(
            style.colors.and_then(|colors| colors.foreground),
            Some(Color::Red)
        );
    }

    #[test]
    fn canvas_widget_preserves_existing_background_under_unstyled_pixels() {
        let widget = canvas::<(), _>(|canvas, _ctx| {
            canvas.set(0, 0);
        });
        let background = Colors {
            foreground: None,
            background: Some(Color::Blue),
        };

        let buffer =
            render_widget_with_background(&widget, 2, 1, RenderStyle::with_colors(background));
        let style = buffer.content()[0].content.style;

        assert_eq!(
            style.colors.and_then(|colors| colors.background),
            Some(Color::Blue)
        );
    }

    fn render_widget<F>(widget: &CanvasWidget<(), F>, width: u16, height: u16) -> Buffer
    where
        F: Fn(&mut crate::canvas::Canvas, CanvasContext),
    {
        let mut buffer = Buffer::new((width, height).into());
        let area = Area::new((0, 0).into(), (width, height).into());
        let mut chunk = Chunk::new(&mut buffer, area).unwrap();
        let store = WidgetStore::new();
        let animation_store = AnimationStore::new();
        let theme = Theme::dark();
        let geometry = RefCell::new(HashMap::<WidgetPath, Area>::new());
        let ctx = RenderCtx::new(&store, &animation_store, &theme, None, &geometry);

        widget.render(&mut chunk, &ctx);
        drop(chunk);
        buffer
    }

    fn render_widget_with_background<F>(
        widget: &CanvasWidget<(), F>,
        width: u16,
        height: u16,
        background: RenderStyle,
    ) -> Buffer
    where
        F: Fn(&mut crate::canvas::Canvas, CanvasContext),
    {
        let mut buffer = Buffer::new((width, height).into());
        let area = Area::new((0, 0).into(), (width, height).into());
        let store = WidgetStore::new();
        let animation_store = AnimationStore::new();
        let theme = Theme::dark();
        let geometry = RefCell::new(HashMap::<WidgetPath, Area>::new());
        let ctx = RenderCtx::new(&store, &animation_store, &theme, None, &geometry);

        {
            let mut chunk = Chunk::new(&mut buffer, area).unwrap();
            for y in 0..height {
                for x in 0..width {
                    let _ = chunk.set_char(x, y, ' ', background);
                }
            }
        }

        let mut chunk = Chunk::new(&mut buffer, area).unwrap();
        widget.render(&mut chunk, &ctx);
        drop(chunk);
        buffer
    }

    fn cell_char(buffer: &Buffer, x: u16, y: u16) -> Option<char> {
        let index = (y * buffer.size().width + x) as usize;
        buffer.content()[index].content.as_char()
    }
}
