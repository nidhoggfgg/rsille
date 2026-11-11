//! Opaque RGB graphics with a fixed source resolution and revision-based drawing.

use std::cell::{Cell, RefCell};
use std::fmt;
use std::sync::Arc;

use crate::render::area::Area;
use crate::render::chunk::Chunk;
use crate::render::raster::{
    RasterAlign, RasterCanvas, RasterCell, RasterError, RasterOptions, RasterScale, Rgb,
    MAX_RASTER_PIXELS,
};
use crate::render::style::Style as RenderStyle;
use crate::tui::layout::{Constraints, Layout};

use crate::tui::widget::{Lifecycle, MeasureCtx, RenderCtx, StateCtx, Widget};
use crossterm::style::{Attributes, Color, Colors};

/// Fixed source-pixel dimensions passed to a raster drawing callback.
///
/// These are deliberately independent of terminal cells, clipping, scaling and
/// presentation backend. Resizing the widget does not rerun the callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RasterCanvasContext {
    pixel_width: u32,
    pixel_height: u32,
}

impl RasterCanvasContext {
    /// Returns the source width in pixels.
    pub fn pixel_width(self) -> u32 {
        self.pixel_width
    }

    /// Returns the source height in pixels.
    pub fn pixel_height(self) -> u32 {
        self.pixel_height
    }
}

enum Source<F> {
    Shared(Arc<RasterCanvas>),
    Draw { width: u32, height: u32, draw: F },
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct SourceKey {
    width: u32,
    height: u32,
    background: Rgb,
    revision: u64,
}

struct CachedSource {
    key: SourceKey,
    result: Result<RasterCanvas, RasterError>,
}

#[derive(Default)]
struct RasterState {
    source: RefCell<Option<CachedSource>>,
    // Keep the allocation before Chunk::with_clip reduces it to the visible area.
    allocation: Cell<Option<Area>>,
}

/// Displays a shared raster canvas or caches a fixed-resolution drawing callback.
///
/// Use [`Self::new`] (or [`raster_canvas`]) for an immutable shared source and
/// [`Self::from_draw`] for procedural drawing. Callback results survive view
/// rebuilds at the same widget identity. Change [`Self::revision`] when captured
/// drawing data changes; a different callback alone does not invalidate pixels.
/// A stable [`Self::key`] preserves the cache when siblings are reordered.
///
/// Source dimensions and background also invalidate the callback cache. Layout,
/// alignment and scale changes only affect presentation. Each callback
/// starts with a fresh, opaque canvas, not the previous revision's pixels.
///
/// Borders and padding belong to the surrounding container. The widget uses its
/// allocated content area and crops an already scaled image when partially
/// visible, rather than fitting the whole source into the visible fragment.
/// Raster submission is protocol-independent and never writes to the terminal.
///
/// ```
/// use rsille::tui::prelude::*;
///
/// let image = RasterCanvas::new(320, 180, Rgb(20, 20, 20)).unwrap();
/// let view = raster_canvas::<()>(image)
///     .scale(RasterScale::Fit)
///     .fixed(40, 12);
/// # let _ = view;
/// ```
pub struct RasterCanvasWidget<M = (), F = fn(&mut RasterCanvas, RasterCanvasContext)> {
    source: Source<F>,
    options: RasterOptions,
    revision: u64,
    constraints: Constraints,
    widget_key: Option<String>,
    // Direct rendering without lifecycle reconciliation still caches per instance.
    local_state: RasterState,
    _phantom: std::marker::PhantomData<M>,
}

impl<M, F> fmt::Debug for RasterCanvasWidget<M, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RasterCanvasWidget")
            .field("revision", &self.revision)
            .field("constraints", &self.constraints)
            .field("widget_key", &self.widget_key)
            .finish_non_exhaustive()
    }
}

impl<M> RasterCanvasWidget<M> {
    /// Displays an owned canvas or an `Arc<RasterCanvas>` without copying pixels.
    pub fn new(canvas: impl Into<Arc<RasterCanvas>>) -> Self {
        Self::with_source(Source::Shared(canvas.into()))
    }
}

