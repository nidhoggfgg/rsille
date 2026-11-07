//! Wrapper configuration and terminal capability hints used by effect planning.

use crate::tui::visual_engine::math::sanitize_cell_aspect;

const LARGE_EFFECT_AREA_CELLS: u32 = 2_400;

/// Local configuration for a [`Visual`](crate::tui::widgets::Visual) wrapper.
///
/// Values set here override theme-level defaults for this single wrapper.
/// By default, cell aspect comes from the theme and performance settings use
/// [`VisualPerformanceConfig::default`].
///
/// ```
/// use rsille::tui::prelude::*;
///
/// let config = VisualConfig::default()
///     .cell_aspect(0.5)
///     .large_area_threshold(4_000)
///     .large_area_policy(LargeAreaPolicy::ReduceMotion);
/// let widget = visual(label::<()>("Hello")).config(config).fade_in();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct VisualConfig {
    cell_aspect: Option<f64>,
    performance: VisualPerformanceConfig,
}

impl VisualConfig {
    /// Override the terminal cell aspect used by geometry effects.
    ///
    /// `cell_aspect` is applied as `logical_x = cell_x * cell_aspect`.
    /// The default `None` means "use `Theme::effects.cell_aspect`".
    /// Finite values are bounded below by [`f64::EPSILON`]; non-finite values
    /// become `1.0`.
    pub fn cell_aspect(mut self, cell_aspect: f64) -> Self {
        self.cell_aspect = Some(sanitize_cell_aspect(cell_aspect));
        self
    }

    /// Return the local cell aspect override, if one was configured.
    pub fn cell_aspect_override(self) -> Option<f64> {
        self.cell_aspect
    }

    /// Override performance behavior for this wrapper.
    pub fn performance(mut self, performance: VisualPerformanceConfig) -> Self {
        self.performance = performance;
        self
    }

    /// Override the cell-count threshold where large-area degradation begins.
    /// Zero is normalized to one; degradation is considered above this threshold.
    pub fn large_area_threshold(mut self, cells: u32) -> Self {
        self.performance = self.performance.large_area_threshold(cells);
        self
    }

    /// Override the strategy used when the wrapped area exceeds the threshold.
    pub fn large_area_policy(mut self, policy: LargeAreaPolicy) -> Self {
        self.performance = self.performance.large_area_policy(policy);
        self
    }

    /// Return the local performance strategy.
    pub fn performance_config(self) -> VisualPerformanceConfig {
        self.performance
    }
}

/// Effective cell aspect and performance settings after resolving wrapper overrides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ResolvedVisualConfig {
    /// Sanitized local override or theme-provided cell aspect ratio.
    pub(crate) cell_aspect: f64,
    /// Performance policy selected for the wrapper.
    pub(crate) performance: VisualPerformanceConfig,
}

/// Controls how visual effects trade fidelity for cost.
///
/// Defaults to [`LargeAreaPolicy::ReduceMotion`] above 2,400 cells. Terminal
/// capability hints can lower the effective threshold. The area test uses the
/// wrapper's allocated rectangle, including blank cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisualPerformanceConfig {
    large_area_threshold: u32,
    large_area_policy: LargeAreaPolicy,
}

impl Default for VisualPerformanceConfig {
    fn default() -> Self {
        Self {
            large_area_threshold: LARGE_EFFECT_AREA_CELLS,
            large_area_policy: LargeAreaPolicy::ReduceMotion,
        }
    }
}

impl VisualPerformanceConfig {
    /// Number of cells after which large-area degradation is considered.
    /// Zero is normalized to one.
    pub fn large_area_threshold(mut self, cells: u32) -> Self {
        self.large_area_threshold = cells.max(1);
        self
    }

    /// Strategy used when the target area is larger than the configured limit.
    pub fn large_area_policy(mut self, policy: LargeAreaPolicy) -> Self {
        self.large_area_policy = policy;
        self
    }

    /// Return the configured cell-count threshold, before terminal capability limits.
    pub fn threshold(self) -> u32 {
        self.large_area_threshold
    }

    /// Return the strategy used when the area exceeds the effective threshold.
    pub fn policy(self) -> LargeAreaPolicy {
        self.large_area_policy
    }
}

/// Terminal feature hints used to degrade visual effects conservatively.
///
/// Defaults assume truecolor and Unicode block support, with no additional
/// cell-count limit. These are caller-provided hints, not terminal detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalVisualCapabilities {
    /// Whether generated effect colors may use RGB; otherwise they use basic colors.
    pub truecolor: bool,
    /// Whether typewriter cursors and blur approximations may use Unicode block glyphs.
    pub unicode_blocks: bool,
    /// Upper bound on the large-area threshold, normalized to at least one when applied.
    /// This selects when to consider degradation, not a hard rendering cell limit.
    pub max_effect_cells: u32,
}

impl Default for TerminalVisualCapabilities {
    fn default() -> Self {
        Self {
            truecolor: true,
            unicode_blocks: true,
            max_effect_cells: u32::MAX,
        }
    }
}

impl TerminalVisualCapabilities {
    /// Set whether generated effect colors may use truecolor.
    pub fn truecolor(mut self, truecolor: bool) -> Self {
        self.truecolor = truecolor;
        self
    }

    /// Set whether effects may use Unicode blocks instead of ASCII fallbacks.
    pub fn unicode_blocks(mut self, unicode_blocks: bool) -> Self {
        self.unicode_blocks = unicode_blocks;
        self
    }

    /// Cap the effective large-area threshold, normalizing zero to one.
    pub fn max_effect_cells(mut self, max_effect_cells: u32) -> Self {
        self.max_effect_cells = max_effect_cells.max(1);
        self
    }
}

/// Large-area degradation strategy.
///
/// Applies when the wrapper area strictly exceeds the effective threshold.
/// Global reduced-motion preferences still apply under [`Self::Preserve`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LargeAreaPolicy {
    /// Preserve the effect exactly even for large regions.
    Preserve,
    /// Keep cheap color/visibility work and replace expensive spatial/noise work.
    ReduceMotion,
    /// Skip all visual effects and copy only the child output.
    SkipEffects,
}
