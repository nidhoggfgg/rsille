//! Terminal-visible raster state, separate from the bounded encoding cache.
//!
//! A cache hit does not prove that a tile is still displayed. Preflight stages
//! output before any frame bytes; visible state is committed only after flush.
//! Changed old coverage requires mode-specific erasure and repaint, even when
//! all replacement tile bytes can be reused.

use std::io;

use crate::area::{Position, Size};
use crate::buffer::Buffer;

use super::output::{CellPixels, RasterOutput, RasterStats};
use super::RasterCell;

#[cfg(feature = "sixel")]
use super::presenter::MAX_FRAME_BYTES;
#[cfg(all(test, feature = "sixel"))]
use super::presenter::MAX_NATIVE_CELLS;
use super::presenter::{check_limits, EncodedCell, Payload, RasterPlan};
type VisibleCell = (u16, u16, RasterCell);

// Move only the DCS anchor, not the cell's source crop. Leading unset bits
// preserve text/graphics above it. Within six cell rows a band boundary aligns
// for every integral cell height; very short viewports may have no safe anchor.
fn anchor_shift(y: u16, rows: u16, cell_height: u16) -> Option<u16> {
    (0..=y.min(5)).find(|&shift| {
        let image_height = u32::from(shift + 1) * u32::from(cell_height);
        let available = u32::from(rows - y + shift) * u32::from(cell_height);
        image_height.div_ceil(6) * 6 <= available
    })
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SixelPresenter {
    pub config: RasterOutput,
    pub alternate_screen: bool,
    visible: Vec<VisibleCell>,
    overlays: bool,
    metrics: Option<CellPixels>,
    origin: Position,
    size: Size,
    // Set before native I/O, cleared only after a successful flush. Even a
    // failed first frame may have left graphics on the terminal.
    uncertain: bool,
    #[cfg(feature = "sixel")]
    cache: super::sixel::SixelCache,
    tiles_sent: u64,
    bytes_sent: u64,
}

pub(super) struct SixelFrame {
    pub encoded: Vec<EncodedCell>,
    visible: Vec<VisibleCell>,
    overlays: bool,
    metrics: Option<CellPixels>,
    origin: Position,
    size: Size,
}

impl SixelPresenter {
    pub fn new(config: RasterOutput, alternate_screen: bool) -> Self {
        Self {
            config,
            alternate_screen,
            ..Self::default()
        }
    }

    pub fn inline_padding_rows(&self, buffer: &Buffer) -> io::Result<u16> {
        self.config
            .resolve(
                buffer.has_raster(),
                buffer.has_unsupported_raster_composition(),
                self.alternate_screen,
                true,
            )
            .map(|metrics| metrics.map_or(0, CellPixels::inline_padding_rows))
    }

    pub fn prepare(
        &mut self,
        buffer: &Buffer,
        origin: Position,
        inline: bool,
        append_newline: bool,
        inline_shrank: bool,
    ) -> io::Result<RasterPlan> {
        let metrics = self.config.resolve(
            buffer.has_raster(),
            buffer.has_unsupported_raster_composition(),
            self.alternate_screen,
            inline,
        )?;
        if metrics.is_some() && append_newline {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "native Sixel does not support appended-newline output",
            ));
        }
        if let Some(pixels) = metrics {
            check_limits(buffer, origin, pixels)?;
        }
        if let Some(pixels) = metrics {
            if !inline
                && buffer
                    .raster_cells()
                    .chain(buffer.overlay_cells())
                    .any(|(_, y, _)| anchor_shift(y, buffer.size().height, pixels.height).is_none())
            {
                return Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        "fullscreen viewport is too short to position a complete Sixel band without scrolling",
                    ));
            }
        }
        let visible: Vec<_> = if metrics.is_some() {
            buffer
                .raster_cells()
                .map(|(x, y, cell)| (x, y, cell.clone()))
                .collect()
        } else {
            Vec::new()
        };
        // Both vectors are row-major. A merge scan detects erased/changed old
        // coverage in linear time; adding a disjoint new tile needs no erase.
        let mut current = visible.iter().peekable();
        let old_changed = self.visible.iter().any(|old| {
            while current
                .peek()
                .is_some_and(|new| (new.1, new.0) < (old.1, old.0))
            {
                current.next();
            }
            current.peek().is_none_or(|new| *new != old)
        });
        // Shrinking can abandon text-only rows while all raster cells stay
        // unchanged. Erase those rows and repaint surviving tiles as well.
        let overlays = buffer.overlay_cells().next().is_some();
        let clear = self.uncertain
            || (self.overlays
                && (buffer.previous().is_none()
                    || origin != self.origin
                    || buffer.size() != self.size
                    || (metrics.is_some() && metrics != self.metrics)))
            || (inline_shrank && (metrics.is_some() || self.overlays))
            || (!self.visible.is_empty()
                && (old_changed
                    || metrics != self.metrics
                    || origin != self.origin
                    || buffer.size() != self.size
                    || buffer.previous().is_none()));
        let mut plan = RasterPlan {
            native: metrics.is_some(),
            overlay_frame: overlays || self.overlays,
            damage_pixels: metrics.or(self.metrics),
            clear,
            inline_padding: if inline {
                metrics.map_or(0, CellPixels::inline_padding_rows)
            } else {
                0
            },
            payload: Payload::Sixel(SixelFrame {
                overlays,
                encoded: Vec::new(),
                visible,
                metrics,
                origin,
                size: buffer.size(),
            }),
        };
        let Payload::Sixel(frame) = &mut plan.payload else {
            unreachable!()
        };
        #[cfg(feature = "sixel")]
        if let Some(pixels) = metrics {
            let mut bytes = 0usize;
            // Every glyph we repaint can cover all tiles across its column span.
            // Replay those tiles bottom-to-top, even if their samples are unchanged.
            let mut damaged = std::collections::BTreeSet::new();
            if plan.overlay_frame && !clear {
                if let Some(diff) = buffer.native_diff(plan.damage_pixels) {
                    for (x, y, base) in diff {
                        for dx in 0..base.width() {
                            damaged.insert((x + dx as u16, y));
                        }
                    }
                }
            }
            for (x, y, cell, overlay) in frame
                .visible
                .iter()
                .map(|(x, y, c)| (*x, *y, c, false))
                .chain(buffer.overlay_cells().map(|(x, y, c)| (x, y, c, true)))
            {
                if !clear
                    && self.metrics == metrics
                    && buffer.previous().is_some()
                    && !damaged.contains(&(x, y))
                    && (overlay || buffer.previous_raster_at((x, y).into()) == Some(cell))
                {
                    continue;
                }
                let shift = if inline {
                    0 // Inline already reserves trailing guard rows.
                } else {
                    anchor_shift(y, buffer.size().height, pixels.height)
                        .expect("anchors validated before encoding")
                };
                let data = self.cache.encode_padded(
                    cell,
                    pixels.width,
                    pixels.height,
                    shift * pixels.height,
                    (!overlay).then_some(cell.options.background),
                )?;
                bytes = bytes.saturating_add(data.len());
                if bytes > MAX_FRAME_BYTES {
                    return Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        "encoded raster frame exceeds 32 MiB",
                    ));
                }
                frame.encoded.push((x, y - shift, data));
            }
        }
        // Frames without raster cells do not allocate source or encoded data.
        #[cfg(not(feature = "sixel"))]
        let _ = frame;
        Ok(plan)
    }

    pub fn begin(&mut self, plan: &RasterPlan) {
        self.uncertain |= plan.native || plan.clear || plan.overlay_frame;
    }

    pub fn commit(&mut self, plan: RasterPlan) {
        let Payload::Sixel(frame) = plan.payload else {
            unreachable!()
        };
        self.tiles_sent = self.tiles_sent.saturating_add(frame.encoded.len() as u64);
        self.bytes_sent = self.bytes_sent.saturating_add(
            frame
                .encoded
                .iter()
                .map(|(_, _, bytes)| bytes.len() as u64)
                .sum::<u64>(),
        );
        self.overlays = frame.overlays;
        self.visible = frame.visible;
        self.metrics = frame.metrics;
        self.origin = frame.origin;
        self.size = frame.size;
        self.uncertain = false;
    }

    // Transfer ownership of a successfully displayed region to terminal history.
    // Keep the bounded encoding cache and counters, but never erase old coverage.
    pub fn forget_region(&mut self) {
        self.overlays = false;
        self.visible.clear();
        self.metrics = None;
        self.origin = Position::default();
        self.size = Size::default();
        self.uncertain = false;
    }

    pub fn stats(&self) -> RasterStats {
        let stats = RasterStats {
            tiles_sent: self.tiles_sent,
            bytes_sent: self.bytes_sent,
            ..Default::default()
        };
        #[cfg(feature = "sixel")]
        let stats = RasterStats {
            encodes: self.cache.encodes(),
            cache_hits: self.cache.hits(),
            cache_bytes: self.cache.bytes(),
            cache_entries: self.cache.len(),
            ..stats
        };
        stats
    }

    pub fn needs_protocol_recovery(&self) -> bool {
        self.uncertain
    }
}

