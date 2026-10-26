//! Canvas widget — sample-based drawing with selectable terminal encoding.

use std::fmt;

use crate::canvas::Encoding;
use crate::render::Draw;

use super::world_canvas::{CanvasPoint, CanvasViewport, WorldCanvas};
use crate::tui::event::{Event, KeyModifiers, MouseEventKind};
use crate::tui::focus::FocusConfig;
use crate::tui::layout::Constraints;
use crate::tui::widget::EventPhase;
use crate::tui::widget::{EventCtx, RenderCtx, Widget};

/// Dimensions and transforms passed to a [`CanvasWidget`] draw callback.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanvasContext {
    cell_width: u16,
    cell_height: u16,
    encoding: Encoding,
    viewport: Option<CanvasViewport>,
}

impl CanvasContext {
    /// A context using the default Braille encoding.
    pub fn new(cell_width: u16, cell_height: u16) -> Self {
        Self {
            cell_width,
            cell_height,
            encoding: Encoding::default(),
            viewport: None,
        }
    }

    pub fn with_encoding(mut self, encoding: Encoding) -> Self {
        self.encoding = encoding;
        self
    }

    pub fn encoding(self) -> Encoding {
        self.encoding
    }

    pub fn with_viewport(mut self, viewport: CanvasViewport) -> Self {
        self.viewport = Some(viewport);
        self
    }

    /// The default viewport follows sample dimensions. Set an explicit viewport
    /// to retain world extents when changing encoding or terminal dimensions.
    pub fn viewport(self) -> CanvasViewport {
        self.viewport.unwrap_or_else(|| {
            CanvasViewport::new(
                0.0,
                0.0,
                (self.sample_width() - 1).max(1) as f64,
                (self.sample_height() - 1).max(1) as f64,
            )
        })
    }

    pub fn world_to_sample(self, point: CanvasPoint) -> Option<CanvasPoint> {
        if self.cell_width == 0
            || self.cell_height == 0
            || !point.x.is_finite()
            || !point.y.is_finite()
        {
            return None;
        }
        let v = self.viewport();
        let point = CanvasPoint::new(
            (point.x - v.x()) / v.width() * (self.sample_width() - 1) as f64,
            (point.y - v.y()) / v.height() * (self.sample_height() - 1) as f64,
        );
        (point.x.is_finite() && point.y.is_finite()).then_some(point)
    }

    /// A one-sample axis maps to the midpoint of its world extent.
    pub fn sample_to_world(self, point: CanvasPoint) -> Option<CanvasPoint> {
        if self.cell_width == 0
            || self.cell_height == 0
            || !point.x.is_finite()
            || !point.y.is_finite()
        {
            return None;
        }
        let v = self.viewport();
        let fraction = |value: f64, count: i32| {
            if count == 1 {
                0.5
            } else {
                value / (count - 1) as f64
            }
        };
        let point = CanvasPoint::new(
            v.x() + fraction(point.x, self.sample_width()) * v.width(),
            v.y() + fraction(point.y, self.sample_height()) * v.height(),
        );
        (point.x.is_finite() && point.y.is_finite()).then_some(point)
    }

    /// Terminal-cell center in world coordinates, accounting for the downward
    /// terminal y axis and this encoding's sample dimensions.
    pub fn cell_to_world(self, x: u16, y: u16) -> Option<CanvasPoint> {
        if x >= self.cell_width || y >= self.cell_height {
            return None;
        }
        let (w, h) = self.encoding.samples_per_cell();
        self.sample_to_world(CanvasPoint::new(
            f64::from(x) * f64::from(w) + f64::from(w - 1) / 2.0,
            f64::from(self.sample_height() - 1)
                - f64::from(y) * f64::from(h)
                - f64::from(h - 1) / 2.0,
        ))
    }

    pub fn world<'a>(self, canvas: &'a mut crate::canvas::Canvas) -> WorldCanvas<'a> {
        WorldCanvas::new(canvas, self)
    }

    /// Width of the widget area in terminal cells.
    pub fn cell_width(&self) -> u16 {
        self.cell_width
    }

    /// Height of the widget area in terminal cells.
    pub fn cell_height(&self) -> u16 {
        self.cell_height
    }

    /// Width of the drawable grid in sample coordinates.
    pub fn sample_width(&self) -> i32 {
        i32::from(self.cell_width) * i32::from(self.encoding.samples_per_cell().0)
    }

    /// Height of the drawable grid in sample coordinates.
    pub fn sample_height(&self) -> i32 {
        i32::from(self.cell_height) * i32::from(self.encoding.samples_per_cell().1)
    }
}

/// A pointer event translated through the same viewport used for drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanvasPointer {
    pub position: CanvasPoint,
    pub kind: MouseEventKind,
    pub modifiers: KeyModifiers,
}

/// Widget for drawing into a fixed-size sample canvas. Braille is the default.
///
/// The draw callback receives a fresh [`crate::canvas::Canvas`] every render.
/// The widget sets the canvas bounds to the allocated terminal-cell area before
/// calling the callback, so drawing can use sample coordinates from `0..sample_width`
/// and `0..sample_height` without manually sizing the canvas.
pub struct CanvasWidget<M = (), F = fn(&mut crate::canvas::Canvas, CanvasContext)> {
    draw: F,
    constraints: Constraints,
    widget_key: Option<String>,
    viewport: Option<CanvasViewport>,
    encoding: Encoding,
    on_pointer: Option<Box<dyn Fn(CanvasPointer) -> M>>,
    _phantom: std::marker::PhantomData<M>,
}

