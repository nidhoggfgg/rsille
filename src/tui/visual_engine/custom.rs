//! Extension interface for user-defined cell transformations.

use std::fmt;
use std::sync::Arc;

use crate::tui::visual_engine::{CellSample, VisualCtx, VisualEffectCost};

/// User-defined terminal-cell effect.
///
/// The trait is object-safe and intentionally cell-scoped: custom effects can
/// read [`VisualCtx`] for progress, theme, motion policy and stable randomness,
/// but they do not own widget state or emit application messages.
///
/// ```
/// use rsille::tui::prelude::*;
///
/// struct ShiftRight;
///
/// impl CellEffect for ShiftRight {
///     fn apply(&self, sample: &mut CellSample, ctx: VisualCtx<'_>) {
///         if !ctx.motion_policy.reduced_motion {
///             sample.dest_x += 2.0 * (1.0 - ctx.progress);
///         }
///     }
/// }
///
/// let widget = visual(label::<()>("Hello")).custom_effect(ShiftRight);
/// ```
pub trait CellEffect: Send + Sync + 'static {
    /// Transform a sampled cell in place using the current render context.
    ///
    /// Keep source coordinates intact and modify destination coordinates for
    /// geometry changes. Set `visible` to `false` to suppress output, or update
    /// `content` to change its glyph and style. Destinations are rounded to cell
    /// positions and clipped to the wrapper's area during blitting.
    ///
    /// Custom effects are retained during reduced-motion planning, so honor
    /// `ctx.motion_policy` here when introducing motion.
    fn apply(&self, sample: &mut CellSample, ctx: VisualCtx<'_>);

    /// Return the approximate per-cell cost; defaults to [`VisualEffectCost::Moderate`].
    fn estimated_cost(&self) -> VisualEffectCost {
        VisualEffectCost::Moderate
    }

    /// Whether the blitter may skip source cells equal to their previous-buffer snapshot.
    ///
    /// Defaults to `false`. Opt in only when skipping unchanged cells is safe;
    /// geometry, visibility, or time-dependent effects generally need every
    /// sample. A backdrop disables this optimization regardless of the result.
    fn can_use_dirty_only(&self) -> bool {
        false
    }
}

/// Named, reference-counted storage for a [`CellEffect`].
///
/// Clones share the same effect instance. Equality requires both an equal name
/// and the same shared allocation; independently constructed effects are unequal
/// even when their names and behavior match.
#[derive(Clone)]
pub struct CustomCellEffect {
    name: &'static str,
    pub(crate) effect: Arc<dyn CellEffect>,
}

impl CustomCellEffect {
    /// Store an effect with a static diagnostic name.
    pub fn new(name: &'static str, effect: impl CellEffect) -> Self {
        Self {
            name,
            effect: Arc::new(effect),
        }
    }

    /// Return the diagnostic name supplied at construction.
    pub fn name(&self) -> &'static str {
        self.name
    }
}

impl fmt::Debug for CustomCellEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CustomCellEffect")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl PartialEq for CustomCellEffect {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && Arc::ptr_eq(&self.effect, &other.effect)
    }
}
