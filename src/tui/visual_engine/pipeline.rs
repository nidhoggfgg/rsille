//! Resolve animation progress and select effects for the active rendering policy.

use crate::tui::animation::MotionPolicy;
use crate::tui::visual_engine::{LargeAreaPolicy, VisualCtx, VisualDegradation, VisualEffect};

/// Effects sharing a progress value resolved by the widget wrapper.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedEffectGroup {
    /// Progress before applying the global motion policy.
    pub(crate) progress: f64,
    /// Effects in application order, before degradation planning.
    pub(crate) effects: Vec<VisualEffect>,
}

/// A planned group with its own progress-adjusted render context.
#[derive(Debug, Clone)]
pub(crate) struct EffectiveEffectGroup<'a> {
    /// Context including policy-adjusted progress for this group.
    pub(crate) ctx: VisualCtx<'a>,
    /// Effects retained or substituted by degradation planning.
    pub(crate) effects: Vec<VisualEffect>,
}

/// Use the last lifecycle frame's progress, or `fallback` when no frame exists.
/// The result is clamped to `0.0..=1.0`.
pub(crate) fn lifecycle_progress(
    frames: &[crate::tui::animation::TimelineFrame],
    fallback: f64,
) -> f64 {
    frames
        .last()
        .map(|frame| frame.progress)
        .unwrap_or(fallback)
        .clamp(0.0, 1.0)
}

/// Planned effects and the conditions that selected them.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EffectPlan {
    /// Effective transforms in their original application order.
    pub(crate) effects: Vec<VisualEffect>,
    /// Flags describing area and motion-policy decisions.
    pub(crate) degradation: VisualDegradation,
}

/// Select original, reduced, or no effects according to motion and area policies.
///
/// Large-area skipping takes precedence over reduction. Otherwise either global
/// reduced motion or the large-area reduction policy selects reduced variants.
pub(crate) fn plan_effects(effects: &[VisualEffect], ctx: &VisualCtx<'_>) -> EffectPlan {
    let large_area = ctx.is_large_area();
    let policy = ctx.performance.policy();
    let skip_for_large_area = large_area && policy == LargeAreaPolicy::SkipEffects;
    let reduce_for_large_area = large_area && policy == LargeAreaPolicy::ReduceMotion;
    let reduce = ctx.motion_policy.reduced_motion || reduce_for_large_area;

    let planned = if skip_for_large_area {
        Vec::new()
    } else if reduce {
        effects.iter().map(VisualEffect::reduced).collect()
    } else {
        effects.to_vec()
    };

    EffectPlan {
        effects: planned,
        degradation: VisualDegradation {
            large_area,
            reduced_motion: reduce,
            skipped_effects: skip_for_large_area,
        },
    }
}

#[cfg(test)]
/// Return only the planned effects for assertions about degradation behavior.
pub(crate) fn effective_effects(
    effects: &[VisualEffect],
    ctx: &VisualCtx<'_>,
) -> Vec<VisualEffect> {
    plan_effects(effects, ctx).effects
}

/// Clamp enabled animation progress, or sample its final state when motion is disabled.
pub(crate) fn effective_progress(progress: f64, motion_policy: MotionPolicy) -> f64 {
    if motion_policy.enabled {
        progress.clamp(0.0, 1.0)
    } else {
        1.0
    }
}
