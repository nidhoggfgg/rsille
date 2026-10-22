use std::time::Duration;

/// Approximate per-cell cost classification for visual effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualEffectCost {
    Cheap,
    Moderate,
    Expensive,
}

impl VisualEffectCost {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VisualProfile {
    pub offscreen_render_time: Duration,
    pub effect_apply_time: Duration,
    pub processed_cells: u64,
    pub skipped_cells: u64,
    pub blank_cells_skipped: u64,
    pub clean_cells_skipped: u64,
    pub degradation: VisualDegradation,
}

/// Degradation flags active for one visual render pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VisualDegradation {
    pub large_area: bool,
    pub reduced_motion: bool,
    pub skipped_effects: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct BlitReport {
    pub(crate) effect_apply_time: Duration,
    pub(crate) processed_cells: u64,
    pub(crate) skipped_cells: u64,
    pub(crate) blank_cells_skipped: u64,
    pub(crate) clean_cells_skipped: u64,
    pub(crate) degradation: VisualDegradation,
}

impl BlitReport {
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

    pub(crate) fn add_degradation(&mut self, degradation: VisualDegradation) {
        self.degradation.large_area |= degradation.large_area;
        self.degradation.reduced_motion |= degradation.reduced_motion;
        self.degradation.skipped_effects |= degradation.skipped_effects;
    }
}