impl<M, F> RasterCanvasWidget<M, F> {
    fn with_source(source: Source<F>) -> Self {
        Self {
            source,
            options: RasterOptions::default(),
            revision: 0,
            constraints: Constraints {
                min_width: 1,
                max_width: None,
                min_height: 1,
                max_height: None,
                flex: Some(1.0),
            },
            widget_key: None,
            local_state: RasterState::default(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Replaces all raster presentation options.
    ///
    /// Background also initializes callback canvases and invalidates their cache.
    pub fn options(mut self, options: RasterOptions) -> Self {
        self.options = options;
        self
    }

    /// Sets the opaque letterbox color and callback canvas's initial background.
    /// This does not recolor an existing shared canvas.
    pub fn background(mut self, background: Rgb) -> Self {
        self.options.background = background;
        self
    }

    /// Selects fit, stretch, or aligned/cropped original-pixel presentation.
    pub fn scale(mut self, scale: RasterScale) -> Self {
        self.options.scale = scale;
        self
    }

    /// Sets horizontal alignment within the allocated content area.
    pub fn align_x(mut self, align: RasterAlign) -> Self {
        self.options.align_x = align;
        self
    }

    /// Sets vertical alignment within the allocated content area.
    pub fn align_y(mut self, align: RasterAlign) -> Self {
        self.options.align_y = align;
        self
    }

    /// Sets the application-controlled callback content revision (initially zero).
    ///
    /// Keep this value stable across unrelated view rebuilds, and change it when
    /// drawing inputs change. Shared sources use their own resource id/version.
    pub fn revision(mut self, revision: u64) -> Self {
        self.revision = revision;
        self
    }

    /// Invalidates this retained widget instance's callback for its next render.
    ///
    /// This does not schedule a frame. Rebuilt views should instead supply an
    /// application-owned [`Self::revision`].
    pub fn invalidate(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    /// Sets the stable identity used across view rebuilds and sibling reordering.
    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    /// Replaces layout constraints, in terminal cells.
    pub fn constraints(mut self, constraints: Constraints) -> Self {
        self.constraints = constraints;
        self
    }

    /// Sets the minimum width and height in terminal cells.
    pub fn min_size(mut self, width: u16, height: u16) -> Self {
        self.constraints.min_width = width;
        self.constraints.min_height = height;
        self
    }

    /// Fixes width and height in terminal cells, independently of source pixels.
    pub fn fixed(mut self, width: u16, height: u16) -> Self {
        self.constraints = Constraints::fixed(width, height);
        self
    }

    /// Fixes the width in terminal columns.
    pub fn width(mut self, width: u16) -> Self {
        self.constraints.min_width = width;
        self.constraints.max_width = Some(width);
        self
    }

    /// Fixes the height in terminal rows.
    pub fn height(mut self, height: u16) -> Self {
        self.constraints.min_height = height;
        self.constraints.max_height = Some(height);
        self
    }

    /// Sets the flex weight used to distribute available layout space.
    pub fn flex(mut self, flex: f32) -> Self {
        self.constraints.flex = Some(flex);
        self
    }

    fn render_error(&self, chunk: &mut Chunk<'_>, allocation: Area, error: RasterError) {
        let Rgb(r, g, b) = self.options.background;
        let foreground = if u16::from(r) + u16::from(g) + u16::from(b) >= 384 {
            0
        } else {
            255
        };
        let style = RenderStyle::with_both(
            Colors::new(
                Color::Rgb {
                    r: foreground,
                    g: foreground,
                    b: foreground,
                },
                Color::Rgb { r, g, b },
            ),
            Attributes::default(),
        );
        let message = match error {
            RasterError::EmptyDimensions => "Raster error: empty dimensions",
            RasterError::TooManyPixels => "Raster error: dimensions too large",
            RasterError::AllocationFailed => "Raster error: allocation failed",
            RasterError::InvalidPixelLength => "Raster error: invalid pixel length",
        };
        // Keep the diagnostic visible even when the source's top-left is clipped.
        let _ = chunk.with_clip(allocation, |visible| {
            let size = visible.area().size();
            let _ = visible.fill(0, 0, size.width, size.height, ' ', style);
            let _ = visible.set_string(0, 0, message, style);
        });
    }

    fn submit(&self, chunk: &mut Chunk<'_>, canvas: &RasterCanvas, allocation: Area) {
        // The full target is mapping geometry, never an offscreen allocation.
        // Sample only visible cells, retaining their offsets through nested clips.
        let _ = chunk.with_clip(allocation, |visible| {
            let area = visible.area();
            let source_x = area.x() - allocation.x();
            let source_y = area.y() - allocation.y();
            for y in 0..area.height() {
                for x in 0..area.width() {
                    let _ = visible.set_raster_cell(
                        x,
                        y,
                        RasterCell {
                            canvas: canvas.clone(),
                            target: allocation.size(),
                            x: source_x + x,
                            y: source_y + y,
                            options: self.options,
                        },
                    );
                }
            }
        });
    }
}

impl<M, F> RasterCanvasWidget<M, F>
where
    F: Fn(&mut RasterCanvas, RasterCanvasContext),
{
    /// Creates a source with explicit, fixed pixel dimensions and cached drawing.
    ///
    /// Validates dimensions without allocating pixels. Allocation and `draw` are
    /// deferred until the widget is visible and its source cache misses. Unrelated
    /// view rebuilds neither allocate source pixels nor rerun the callback.
    ///
    /// # Errors
    ///
    /// Returns [`RasterError::EmptyDimensions`] for a zero dimension, or
    /// [`RasterError::TooManyPixels`] for overflow or exceeding [`MAX_RASTER_PIXELS`].
    /// Deferred allocation failure displays an opaque `Raster error: allocation
    /// failed` diagnostic instead of silently omitting the widget. Failures are
    /// cached; change the revision, dimensions or background to retry. The draw
    /// callback is not called on failure. For application-level handling of
    /// allocation errors before rendering, construct a [`RasterCanvas`] yourself
    /// and pass it to [`RasterCanvasWidget::new`].
    ///
    /// ```
    /// use rsille::tui::prelude::*;
    ///
    /// let view = RasterCanvasWidget::<(), _>::from_draw(320, 180, |canvas, ctx| {
    ///     canvas.set_pixel((ctx.pixel_width() / 2) as i32,
    ///                      (ctx.pixel_height() / 2) as i32, Rgb(255, 80, 20));
    /// })?
    /// .revision(1)
    /// .background(Rgb(0, 0, 0))
    /// .align_x(RasterAlign::Center)
    /// .align_y(RasterAlign::Center);
    /// # let _ = view;
    /// # Ok::<(), RasterError>(())
    /// ```
    pub fn from_draw(width: u32, height: u32, draw: F) -> Result<Self, RasterError> {
        if width == 0 || height == 0 {
            return Err(RasterError::EmptyDimensions);
        }
        usize::try_from(width)
            .ok()
            .and_then(|width| {
                usize::try_from(height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .filter(|&pixels| pixels <= MAX_RASTER_PIXELS)
            .ok_or(RasterError::TooManyPixels)?;
        Ok(Self::with_source(Source::Draw {
            width,
            height,
            draw,
        }))
    }
}

impl<M, F> Widget<M> for RasterCanvasWidget<M, F>
where
    F: Fn(&mut RasterCanvas, RasterCanvasContext),
{
    fn lifecycle(&self, ctx: &mut StateCtx) {
        if ctx.lifecycle() != Lifecycle::Unmount {
            ctx.state_mut::<RasterState>();
        }
    }

    fn layout(&self, area: Area, ctx: &MeasureCtx) -> Layout {
        ctx.state::<RasterState>()
            .unwrap_or(&self.local_state)
            .allocation
            .set(Some(area));
        Layout::default()
    }

    fn render(&self, chunk: &mut Chunk<'_>, ctx: &RenderCtx) {
        let state = ctx.state::<RasterState>().unwrap_or(&self.local_state);
        let allocation = ctx
            .layout_target()
            .or_else(|| state.allocation.get())
            .unwrap_or_else(|| chunk.area());
        if allocation.clamp_to(&chunk.area()).is_none() {
            return;
        }
        match &self.source {
            Source::Shared(canvas) => {
                // Do not retain an unused procedural source when switching modes.
                state.source.borrow_mut().take();
                self.submit(chunk, canvas, allocation);
            }
            Source::Draw {
                width,
                height,
                draw,
            } => {
                let key = SourceKey {
                    width: *width,
                    height: *height,
                    background: self.options.background,
                    revision: self.revision,
                };
                let mut cached = state.source.borrow_mut();
                if cached.as_ref().map(|cached| cached.key) != Some(key) {
                    let result =
                        RasterCanvas::new(*width, *height, key.background).map(|mut canvas| {
                            draw(
                                &mut canvas,
                                RasterCanvasContext {
                                    pixel_width: *width,
                                    pixel_height: *height,
                                },
                            );
                            canvas
                        });
                    *cached = Some(CachedSource { key, result });
                }
                if let Some(cached) = cached.as_ref() {
                    match &cached.result {
                        Ok(canvas) => self.submit(chunk, canvas, allocation),
                        Err(error) => self.render_error(chunk, allocation, *error),
                    }
                }
            }
        }
    }

    fn constraints(&self) -> Constraints {
        self.constraints
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

/// Creates a raster widget from an owned canvas or a shared `Arc<RasterCanvas>`.
/// For cached procedural drawing, use [`RasterCanvasWidget::from_draw`].
pub fn raster_canvas<M>(canvas: impl Into<Arc<RasterCanvas>>) -> RasterCanvasWidget<M> {
    RasterCanvasWidget::new(canvas)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::render::buffer::Buffer;
    use crate::tui::animation::{AnimationStore, MotionPolicy};
    use crate::tui::app::App;
    use crate::tui::event::Event;
    use crate::tui::layout::{col, ChildLayout};
    use crate::tui::style::{Padding, Theme};
    use crate::tui::widget::{WidgetId, WidgetPath, WidgetStore};
    use crate::tui::widgets::{label, panel};

    const RED: Rgb = Rgb(240, 10, 20);
    const BLUE: Rgb = Rgb(10, 20, 240);

    fn area(x: u16, y: u16, width: u16, height: u16) -> Area {
        Area::new((x, y).into(), (width, height).into())
    }

    fn reconcile(widget: &impl Widget<()>, store: &mut WidgetStore, lifecycle: Lifecycle) {
        widget.lifecycle(&mut StateCtx::new(
            store,
            WidgetPath::root(),
            WidgetId::root(),
            lifecycle,
        ));
    }

    fn render(
        widget: &impl Widget<()>,
        store: &WidgetStore,
        allocation: Area,
        clip: Area,
        prepare_layout: bool,
    ) -> Buffer {
        let mut buffer = Buffer::new(clip.real_size());
        let animations = AnimationStore::new();
        let theme = Theme::dark();
        let geometry = RefCell::new(HashMap::new());
        let ctx = RenderCtx::new(store, &animations, &theme, None, &geometry);
        if prepare_layout {
            widget.layout(allocation, &ctx.measure_ctx());
        }
        widget.render(&mut Chunk::new(&mut buffer, clip).unwrap(), &ctx);
        buffer
    }

    #[test]
    fn shared_source_and_options_are_submitted_only_inside_content_area() {
        let source = Arc::new(RasterCanvas::new(7, 5, RED).unwrap());
        let options = RasterOptions {
            background: BLUE,
            scale: RasterScale::Stretch,
            align_x: RasterAlign::End,
            align_y: RasterAlign::Start,
        };
        let widget = raster_canvas::<()>(source.clone()).options(options);
        let bounds = area(2, 1, 4, 2);
        let frame = render(&widget, &WidgetStore::new(), bounds, bounds, true);
        assert_eq!(Arc::strong_count(&source), 2);
        assert_eq!(frame.raster_cells().count(), 8);
        for (x, y, cell) in frame.raster_cells() {
            assert!(x >= 2 && y >= 1);
            assert_eq!(cell.canvas.id(), source.id());
            assert_eq!(cell.canvas.version(), source.version());
            assert_eq!(cell.target, bounds.size());
            assert_eq!((cell.x, cell.y), (x - 2, y - 1));
            assert_eq!(cell.options, options);
        }
        assert!(frame.raster_at((0, 0).into()).is_none());
        assert_eq!((source.width(), source.height()), (7, 5));
    }

    #[test]
    fn replacing_a_shared_snapshot_uses_its_current_version() {
        let mut source = RasterCanvas::new(2, 2, BLUE).unwrap();
        let bounds = area(0, 0, 2, 1);
        let mut store = WidgetStore::new();
        let original = raster_canvas::<()>(source.clone());
        reconcile(&original, &mut store, Lifecycle::Mount);
        let first = render(&original, &store, bounds, bounds, true);
        source.set_pixel(0, 0, RED);
        let updated = raster_canvas::<()>(source.clone());
        reconcile(&updated, &mut store, Lifecycle::Sync);
        let second = render(&updated, &store, bounds, bounds, false);
        let before = &first.raster_at((0, 0).into()).unwrap().canvas;
        let after = &second.raster_at((0, 0).into()).unwrap().canvas;
        assert_eq!(before.id(), after.id());
        assert_ne!(before.version(), after.version());
        assert_eq!(before.pixels()[0], BLUE);
        assert_eq!(after.pixels()[0], RED);
        assert_eq!(after.version(), source.version());
    }

    #[test]
    fn fit_alignment_background_and_stretch_are_presentation_only() {
        let source = Arc::new(RasterCanvas::new(1, 2, RED).unwrap());
        let bounds = area(0, 0, 4, 2);
        let store = WidgetStore::new();
        for (align, red_columns) in [
            (RasterAlign::Start, 0..2),
            (RasterAlign::Center, 1..3),
            (RasterAlign::End, 2..4),
        ] {
            let widget = raster_canvas::<()>(source.clone())
                .background(BLUE)
                .scale(RasterScale::Fit)
                .align_x(align);
            let frame = render(&widget, &store, bounds, bounds, true);
            for x in 0..4 {
                let cell = frame.raster_at((x, 0).into()).unwrap();
                assert_eq!(
                    cell.sample(0, 0, 1, 2),
                    if red_columns.contains(&x) { RED } else { BLUE }
                );
            }
        }
        let widget = raster_canvas::<()>(source.clone()).scale(RasterScale::Stretch);
        let frame = render(&widget, &store, bounds, bounds, true);
        assert!(frame
            .raster_cells()
            .all(|(_, _, cell)| cell.sample(0, 0, 1, 2) == RED));
        assert_eq!(source.pixels(), &[RED, RED]);

        let wide = Arc::new(RasterCanvas::new(4, 1, RED).unwrap());
        for (align, red_rows) in [
            (RasterAlign::Start, 0..1),
            (RasterAlign::Center, 1..2),
            (RasterAlign::End, 3..4),
        ] {
            let widget = raster_canvas::<()>(wide.clone())
                .background(BLUE)
                .align_y(align);
            let frame = render(&widget, &store, bounds, bounds, true);
            for pixel_y in 0..4 {
                let cell = frame.raster_at((0, pixel_y / 2).into()).unwrap();
                assert_eq!(
                    cell.sample(0, u32::from(pixel_y % 2), 1, 2),
                    if red_rows.contains(&pixel_y) {
                        RED
                    } else {
                        BLUE
                    }
                );
            }
        }
    }

    #[test]
    fn construction_and_mount_validate_dimensions_without_allocating_source_pixels() {
        let widget = RasterCanvasWidget::<(), _>::from_draw(MAX_RASTER_PIXELS as u32, 1, |_, _| {
            panic!("construction and mount must not draw");
        })
        .unwrap();
        // A procedural description contains dimensions and drawing code, not a
        // pixel buffer. Even the maximum supported source is cheap to describe.
        let Source::Draw {
            width,
            height,
            draw: _,
        } = &widget.source
        else {
            panic!("expected procedural source");
        };
        assert_eq!((*width, *height), (MAX_RASTER_PIXELS as u32, 1));
        assert!(widget.local_state.source.borrow().is_none());
        let mut store = WidgetStore::new();
        reconcile(&widget, &mut store, Lifecycle::Mount);
        assert!(store
            .get::<RasterState>(&WidgetId::root())
            .unwrap()
            .source
            .borrow()
            .is_none());
        let too_large =
            RasterCanvasWidget::<(), _>::from_draw(MAX_RASTER_PIXELS as u32 + 1, 1, |_, _| {});
        assert_eq!(too_large.unwrap_err(), RasterError::TooManyPixels);
    }

    #[test]
    fn cached_allocation_failures_are_visible_survive_rebuilds_and_retry_on_invalidation() {
        let calls = Cell::new(0);
        let make = || {
            RasterCanvasWidget::<(), _>::from_draw(8, 6, |_, _| {
                calls.set(calls.get() + 1);
            })
            .unwrap()
            .background(BLUE)
        };
        let widget = make();
        let mut store = WidgetStore::new();
        reconcile(&widget, &mut store, Lifecycle::Mount);
        // Seed a failed allocation deterministically rather than exhausting the
        // process allocator. Rendering must retain and surface this result.
        store
            .get::<RasterState>(&WidgetId::root())
            .unwrap()
            .source
            .replace(Some(CachedSource {
                key: SourceKey {
                    width: 8,
                    height: 6,
                    background: BLUE,
                    revision: 0,
                },
                result: Err(RasterError::AllocationFailed),
            }));
        let allocation = area(2, 1, 40, 3);
        let clip = area(4, 2, 36, 1);
        let first = render(&widget, &store, allocation, clip, true);
        assert!(!first.has_raster());
        let text: String = first
            .content()
            .iter()
            .filter_map(|cell| cell.content.as_char())
            .collect();
        assert!(text.contains("Raster error: allocation failed"));
        let first_visible = 2 * first.size().width as usize + 4;
        assert_eq!(
            first.content()[first_visible]
                .content
                .style
                .colors
                .unwrap()
                .background,
            Some(Color::Rgb {
                r: BLUE.0,
                g: BLUE.1,
                b: BLUE.2
            })
        );
        assert_eq!(calls.get(), 0);

        let mut rebuilt = make();
        reconcile(&rebuilt, &mut store, Lifecycle::Sync);
        let second = render(&rebuilt, &store, allocation, clip, false);
        assert_eq!(first.content(), second.content());
        assert_eq!(calls.get(), 0);
        rebuilt.invalidate();
        let retried = render(&rebuilt, &store, allocation, clip, false);
        assert!(retried.has_raster());
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn callback_uses_fixed_pixels_and_reuses_cache_across_resizing() {
        let calls = Cell::new(0);
        let widget = RasterCanvasWidget::<(), _>::from_draw(17, 11, |canvas, ctx| {
            calls.set(calls.get() + 1);
            assert_eq!((ctx.pixel_width(), ctx.pixel_height()), (17, 11));
            assert_eq!((canvas.width(), canvas.height()), (17, 11));
            canvas.set_pixel(0, 0, RED);
        })
        .unwrap();
        let store = WidgetStore::new();
        let small = area(0, 0, 2, 1);
        let large = area(1, 2, 8, 5);
        let first = render(&widget, &store, small, small, true);
        let second = render(&widget, &store, large, large, true);
        assert_eq!(calls.get(), 1);
        assert_eq!(
            first.raster_at((0, 0).into()).unwrap().canvas,
            second.raster_at((1, 2).into()).unwrap().canvas,
        );
    }

    #[test]
    fn lifecycle_cache_survives_description_rebuilds_and_only_source_changes_redraw() {
        let calls = Cell::new(0);
        let make = |revision, width, background, scale| {
            RasterCanvasWidget::<(), _>::from_draw(width, 5, |_, _| calls.set(calls.get() + 1))
                .unwrap()
                .revision(revision)
                .background(background)
                .scale(scale)
        };
        let mut store = WidgetStore::new();
        let bounds = area(0, 0, 4, 2);
        let initial = make(0, 7, RED, RasterScale::Fit);
        reconcile(&initial, &mut store, Lifecycle::Mount);
        let first = render(&initial, &store, bounds, bounds, true);
        drop(initial);
        let rebuilt = make(0, 7, RED, RasterScale::Stretch);
        reconcile(&rebuilt, &mut store, Lifecycle::Sync);
        let second = render(&rebuilt, &store, bounds, bounds, false);
        assert_eq!(calls.get(), 1);
        assert_eq!(
            first.raster_at((0, 0).into()).unwrap().canvas,
            second.raster_at((0, 0).into()).unwrap().canvas
        );
        for (revision, width, background, expected_calls) in
            [(1, 7, RED, 2), (1, 9, RED, 3), (1, 9, BLUE, 4)]
        {
            let widget = make(revision, width, background, RasterScale::Fit);
            reconcile(&widget, &mut store, Lifecycle::Sync);
            render(&widget, &store, bounds, bounds, false);
            assert_eq!(calls.get(), expected_calls);
        }
    }

    #[test]
    fn explicit_invalidation_starts_with_a_fresh_background() {
        let calls = Cell::new(0);
        let mut widget = RasterCanvasWidget::<(), _>::from_draw(2, 2, |canvas, _| {
            if calls.get() == 0 {
                canvas.set_pixel(0, 0, RED);
            }
            calls.set(calls.get() + 1);
        })
        .unwrap()
        .background(BLUE);
        let store = WidgetStore::new();
        let bounds = area(0, 0, 2, 1);
        let first = render(&widget, &store, bounds, bounds, true);
        assert_eq!(
            first.raster_at((0, 0).into()).unwrap().canvas.pixels()[0],
            RED
        );
        widget.invalidate();
        let second = render(&widget, &store, bounds, bounds, true);
        assert_eq!(calls.get(), 2);
        assert_eq!(
            second.raster_at((0, 0).into()).unwrap().canvas.pixels(),
            &[BLUE; 4]
        );
    }

    #[test]
    fn maximum_target_with_tiny_viewport_only_submits_visible_cells() {
        let source = Arc::new(RasterCanvas::new(2, 2, RED).unwrap());
        let allocation = area(0, 0, u16::MAX, u16::MAX);
        let clip = area(3, 2, 2, 2);
        for scale in [
            RasterScale::Fit,
            RasterScale::Stretch,
            RasterScale::Original,
        ] {
            let widget = raster_canvas::<()>(source.clone()).scale(scale);
            let frame = render(&widget, &WidgetStore::new(), allocation, clip, true);
            assert_eq!(frame.size(), (5, 4).into());
            assert_eq!(frame.content().len(), 20);
            assert_eq!(frame.raster_cells().count(), 4);
            for (x, y, cell) in frame.raster_cells() {
                assert_eq!(
                    cell,
                    &RasterCell {
                        canvas: source.as_ref().clone(),
                        target: allocation.size(),
                        x,
                        y,
                        options: widget.options,
                    }
                );
                let index = y as usize * frame.size().width as usize + x as usize;
                assert_eq!(frame.content()[index].content, cell.placeholder());
            }
            assert!(frame.raster_at((2, 2).into()).is_none());
            assert!(frame.raster_at((3, 1).into()).is_none());
        }
    }

    #[test]
    fn nested_clips_preserve_nonzero_target_origin() {
        let mut source = RasterCanvas::new(6, 8, BLUE).unwrap();
        source.fill_rect(0, 0, 3, 4, RED);
        let source = Arc::new(source);
        let widget = raster_canvas::<()>(source.clone());
        let allocation = area(2, 1, 6, 4);
        let full = render(&widget, &WidgetStore::new(), allocation, allocation, true);
        let mut frame = Buffer::new((10, 7).into());
        let mut chunk = Chunk::new(&mut frame, area(1, 1, 8, 6)).unwrap();
        chunk
            .with_clip(area(3, 2, 4, 3), |outer| {
                // Extends outside the outer clip; only their intersection is visible.
                outer
                    .with_clip(area(4, 0, 5, 4), |inner| {
                        widget.submit(inner, &source, allocation);
                    })
                    .unwrap();
            })
            .unwrap();
        assert_eq!(frame.raster_cells().count(), 6);
        for (x, y, cell) in frame.raster_cells() {
            assert!((4..7).contains(&x) && (2..4).contains(&y));
            assert_eq!(cell, full.raster_at((x, y).into()).unwrap());
            assert_eq!((cell.x, cell.y), (x - 2, y - 1));
            let index = y as usize * frame.size().width as usize + x as usize;
            assert_eq!(frame.content()[index].content, cell.placeholder());
        }
    }

    #[test]
    fn clipping_preserves_target_geometry_source_offsets_and_placeholder_cells() {
        let mut source = RasterCanvas::new(6, 8, BLUE).unwrap();
        source.fill_rect(0, 0, 3, 4, RED);
        let source = Arc::new(source);
        let allocation = area(1, 1, 6, 4);
        for scale in [
            RasterScale::Fit,
            RasterScale::Stretch,
            RasterScale::Original,
        ] {
            let widget = raster_canvas::<()>(source.clone()).scale(scale);
            let store = WidgetStore::new();
            let full = render(&widget, &store, allocation, allocation, true);
            for clip in [area(3, 2, 2, 1), area(1, 1, 2, 2), area(5, 3, 2, 2)] {
                let cropped = render(&widget, &store, allocation, clip, false);
                assert_eq!(cropped.raster_cells().count(), clip.area());
                for (x, y, cell) in cropped.raster_cells() {
                    let original = full.raster_at((x, y).into()).unwrap();
                    assert_eq!(cell, original);
                    assert_eq!(cell.target, allocation.size());
                    assert_eq!((cell.x, cell.y), (x - allocation.x(), y - allocation.y()));
                    assert_eq!(cell.placeholder(), original.placeholder());
                    let index = y as usize * cropped.size().width as usize + x as usize;
                    assert_eq!(cropped.content()[index].content, cell.placeholder());
                }
            }
        }
    }

    #[test]
    fn empty_and_invisible_targets_defer_drawing_and_invalid_sources_return_errors() {
        let calls = Cell::new(0);
        let widget =
            RasterCanvasWidget::<(), _>::from_draw(4, 4, |_, _| calls.set(calls.get() + 1))
                .unwrap();
        let store = WidgetStore::new();
        let empty = area(0, 0, 0, 0);
        assert!(!render(&widget, &store, empty, empty, true).has_raster());
        assert!(!render(&widget, &store, area(4, 4, 2, 2), area(0, 0, 2, 2), true).has_raster());
        for (width, height, error) in [
            (0, 4, RasterError::EmptyDimensions),
            (4, 0, RasterError::EmptyDimensions),
            (u32::MAX, u32::MAX, RasterError::TooManyPixels),
        ] {
            let invalid = RasterCanvasWidget::<(), _>::from_draw(width, height, |_, _| {
                calls.set(calls.get() + 1)
            });
            assert_eq!(invalid.unwrap_err(), error);
        }
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn headless_panel_reserves_border_and_padding_and_exports_are_available() {
        use crate::tui::prelude::{
            raster_canvas, RasterCanvas, RasterCanvasContext, RasterCanvasWidget, Rgb,
        };
        let _: Option<RasterCanvasContext> = None;
        let source = Arc::new(RasterCanvas::new(8, 6, Rgb(3, 4, 5)).unwrap());
        let mut app = App::new(())
            .build(
                |_, ()| {},
                move |_| {
                    let widget: RasterCanvasWidget<()> = raster_canvas(source.clone());
                    panel()
                        .padding(Padding::uniform(1))
                        .child(widget.height(3).key("image"))
                },
            )
            .size(12, 8);
        app.step(&[]).unwrap();
        let expected = area(2, 2, 8, 3);
        assert_eq!(
            app.bounds(&WidgetPath::root().child("image")),
            Some(expected)
        );
        assert_eq!(app.frame().raster_cells().count(), expected.area());
        for (x, y, cell) in app.frame().raster_cells() {
            assert!((2..10).contains(&x) && (2..5).contains(&y));
            assert_eq!(cell.target, expected.size());
        }
        assert!(!app.frame().content()[0].content.is_empty());
        assert!(app.frame().raster_at((0, 0).into()).is_none());
    }

    #[test]
    fn headless_unrelated_updates_and_resize_preserve_callback_resource() {
        let calls = Arc::new(AtomicUsize::new(0));
        let draw_calls = calls.clone();
        let mut app = App::new((0_u64, 0_u64))
            .with_motion_policy(MotionPolicy::disabled())
            .build(
                |state, next| *state = next,
                move |state| {
                    let draw_calls = draw_calls.clone();
                    col().child(label(state.0.to_string())).child(
                        RasterCanvasWidget::from_draw(16, 12, move |canvas, _| {
                            draw_calls.fetch_add(1, Ordering::SeqCst);
                            canvas.set_pixel(0, 0, RED);
                        })
                        .unwrap()
                        .revision(state.1)
                        .key("image"),
                    )
                },
            )
            .size(12, 8);
        app.step(&[]).unwrap();
        let id = app.frame().raster_cells().next().unwrap().2.canvas.id();
        app.handle().send((1, 0)).unwrap();
        assert!(app.step(&[]).unwrap().rendered);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(app.frame().raster_cells().next().unwrap().2.canvas.id(), id);
        app.step(&[Event::Resize(20, 10)]).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(app.frame().raster_cells().next().unwrap().2.canvas.id(), id);
        app.handle().send((1, 1)).unwrap();
        app.step(&[]).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_ne!(app.frame().raster_cells().next().unwrap().2.canvas.id(), id);
        assert!(app
            .widget_state::<RasterState>(&WidgetId::root().child("image"))
            .is_some());
    }

    #[test]
    fn headless_parent_clipping_does_not_rescale_to_visible_width() {
        struct Clipped {
            children: Vec<Box<dyn Widget<()>>>,
        }
        impl Widget<()> for Clipped {
            fn render(&self, chunk: &mut Chunk<'_>, ctx: &RenderCtx) {
                ctx.render_children(chunk, self);
            }
            fn layout(&self, _: Area, _: &MeasureCtx) -> Layout {
                Layout::new([ChildLayout::new(0, area(1, 0, 8, 3))])
            }
            fn children(&self) -> &[Box<dyn Widget<()>>] {
                &self.children
            }
            fn constraints(&self) -> Constraints {
                Constraints::fill()
            }
        }
        let mut app = App::new(())
            .build(
                |_, ()| {},
                |_| Clipped {
                    children: vec![Box::new(raster_canvas(
                        RasterCanvas::new(8, 6, RED).unwrap(),
                    ))],
                },
            )
            .size(4, 2);
        app.step(&[]).unwrap();
        assert_eq!(app.frame().raster_cells().count(), 6);
        for (x, y, cell) in app.frame().raster_cells() {
            assert_eq!(cell.target, (8, 3).into());
            assert_eq!((cell.x, cell.y), (x - 1, y));
        }
    }
}
