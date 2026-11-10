//! Apply cell effects and optional backdrop composition while copying offscreen output.

use std::time::Instant;

use crate::render::buffer::Buffer;

use super::backdrop::BackdropSnapshot;
use crate::tui::offscreen::{for_each_blit_cell, BlitOptions, BlitRegion};
use crate::tui::visual_engine::pipeline::{
    effective_progress, plan_effects, EffectiveEffectGroup, ResolvedEffectGroup,
};
use crate::tui::visual_engine::{Backdrop, BlitReport, CellSample, VisualCtx, VisualEffect};

/// Blit one effect list at the supplied context's progress without degradation planning.
///
/// Effects run in order until a sample becomes hidden. Destinations are rounded
/// and clipped to the wrapper area before backdrop composition. Without a
/// backdrop, blank cells are skipped and dirty-only sampling is used when every
/// effect permits it. Timing is collected only when `profile_enabled` is true.
pub(crate) fn blit_with_effects(
    chunk: &mut crate::render::chunk::Chunk,
    source: &Buffer,
    effects: &[VisualEffect],
    backdrop: Option<&Backdrop>,
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

    let backdrop_uses_raster = backdrop.is_some() && chunk.buffer().has_raster();
    let snapshot =
        backdrop.map(|backdrop| BackdropSnapshot::capture(chunk.buffer(), backdrop, ctx));
    let options = if backdrop.is_some() {
        BlitOptions::default()
    } else if effects.iter().all(VisualEffect::can_use_dirty_only) {
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
            if let (Some(backdrop), Some(snapshot)) = (backdrop, snapshot.as_ref()) {
                snapshot.apply(backdrop, &mut sample, ctx);
            }
            if sample.visible {
                let _ = chunk.set_forced(dest_x as u16, dest_y as u16, sample.content);
                report.processed_cells += 1;
            } else {
                report.skipped_cells += 1;
            }
        } else if sample.visible {
            report.skipped_cells += 1;
        }
    });
    // Backdrops sample stored fallbacks, including a halo outside this chunk.
    // Conservatively reject native composition if any lower raster is present.
    if blit_stats.unsupported_raster_composition || backdrop_uses_raster {
        chunk.mark_unsupported_raster_composition();
    }
    report.blank_cells_skipped = blit_stats.skipped_blank;
    report.clean_cells_skipped = blit_stats.skipped_clean;
    if let Some(started) = started {
        report.effect_apply_time = started.elapsed();
    }
    report
}

/// Plan and blit effect groups with independent progress values.
///
/// Each group receives motion-policy-adjusted progress and area-based degradation.
/// Groups transform the same sample in order, stopping when it becomes hidden.
/// A backdrop is sampled once before writing and composed after the effects.
/// The report combines degradation flags across groups; timing includes planning.
pub(crate) fn blit_with_effect_groups(
    chunk: &mut crate::render::chunk::Chunk,
    source: &Buffer,
    groups: &[ResolvedEffectGroup],
    backdrop: Option<&Backdrop>,
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

    let backdrop_uses_raster = backdrop.is_some() && chunk.buffer().has_raster();
    let snapshot =
        backdrop.map(|backdrop| BackdropSnapshot::capture(chunk.buffer(), backdrop, ctx));
    let options = if backdrop.is_some() {
        BlitOptions::default()
    } else if effective_groups
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
            if let (Some(backdrop), Some(snapshot)) = (backdrop, snapshot.as_ref()) {
                snapshot.apply(backdrop, &mut sample, ctx);
            }
            if sample.visible {
                let _ = chunk.set_forced(dest_x as u16, dest_y as u16, sample.content);
                report.processed_cells += 1;
            } else {
                report.skipped_cells += 1;
            }
        } else if sample.visible {
            report.skipped_cells += 1;
        }
    });
    if blit_stats.unsupported_raster_composition || backdrop_uses_raster {
        chunk.mark_unsupported_raster_composition();
    }
    report.blank_cells_skipped = blit_stats.skipped_blank;
    report.clean_cells_skipped = blit_stats.skipped_clean;
    if let Some(started) = started {
        report.effect_apply_time = started.elapsed();
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{area::Area, chunk::Chunk, raster::RasterCanvas, style::Stylized};
    use crate::tui::{animation::MotionPolicy, style::Theme};

    fn raster_source() -> Buffer {
        let mut buffer = Buffer::new((2, 1).into());
        let canvas = RasterCanvas::new(2, 2, Default::default()).unwrap();
        Chunk::new(&mut buffer, Area::new((0, 0).into(), (2, 1).into()))
            .unwrap()
            .raster(&canvas, Default::default())
            .unwrap();
        buffer
    }

    fn apply(
        grouped: bool,
        source: &Buffer,
        target: &mut Buffer,
        progress: f64,
        backdrop: Option<&Backdrop>,
    ) -> BlitReport {
        let theme = Theme::dark();
        let area = Area::new((0, 0).into(), (2, 1).into());
        let ctx = VisualCtx::new(
            progress,
            area,
            Instant::now(),
            0,
            1.0,
            MotionPolicy::default(),
            &theme,
            None,
            0,
        );
        let mut chunk = Chunk::new(target, area).unwrap();
        if grouped {
            blit_with_effect_groups(
                &mut chunk,
                source,
                &[ResolvedEffectGroup {
                    progress,
                    effects: vec![VisualEffect::fade_in()],
                }],
                backdrop,
                &ctx,
                false,
            )
        } else {
            blit_with_effects(
                &mut chunk,
                source,
                &[VisualEffect::fade_in()],
                backdrop,
                &ctx,
                false,
            )
        }
    }

    #[test]
    fn both_effect_paths_flatten_raster_and_propagate_unsupported_even_when_hidden() {
        let source = raster_source();
        for grouped in [false, true] {
            for progress in [0.0, 1.0] {
                let mut target = Buffer::new(source.size());
                let report = apply(grouped, &source, &mut target, progress, None);
                assert!(target.has_unsupported_raster_composition());
                assert!(!target.has_raster());
                if progress == 1.0 {
                    assert_eq!(report.processed_cells, 2);
                    assert_eq!(target.content(), source.content());
                } else {
                    assert_eq!(report.processed_cells, 0);
                    assert_eq!(target.content(), Buffer::new(source.size()).content());
                }
            }
        }
    }

    #[test]
    fn both_effect_paths_propagate_nested_flattening_without_flagging_pure_text() {
        let mut source = Buffer::new((2, 1).into());
        source.set((0, 0).into(), Stylized::plain('x')).unwrap();
        for flattened in [false, true] {
            if flattened {
                source.mark_unsupported_raster_composition();
            }
            for grouped in [false, true] {
                let mut target = Buffer::new(source.size());
                apply(grouped, &source, &mut target, 1.0, None);
                assert_eq!(target.has_unsupported_raster_composition(), flattened);
                assert_eq!(target.get((0, 0).into()).unwrap().as_char(), Some('x'));
            }
        }
    }

    #[test]
    fn backdrop_sampling_lower_raster_is_not_silently_native() {
        let source = Buffer::new((2, 1).into());
        let backdrop = Backdrop::frosted();
        for grouped in [false, true] {
            let mut target = raster_source();
            let report = apply(grouped, &source, &mut target, 1.0, Some(&backdrop));
            assert_eq!(report.processed_cells, 2);
            assert!(!target.has_raster());
            assert!(target.has_unsupported_raster_composition());
        }
    }
}