impl<M, F> fmt::Debug for CanvasWidget<M, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CanvasWidget")
            .field("encoding", &self.encoding)
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
            viewport: None,
            encoding: Encoding::default(),
            on_pointer: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Select the encoding used by drawing dimensions and pointer transforms.
    pub fn encoding(mut self, encoding: Encoding) -> Self {
        self.encoding = encoding;
        self
    }

    pub fn viewport(mut self, viewport: CanvasViewport) -> Self {
        self.viewport = Some(viewport);
        self
    }
    /// Use pointer messages to implement application-controlled panning, wheel zoom and hit testing.
    pub fn on_pointer(mut self, handler: impl Fn(CanvasPointer) -> M + 'static) -> Self {
        self.on_pointer = Some(Box::new(handler));
        self
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

        let mut canvas_ctx =
            CanvasContext::new(area.width(), area.height()).with_encoding(self.encoding);
        if let Some(viewport) = self.viewport {
            canvas_ctx = canvas_ctx.with_viewport(viewport);
        }
        let mut canvas = crate::canvas::Canvas::with_encoding(self.encoding);
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

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if ctx.phase() != EventPhase::Target {
            return;
        }
        let (Some(handler), Event::Mouse(mouse)) = (&self.on_pointer, event) else {
            return;
        };
        let Some((x, y)) = ctx.local_mouse_position(event) else {
            return;
        };
        let Some(area) = ctx.bounds() else {
            return;
        };
        let mut context =
            CanvasContext::new(area.width(), area.height()).with_encoding(self.encoding);
        if let Some(viewport) = self.viewport {
            context = context.with_viewport(viewport);
        }
        if let Some(position) = context.cell_to_world(x, y) {
            ctx.emit(handler(CanvasPointer {
                position,
                kind: mouse.kind,
                modifiers: mouse.modifiers,
            }));
            ctx.set_handled();
        }
    }
    fn focus_config(&self) -> FocusConfig {
        if self.on_pointer.is_some() {
            FocusConfig::Leaf
        } else {
            FocusConfig::None
        }
    }

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
    fn canvas_widget_passes_cell_and_sample_dimensions() {
        let seen = RefCell::new(None);
        let widget = canvas::<(), _>(|canvas, ctx| {
            *seen.borrow_mut() = Some(ctx);
            canvas.set(0, 0);
        });

        render_widget(&widget, 5, 3);

        assert_eq!(*seen.borrow(), Some(CanvasContext::new(5, 3)));
        let ctx = seen.borrow().unwrap();
        assert_eq!(ctx.sample_width(), 10);
        assert_eq!(ctx.sample_height(), 12);
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
            canvas.set_color(0, 0, Color::Red);
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

    #[test]
    fn half_block_widget_shares_encoding_dimensions_and_world_colors() {
        let widget = canvas::<(), _>(|surface, ctx| {
            assert_eq!(surface.encoding(), ctx.encoding());
            assert_eq!((ctx.sample_width(), ctx.sample_height()), (2, 2));
            let world = ctx.world(surface);
            world
                .color(crate::tui::style::Color::Rgb(255, 0, 0))
                .point(CanvasPoint::new(0.0, 1.0));
            ctx.world(surface)
                .color(Color::Blue)
                .point(CanvasPoint::new(0.0, 0.0));
        })
        .encoding(Encoding::HalfBlock)
        .fixed(2, 1);
        let buffer = render_widget(&widget, 2, 1);
        assert_eq!(cell_char(&buffer, 0, 0), Some('▀'));
        let colors = buffer.content()[0].content.style.colors.unwrap();
        assert_eq!(colors.foreground, Some(Color::Rgb { r: 255, g: 0, b: 0 }));
        assert_eq!(colors.background, Some(Color::Blue));
    }

    #[test]
    fn one_column_half_block_maps_to_world_midpoint_without_division_by_zero() {
        let ctx = CanvasContext::new(1, 1)
            .with_encoding(Encoding::HalfBlock)
            .with_viewport(CanvasViewport::new(-10.0, -20.0, 20.0, 40.0));
        assert_eq!(
            ctx.world_to_sample(CanvasPoint::new(7.0, 20.0)),
            Some(CanvasPoint::new(0.0, 1.0))
        );
        assert_eq!(
            ctx.sample_to_world(CanvasPoint::new(0.0, 1.0)),
            Some(CanvasPoint::new(0.0, 20.0))
        );
        assert_eq!(ctx.cell_to_world(0, 0), Some(CanvasPoint::new(0.0, 0.0)));
        assert!(ctx.cell_to_world(1, 0).is_none());
        let widget = canvas::<(), _>(|surface, ctx| {
            ctx.world(surface)
                .line(CanvasPoint::new(-1e6, 0.0), CanvasPoint::new(1e6, 0.0));
        })
        .encoding(Encoding::HalfBlock)
        .viewport(ctx.viewport())
        .fixed(1, 1);
        assert_ne!(cell_char(&render_widget(&widget, 1, 1), 0, 0), Some(' '));
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
