use crate::tui::animation::MotionPolicy;
use crate::tui::visual_engine::{LargeAreaPolicy, VisualCtx, VisualDegradation, VisualEffect};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedEffectGroup {
    pub(crate) progress: f64,
    pub(crate) effects: Vec<VisualEffect>,
}

#[derive(Debug, Clone)]
pub(crate) struct EffectiveEffectGroup<'a> {
    pub(crate) ctx: VisualCtx<'a>,
    pub(crate) effects: Vec<VisualEffect>,
}

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

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EffectPlan {
    pub(crate) effects: Vec<VisualEffect>,
    pub(crate) degradation: VisualDegradation,
}

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
pub(crate) fn effective_effects(
    effects: &[VisualEffect],
    ctx: &VisualCtx<'_>,
) -> Vec<VisualEffect> {
    plan_effects(effects, ctx).effects
}

pub(crate) fn effective_progress(progress: f64, motion_policy: MotionPolicy) -> f64 {
    if motion_policy.enabled {
        progress.clamp(0.0, 1.0)
    } else {
        1.0
    }
}
