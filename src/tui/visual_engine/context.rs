use std::time::Instant;

use crate::render::area::Area;

use crate::tui::animation::MotionPolicy;
use crate::tui::style::Theme;
use crate::tui::visual_engine::config::{TerminalVisualCapabilities, VisualPerformanceConfig};

#[derive(Debug, Clone, Copy)]
pub struct VisualCtx<'a> {
    /// Effect progress in the inclusive range `0.0..=1.0`.
    pub progress: f64,
    /// Logical area occupied by the wrapped widget.
    pub area: Area,
    /// Runtime render instant for this frame.
    pub now: Instant,
    /// Runtime render frame number for this frame.
    pub frame: u64,
    /// Terminal cell width divided by cell height. Defaults to `1.0` until
    /// terminal-specific configuration is introduced.
    pub cell_aspect: f64,
    /// Global motion behavior active for this render pass.
    pub motion_policy: MotionPolicy,
    /// Theme active for this render pass.
    pub theme: &'a Theme,
    /// Optional named entry point for future theme effect presets.
    pub effect_preset: Option<&'a str>,
    /// Stable seed available to deterministic effects.
    pub seed: u64,
    /// Performance and degradation strategy active for this wrapper.
    pub performance: VisualPerformanceConfig,
    /// Terminal feature hints active for this render pass.
    pub capabilities: TerminalVisualCapabilities,
}

impl<'a> VisualCtx<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        progress: f64,
        area: Area,
        now: Instant,
        frame: u64,
        cell_aspect: f64,
        motion_policy: MotionPolicy,
        theme: &'a Theme,
        effect_preset: Option<&'a str>,
        seed: u64,
    ) -> Self {
        Self {
            progress: progress.clamp(0.0, 1.0),
            area,
            now,
            frame,
            cell_aspect: cell_aspect.max(f64::EPSILON),
            motion_policy,
            theme,
            effect_preset,
            seed,
            performance: VisualPerformanceConfig::default(),
            capabilities: TerminalVisualCapabilities::default(),
        }
    }

    /// Convert a cell-space x coordinate into aspect-corrected logical space.
    pub fn logical_x(self, cell_x: f64) -> f64 {
        cell_x * self.cell_aspect
    }

    /// Convert a cell-space point into aspect-corrected logical space.
    pub fn logical_point(self, cell_x: f64, cell_y: f64) -> (f64, f64) {
        (self.logical_x(cell_x), cell_y)
    }

    /// Scale a horizontal cell offset by the active cell aspect.
    pub fn aspect_adjusted_x_offset(self, cell_dx: f64) -> f64 {
        cell_dx * self.cell_aspect
    }

    /// Measure the logical distance between two local cell-space points.
    pub fn logical_distance(self, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
        let (ax, ay) = self.logical_point(ax, ay);
        let (bx, by) = self.logical_point(bx, by);
        ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt()
    }

    /// Whether area-sensitive effects should prefer cheaper reduced behavior.
    pub fn is_large_area(self) -> bool {
        self.area.width() as u32 * self.area.height() as u32 > self.performance.threshold()
    }

    pub(crate) fn with_progress(self, progress: f64) -> Self {
        Self {
            progress: progress.clamp(0.0, 1.0),
            ..self
        }
    }

    pub(crate) fn with_performance(self, performance: VisualPerformanceConfig) -> Self {
        Self {
            performance,
            ..self
        }
    }

    pub(crate) fn with_capabilities(self, capabilities: TerminalVisualCapabilities) -> Self {
        let threshold = self
            .performance
            .threshold()
            .min(capabilities.max_effect_cells.max(1));
        Self {
            capabilities,
            performance: self.performance.large_area_threshold(threshold),
            ..self
        }
    }
}
