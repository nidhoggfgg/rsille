use std::time::Instant;

use crate::render::buffer::Buffer;

use crate::tui::offscreen::{for_each_blit_cell, BlitOptions, BlitRegion};
use crate::tui::visual_engine::pipeline::{
    effective_progress, plan_effects, EffectiveEffectGroup, ResolvedEffectGroup,
};
use crate::tui::visual_engine::{BlitReport, CellSample, VisualCtx, VisualEffect};

pub(crate) fn blit_with_effects(
    chunk: &mut crate::render::chunk::Chunk,
    source: &Buffer,
    effects: &[VisualEffect],
    ctx: &VisualCtx<'_>,
    profile_enabled: bool,
) -> BlitReport {
    let started = profile_enabled.then(Instant::now);
    let mut report = BlitReport::default();
    let region = BlitRegion::new(
        ctx.area.x() as usize,
        ctx.area.y() as usize,
        0,
        0,
        ctx.area.width() as usize,
        ctx.area.height() as usize,
    );

    let options = if effects.iter().all(VisualEffect::can_use_dirty_only) {
        BlitOptions::default().skip_blank().dirty_only()
    } else {
        BlitOptions::default().skip_blank()
    };
    let blit_stats = for_each_blit_cell(source, region, Some(ctx.area.size()), options, |cell| {
        let mut sample = CellSample::new(cell.dest_x, cell.dest_y, cell.content);

        for effect in effects {
            effect.apply(&mut sample, ctx);
            if !sample.visible {
                report.skipped_cells += 1;
                break;
            }
        }

        let dest_x = sample.dest_x.round() as i32;
        let dest_y = sample.dest_y.round() as i32;
        if sample.visible
            && dest_x >= 0
            && dest_y >= 0
            && dest_x < ctx.area.width() as i32
            && dest_y < ctx.area.height() as i32
        {
            let _ = chunk.set_forced(dest_x as u16, dest_y as u16, sample.content);
            report.processed_cells += 1;
        } else if sample.visible {
            report.skipped_cells += 1;
        }
    });
    report.blank_cells_skipped = blit_stats.skipped_blank;
    report.clean_cells_skipped = blit_stats.skipped_clean;
    if let Some(started) = started {
        report.effect_apply_time = started.elapsed();
    }
    report
}

pub(crate) fn blit_with_effect_groups(
    chunk: &mut crate::render::chunk::Chunk,
    source: &Buffer,
    groups: &[ResolvedEffectGroup],
    ctx: &VisualCtx<'_>,
    profile_enabled: bool,
) -> BlitReport {
    let started = profile_enabled.then(Instant::now);
    let mut report = BlitReport::default();
    let effective_groups: Vec<EffectiveEffectGroup<'_>> = groups
        .iter()
        .map(|group| {
            let group_ctx =
                ctx.with_progress(effective_progress(group.progress, ctx.motion_policy));
            let plan = plan_effects(&group.effects, &group_ctx);
            report.add_degradation(plan.degradation);
            EffectiveEffectGroup {
                ctx: group_ctx,
                effects: plan.effects,
            }
        })
        .collect();
    let region = BlitRegion::new(
        ctx.area.x() as usize,
        ctx.area.y() as usize,
        0,
        0,
        ctx.area.width() as usize,
        ctx.area.height() as usize,
    );

    let options = if effective_groups
        .iter()
        .all(|group| group.effects.iter().all(VisualEffect::can_use_dirty_only))
    {
        BlitOptions::default().skip_blank().dirty_only()
    } else {
        BlitOptions::default().skip_blank()
    };
    let blit_stats = for_each_blit_cell(source, region, Some(ctx.area.size()), options, |cell| {
        let mut sample = CellSample::new(cell.dest_x, cell.dest_y, cell.content);

        for group in &effective_groups {
            if group.effects.is_empty() {
                continue;
            }
            for effect in &group.effects {
                effect.apply(&mut sample, &group.ctx);
                if !sample.visible {
                    report.skipped_cells += 1;
                    break;
                }
            }

            if !sample.visible {
                break;
            }
        }

        let dest_x = sample.dest_x.round() as i32;
        let dest_y = sample.dest_y.round() as i32;
        if sample.visible
            && dest_x >= 0
            && dest_y >= 0
            && dest_x < ctx.area.width() as i32
            && dest_y < ctx.area.height() as i32
        {
            let _ = chunk.set_forced(dest_x as u16, dest_y as u16, sample.content);
            report.processed_cells += 1;
        } else if sample.visible {
            report.skipped_cells += 1;
        }
    });
    report.blank_cells_skipped = blit_stats.skipped_blank;
    report.clean_cells_skipped = blit_stats.skipped_clean;
    if let Some(started) = started {
        report.effect_apply_time = started.elapsed();
    }
    report
}
