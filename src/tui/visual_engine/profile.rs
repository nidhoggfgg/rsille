//! Cost hints, degradation decisions, and render-pass measurements.

use std::time::Duration;

/// Approximate per-cell cost classification for visual effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualEffectCost {
    /// Lightweight visibility tests or color updates.
    Cheap,
    /// Geometry, blending, or composition work of intermediate complexity.
    Moderate,
    /// Noise-heavy scattering, dissolving, or glyph corruption.
    Expensive,
}

impl VisualEffectCost {
    /// Return the more expensive category without adding individual costs.
    pub(crate) fn max(self, other: Self) -> Self {
        use VisualEffectCost::{Cheap, Expensive, Moderate};
        match (self, other) {
            (Expensive, _) | (_, Expensive) => Expensive,
            (Moderate, _) | (_, Moderate) => Moderate,
            _ => Cheap,
        }
    }
}

/// Profiling data emitted after one visual render pass.
///
/// Register a callback with [`Visual::profile`](crate::tui::widgets::Visual::profile).
/// Counters describe sampled source cells, not unique destination positions;
/// multiple samples can map to the same destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VisualProfile {
    /// Time spent rendering the child into its offscreen buffer.
    pub offscreen_render_time: Duration,
    /// Time spent planning effects, sampling any backdrop, applying effects, and blitting.
    pub effect_apply_time: Duration,
    /// Visible, in-area samples submitted to the destination chunk.
    pub processed_cells: u64,
    /// Samples rejected by visibility, destination bounds, or backdrop coverage.
    /// Excludes the blank and clean cells counted separately below.
    pub skipped_cells: u64,
    /// Blank cells and wide-cell continuation positions skipped before effect evaluation.
    pub blank_cells_skipped: u64,
    /// Source cells skipped because they matched the previous-buffer snapshot.
    pub clean_cells_skipped: u64,
    /// Degradation conditions combined across all effect groups in the pass.
    pub degradation: VisualDegradation,
}

/// Degradation flags active for one visual render pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VisualDegradation {
    /// The allocated area exceeded the effective threshold, even if effects were preserved.
    pub large_area: bool,
    /// Reduced behavior was requested by motion policy or large-area policy.
    /// This may also be true when all effects were skipped.
    pub reduced_motion: bool,
    /// Large-area policy removed all visual effects from at least one planned group.
    pub skipped_effects: bool,
}

/// Internal blit counters and timing before adding the child-render duration.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct BlitReport {
    /// Elapsed blit-pass time, or zero when profiling is disabled.
    pub(crate) effect_apply_time: Duration,
    /// Visible, in-area samples submitted to the chunk.
    pub(crate) processed_cells: u64,
    /// Samples rejected after sampling, excluding blank and clean skips.
    pub(crate) skipped_cells: u64,
    /// Blank cells and wide-cell continuation positions excluded before sampling.
    pub(crate) blank_cells_skipped: u64,
    /// Source cells excluded by the dirty-only optimization.
    pub(crate) clean_cells_skipped: u64,
    /// Combined planning decisions for this pass.
    pub(crate) degradation: VisualDegradation,
}

impl BlitReport {
    /// Combine blit measurements with the separately measured child-render duration.
    pub(crate) fn into_profile(self, offscreen_render_time: Duration) -> VisualProfile {
        VisualProfile {
            offscreen_render_time,
            effect_apply_time: self.effect_apply_time,
            processed_cells: self.processed_cells,
            skipped_cells: self.skipped_cells,
            blank_cells_skipped: self.blank_cells_skipped,
            clean_cells_skipped: self.clean_cells_skipped,
            degradation: self.degradation,
        }
    }

    /// Merge another group's flags so any active degradation is retained.
    pub(crate) fn add_degradation(&mut self, degradation: VisualDegradation) {
        self.degradation.large_area |= degradation.large_area;
        self.degradation.reduced_motion |= degradation.reduced_motion;
        self.degradation.skipped_effects |= degradation.skipped_effects;
    }
}