#[cfg(all(test, feature = "sixel"))]
mod tests {
    use super::*;
    use crate::raster::output::RasterCapabilities;
    use crate::raster::{RasterCanvas, RasterOptions, Rgb};

    fn frame(width: u16) -> Buffer {
        let mut buffer = Buffer::new((width, 3).into());
        let canvas = RasterCanvas::new(1, 1, Rgb(0, 0, 0)).unwrap();
        for x in 0..width {
            buffer
                .set_raster(
                    (x, 0).into(),
                    RasterCell {
                        canvas: canvas.clone(),
                        target: buffer.size(),
                        x,
                        y: 0,
                        options: RasterOptions::default(),
                    },
                )
                .unwrap();
        }
        buffer
    }

    fn presenter(width: u16, height: u16) -> SixelPresenter {
        SixelPresenter::new(
            RasterOutput {
                capabilities: RasterCapabilities {
                    sixel: true,
                    reliable_erase: true,
                    reliable_inline_erase: false,
                    no_scroll: true,
                    cell_pixels: Some(CellPixels { width, height }),
                },
            },
            true,
        )
    }

    #[test]
    fn native_frame_limits_are_checked_before_any_encoding() {
        for (cells, pixel_width, pixel_height) in
            [((MAX_NATIVE_CELLS + 1) as u16, 1, 6), (300, 1000, 60)]
        {
            let buffer = frame(cells);

            let mut presenter = presenter(pixel_width, pixel_height);
            let plan = presenter.prepare(&buffer, Position::default(), false, false, false);

            assert_eq!(plan.err().unwrap().kind(), io::ErrorKind::Unsupported);

            assert_eq!(presenter.stats(), RasterStats::default());
        }
    }

