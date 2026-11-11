//! Terminal-visible raster state, separate from the bounded encoding cache.

use std::io;

use crate::render::area::{Position, Size};
use crate::render::buffer::Buffer;

use super::output::{CellPixels, RasterOutput, RasterPolicy, RasterStats};
use super::RasterCell;

// Bound a single native frame as well as each encoded tile/cache. Oversized
// layouts remain usable through character output rather than unbounded staging.
const MAX_NATIVE_CELLS: usize = 32 * 1024;
const MAX_NATIVE_PIXELS: u64 = 16 * 1024 * 1024;
#[cfg(feature = "sixel")]
const MAX_FRAME_BYTES: usize = 32 * 1024 * 1024;

type VisibleCell = (u16, u16, RasterCell);
type EncodedCell = (u16, u16, std::sync::Arc<[u8]>);

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
pub(in crate::render) struct RasterPresenter {
    pub config: RasterOutput,
    pub alternate_screen: bool,
    visible: Vec<VisibleCell>,
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

pub(in crate::render) struct RasterPlan {
    pub native: bool,
    pub clear: bool,
    pub inline_padding: u16,
    pub encoded: Vec<EncodedCell>,
    visible: Vec<VisibleCell>,
    metrics: Option<CellPixels>,
    origin: Position,
    size: Size,
}

impl RasterPresenter {
    pub fn new(config: RasterOutput, alternate_screen: bool) -> Self {
        Self {
            config,
            alternate_screen,
            ..Self::default()
        }
    }

    pub fn inline_padding_rows(&self) -> u16 {
        self.config
            .resolve(true, false, self.alternate_screen, true)
            .ok()
            .flatten()
            .map_or(0, CellPixels::inline_padding_rows)
    }

