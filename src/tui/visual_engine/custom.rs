use std::fmt;
use std::sync::Arc;

use crate::tui::visual_engine::{CellSample, VisualCtx, VisualEffectCost};

/// User-defined terminal-cell effect.
///
/// The trait is object-safe and intentionally cell-scoped: custom effects can
/// read [`VisualCtx`] for progress, theme, motion policy and stable randomness,
/// but they do not own widget state or emit application messages.
pub trait CellEffect: Send + Sync + 'static {
    fn apply(&self, sample: &mut CellSample, ctx: VisualCtx<'_>);

    fn estimated_cost(&self) -> VisualEffectCost {
        VisualEffectCost::Moderate
    }

    fn can_use_dirty_only(&self) -> bool {
        false
    }
}

#[derive(Clone)]
pub struct CustomCellEffect {
    name: &'static str,
    pub(crate) effect: Arc<dyn CellEffect>,
}

impl CustomCellEffect {
    pub fn new(name: &'static str, effect: impl CellEffect) -> Self {
        Self {
            name,
            effect: Arc::new(effect),
        }
    }

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