    #[test]
    fn million_pixel_frame_and_cell_budget_boundary_use_native_output() {
        for cells in [5000, MAX_NATIVE_CELLS as u16] {
            let mut presenter = presenter(10, 20);
            let plan = presenter
                .prepare(&frame(cells), Position::default(), false, false, false)
                .unwrap();
            assert!(plan.native);
            let Payload::Sixel(frame) = plan.payload else {
                unreachable!()
            };
            assert_eq!(frame.encoded.len(), usize::from(cells));
            assert_eq!(frame.visible.len(), usize::from(cells));
        }
    }

    #[test]
    fn strict_limit_error_reports_usage_and_limits() {
        let mut presenter = presenter(10, 20);
        let error = presenter
            .prepare(
                &frame((MAX_NATIVE_CELLS + 1) as u16),
                Position::default(),
                false,
                false,
                false,
            )
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains("32769 cells (limit 32768)"));
        assert!(error.contains("6553800 physical pixels (limit 16777216)"));
        assert!(error.contains("cell pixels 10x20, coordinates valid: true"));
    }

    #[test]
    fn encoded_frame_limit_rejects_without_committing_graphics() {
        let mut canvas = RasterCanvas::new(24, 16, Rgb(0, 0, 0)).unwrap();
        for y in 0..16 {
            for x in 0..24 {
                let value = (y * 24 + x) as u8;
                canvas.set_pixel(
                    x,
                    y,
                    Rgb(value, value.wrapping_mul(71), value.wrapping_mul(137)),
                );
            }
        }
        let cell = RasterCell {
            canvas,
            target: (1, 1).into(),
            x: 0,
            y: 0,
            options: RasterOptions::default(),
        };
        let mut buffer = Buffer::new((MAX_NATIVE_CELLS as u16, 3).into());
        for x in 0..MAX_NATIVE_CELLS as u16 {
            buffer.set_raster((x, 0).into(), cell.clone()).unwrap();
        }
        let mut presenter = presenter(24, 16);
        let error = presenter
            .prepare(&buffer, Position::default(), false, false, false)
            .err()
            .unwrap();
        assert_eq!(error.kind(), io::ErrorKind::Unsupported);
        assert!(error.to_string().contains("encoded raster frame exceeds"));
        assert!(presenter.visible.is_empty());
        assert!(!presenter.needs_protocol_recovery());
        assert_eq!(presenter.stats().tiles_sent, 0);
        assert_eq!(presenter.stats().bytes_sent, 0);
    }

    #[test]
    fn coordinate_overflow_is_rejected_before_encoding() {
        let mut presenter = presenter(8, 18);
        let error = presenter
            .prepare(&frame(2), (u16::MAX, 0).into(), false, false, false)
            .err()
            .unwrap();
        assert_eq!(error.kind(), io::ErrorKind::Unsupported);
        assert_eq!(presenter.stats().encodes, 0);
    }

    #[test]
    fn resource_limit_rejection_preserves_previously_visible_graphics() {
        let mut presenter = presenter(8, 18);
        let first = presenter
            .prepare(&frame(1), Position::default(), false, false, false)
            .unwrap();
        presenter.begin(&first);
        presenter.commit(first);
        let before = presenter.stats();
        assert!(presenter
            .prepare(
                &frame((MAX_NATIVE_CELLS + 1) as u16),
                Position::default(),
                false,
                false,
                false
            )
            .is_err());
        assert_eq!(presenter.stats(), before);
        assert_eq!(presenter.visible.len(), 1);
        let empty = Buffer::new((1, 3).into());
        let cleanup = presenter
            .prepare(&empty, Position::default(), false, false, false)
            .unwrap();
        assert!(cleanup.clear);
        assert!(!cleanup.native);
    }
}