    pub fn prepare(
        &mut self,
        buffer: &Buffer,
        origin: Position,
        inline: bool,
        append_newline: bool,
        inline_shrank: bool,
    ) -> io::Result<RasterPlan> {
        let mut metrics = self.config.resolve(
            buffer.has_raster(),
            buffer.has_unsupported_raster_composition(),
            self.alternate_screen,
            inline,
        )?;
        if metrics.is_some() && append_newline {
            if self.config.policy == RasterPolicy::RequireSixel {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "native Sixel does not support appended-newline output",
                ));
            }
            metrics = None;
        }
        if let Some(pixels) = metrics {
            let count = buffer.raster_cells().count();
            let pixel_count = count as u64 * u64::from(pixels.width) * u64::from(pixels.height);
            let coordinates_valid = origin.x.checked_add(buffer.size().width).is_some()
                && origin.y.checked_add(buffer.size().height).is_some();
            let valid =
                count <= MAX_NATIVE_CELLS && pixel_count <= MAX_NATIVE_PIXELS && coordinates_valid;
            if !valid {
                if self.config.policy == RasterPolicy::RequireSixel {
                    return Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        format!(
                            "native raster frame exceeds resource or coordinate limits: \
                             {count} cells (limit {MAX_NATIVE_CELLS}), \
                             {pixel_count} physical pixels (limit {MAX_NATIVE_PIXELS}), \
                             cell pixels {}x{}, coordinates valid: {coordinates_valid}",
                            pixels.width, pixels.height,
                        ),
                    ));
                }
                metrics = None;
            }
        }
        if let Some(pixels) = metrics {
            if !inline
                && buffer
                    .raster_cells()
                    .any(|(_, y, _)| anchor_shift(y, buffer.size().height, pixels.height).is_none())
            {
                if self.config.policy == RasterPolicy::RequireSixel {
                    return Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        "fullscreen viewport is too short to position a complete Sixel band without scrolling",
                    ));
                }
                metrics = None;
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
        let clear = self.uncertain
            || (inline_shrank && metrics.is_some())
            || (!self.visible.is_empty()
                && (old_changed
                    || metrics != self.metrics
                    || origin != self.origin
                    || buffer.size() != self.size
                    || buffer.previous().is_none()));
        let mut plan = RasterPlan {
            native: metrics.is_some(),
            clear,
            inline_padding: if inline {
                metrics.map_or(0, CellPixels::inline_padding_rows)
            } else {
                0
            },
            encoded: Vec::new(),
            visible,
            metrics,
            origin,
            size: buffer.size(),
        };
        #[cfg(feature = "sixel")]
        if let Some(pixels) = metrics {
            let mut bytes = 0usize;
            for (x, y, cell) in &plan.visible {
                if !clear
                    && self.metrics == metrics
                    && buffer.previous_raster_at((*x, *y).into()) == Some(cell)
                {
                    continue;
                }
                let shift = if inline {
                    0 // Inline already reserves trailing guard rows.
                } else {
                    anchor_shift(*y, buffer.size().height, pixels.height)
                        .expect("anchors validated before encoding")
                };
                let data = self.cache.encode_padded(
                    cell,
                    pixels.width,
                    pixels.height,
                    shift * pixels.height,
                )?;
                bytes = bytes.saturating_add(data.len());
                if bytes > MAX_FRAME_BYTES {
                    if self.config.policy == RasterPolicy::RequireSixel {
                        return Err(io::Error::new(
                            io::ErrorKind::Unsupported,
                            "encoded raster frame exceeds 32 MiB",
                        ));
                    }
                    plan.native = false;
                    plan.inline_padding = 0;
                    plan.metrics = None;
                    plan.visible.clear();
                    plan.encoded.clear();
                    plan.clear |= !self.visible.is_empty();
                    break;
                }
                plan.encoded.push((*x, *y - shift, data));
            }
        }
        // No source/cached encoded allocation is performed on the text-only path.
        #[cfg(not(feature = "sixel"))]
        let _ = &mut plan;
        Ok(plan)
    }

    pub fn begin(&mut self, plan: &RasterPlan) {
        self.uncertain |= plan.native || plan.clear;
    }

    pub fn commit(&mut self, plan: RasterPlan) {
        self.tiles_sent = self.tiles_sent.saturating_add(plan.encoded.len() as u64);
        self.bytes_sent = self.bytes_sent.saturating_add(
            plan.encoded
                .iter()
                .map(|(_, _, bytes)| bytes.len() as u64)
                .sum::<u64>(),
        );
        self.visible = plan.visible;
        self.metrics = plan.metrics;
        self.origin = plan.origin;
        self.size = plan.size;
        self.uncertain = false;
    }

    // Transfer ownership of a successfully displayed region to terminal history.
    // Keep the bounded encoding cache and counters, but never erase old coverage.
    pub fn forget_region(&mut self) {
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
    use crate::render::raster::output::RasterCapabilities;
    use crate::render::raster::{RasterCanvas, RasterOptions, Rgb};

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

    fn presenter(policy: RasterPolicy, width: u16, height: u16) -> RasterPresenter {
        RasterPresenter::new(
            RasterOutput {
                policy,
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
            for policy in [RasterPolicy::Auto, RasterPolicy::RequireSixel] {
                let mut presenter = presenter(policy, pixel_width, pixel_height);
                let plan = presenter.prepare(&buffer, Position::default(), false, false, false);
                if policy == RasterPolicy::Auto {
                    let plan = plan.unwrap();
                    assert!(!plan.native);
                    assert!(plan.encoded.is_empty());
                } else {
                    assert_eq!(plan.err().unwrap().kind(), io::ErrorKind::Unsupported);
                }
                assert_eq!(presenter.stats(), RasterStats::default());
            }
        }
    }

    #[test]
    fn million_pixel_frame_and_cell_budget_boundary_use_native_output() {
        for cells in [5000, MAX_NATIVE_CELLS as u16] {
            for policy in [RasterPolicy::Auto, RasterPolicy::RequireSixel] {
                let mut presenter = presenter(policy, 10, 20);
                let plan = presenter
                    .prepare(&frame(cells), Position::default(), false, false, false)
                    .unwrap();
                assert!(plan.native);
                assert_eq!(plan.encoded.len(), usize::from(cells));
                assert_eq!(plan.visible.len(), usize::from(cells));
            }
        }
    }

    #[test]
    fn strict_limit_error_reports_usage_and_limits() {
        let mut presenter = presenter(RasterPolicy::RequireSixel, 10, 20);
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
    fn coordinate_overflow_falls_back_before_encoding() {
        let mut presenter = presenter(RasterPolicy::Auto, 8, 18);
        let plan = presenter
            .prepare(&frame(2), (u16::MAX, 0).into(), false, false, false)
            .unwrap();
        assert!(!plan.native);
        assert_eq!(presenter.stats().encodes, 0);
    }

    #[test]
    fn resource_limit_fallback_clears_previously_visible_graphics() {
        let mut presenter = presenter(RasterPolicy::Auto, 8, 18);
        let first = presenter
            .prepare(&frame(1), Position::default(), false, false, false)
            .unwrap();
        assert!(first.native);
        presenter.begin(&first);
        presenter.commit(first);
        let plan = presenter
            .prepare(
                &frame((MAX_NATIVE_CELLS + 1) as u16),
                Position::default(),
                false,
                false,
                false,
            )
            .unwrap();
        assert!(!plan.native);
        assert!(plan.clear);
        assert!(plan.encoded.is_empty());
    }
}
