//! Terminal-visible raster state, separate from the bounded encoding cache.

use std::io;

use crate::render::area::{Position, Size};
use crate::render::buffer::Buffer;

use super::output::{CellPixels, RasterOutput, RasterPolicy, RasterStats};
use super::RasterCell;

// Bound a single native frame as well as each encoded tile/cache. Oversized
// layouts remain usable through character output rather than unbounded staging.
const MAX_NATIVE_CELLS: usize = 4096;
const MAX_NATIVE_PIXELS: u64 = 16 * 1024 * 1024;
#[cfg(feature = "sixel")]
const MAX_FRAME_BYTES: usize = 32 * 1024 * 1024;

type VisibleCell = (u16, u16, RasterCell);
type EncodedCell = (u16, u16, std::sync::Arc<[u8]>);

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

    pub fn prepare(
        &mut self,
        buffer: &Buffer,
        origin: Position,
        inline: bool,
    ) -> io::Result<RasterPlan> {
        let mut metrics = self.config.resolve(
            buffer.has_raster(),
            buffer.has_unsupported_raster_composition(),
            self.alternate_screen,
            inline,
        )?;
        if let Some(pixels) = metrics {
            let count = buffer.raster_cells().count();
            let valid = count <= MAX_NATIVE_CELLS
                && count as u64 * u64::from(pixels.width) * u64::from(pixels.height)
                    <= MAX_NATIVE_PIXELS
                && origin.x.checked_add(buffer.size().width).is_some()
                && origin.y.checked_add(buffer.size().height).is_some();
            if !valid {
                if self.config.policy == RasterPolicy::RequireSixel {
                    return Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        "native raster frame exceeds resource or coordinate limits",
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
        let clear = self.uncertain
            || (!self.visible.is_empty()
                && (old_changed
                    || metrics != self.metrics
                    || origin != self.origin
                    || buffer.size() != self.size
                    || buffer.previous().is_none()));
        let mut plan = RasterPlan {
            native: metrics.is_some(),
            clear,
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
                let data = self.cache.encode(cell, pixels.width, pixels.height)?;
                bytes = bytes.saturating_add(data.len());
                if bytes > MAX_FRAME_BYTES {
                    if self.config.policy == RasterPolicy::RequireSixel {
                        return Err(io::Error::new(
                            io::ErrorKind::Unsupported,
                            "encoded raster frame exceeds 32 MiB",
                        ));
                    }
                    plan.native = false;
                    plan.metrics = None;
                    plan.visible.clear();
                    plan.encoded.clear();
                    plan.clear |= !self.visible.is_empty();
                    break;
                }
                plan.encoded.push((*x, *y, data));
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
        let mut buffer = Buffer::new((width, 1).into());
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
                    no_scroll: true,
                    cell_pixels: Some(CellPixels { width, height }),
                },
            },
            true,
        )
    }

    #[test]
    fn native_frame_limits_are_checked_before_any_encoding() {
        for (cells, pixel_width, pixel_height) in [(4097, 1, 6), (300, 1000, 60)] {
            let buffer = frame(cells);
            for policy in [RasterPolicy::Auto, RasterPolicy::RequireSixel] {
                let mut presenter = presenter(policy, pixel_width, pixel_height);
                let plan = presenter.prepare(&buffer, Position::default(), false);
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
    fn coordinate_overflow_falls_back_before_encoding() {
        let mut presenter = presenter(RasterPolicy::Auto, 8, 18);
        let plan = presenter
            .prepare(&frame(2), (u16::MAX, 0).into(), false)
            .unwrap();
        assert!(!plan.native);
        assert_eq!(presenter.stats().encodes, 0);
    }

    #[test]
    fn resource_limit_fallback_clears_previously_visible_graphics() {
        let mut presenter = presenter(RasterPolicy::Auto, 8, 18);
        let first = presenter
            .prepare(&frame(1), Position::default(), false)
            .unwrap();
        assert!(first.native);
        presenter.begin(&first);
        presenter.commit(first);
        let plan = presenter
            .prepare(&frame(4097), Position::default(), false)
            .unwrap();
        assert!(!plan.native);
        assert!(plan.clear);
        assert!(plan.encoded.is_empty());
    }
}
