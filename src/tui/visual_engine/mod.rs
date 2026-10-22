//! Internal terminal-cell visual effects engine.
//!
//! This module owns visual effect data, cell sampling, degradation planning,
//! profiling, and source-buffer blitting. Widget wrappers feed it offscreen
//! buffers and runtime context; it does not know about widget trees.

mod blit;
mod color;
mod config;
mod context;
mod custom;
mod effect;
mod effects;
mod math;
mod pipeline;
mod profile;
mod sample;

pub(crate) use blit::{blit_with_effect_groups, blit_with_effects};
pub(crate) use config::ResolvedVisualConfig;
pub use config::{
    LargeAreaPolicy, TerminalVisualCapabilities, VisualConfig, VisualPerformanceConfig,
};
pub use context::VisualCtx;
pub use custom::{CellEffect, CustomCellEffect};
pub use effect::{
    BlurMode, DissolveMode, GradientDirection, GradientTarget, StaggerMode, TypewriterMode,
    VisualAnchor, VisualEffect, WaveAxis, WipeDirection, WipeMode,
};
pub(crate) use math::stable_seed;
pub(crate) use pipeline::{lifecycle_progress, ResolvedEffectGroup};
pub(crate) use profile::BlitReport;
pub use profile::{VisualDegradation, VisualEffectCost, VisualProfile};
pub use sample::CellSample;

#[cfg(test)]
pub(crate) use effects::staggered_progress;
#[cfg(test)]
pub(crate) use pipeline::{effective_effects, effective_progress};

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use crate::render::area::Area;
    use crate::render::buffer::Buffer;
    use crate::render::chunk::Chunk;
    use crate::render::style::Stylized;
    use crossterm::style::Color as CrosstermColor;

    use super::*;
    use crate::tui::animation::{AnimationStore, InitialAnimation, MotionPolicy};
    use crate::tui::style::{Color, EffectSlot, Theme, ThemeEffects};
    use crate::tui::widget::{RenderCtx, Widget, WidgetPath, WidgetStore};
    use crate::tui::widgets::label;
    use crate::tui::widgets::visual::{visual, LifecycleVisualEffect};

    #[test]
    fn visual_wrapper_renders_child_content() {
        let widget = visual(label::<()>("hello")).progress(1.0).fade_in();
        let buffer = render_widget(&widget, 8, 1);

        assert_eq!(cell_char(&buffer, 0, 0), Some('h'));
        assert_eq!(cell_char(&buffer, 4, 0), Some('o'));
    }

    #[test]
    fn fade_can_hide_cells_at_zero_progress() {
        let widget = visual(label::<()>("hello")).progress(0.0).fade_in();
        let buffer = render_widget(&widget, 8, 1);

        assert_eq!(cell_char(&buffer, 0, 0), Some(' '));
        assert_eq!(cell_char(&buffer, 4, 0), Some(' '));
    }

    #[test]
    fn gradient_overrides_cell_foreground() {
        let widget = visual(label::<()>("ab")).progress(1.0).gradient(
            Color::Rgb(0, 0, 0),
            Color::Rgb(10, 0, 0),
            GradientDirection::Horizontal,
        );
        let buffer = render_widget(&widget, 2, 1);
        let first = cell(&buffer, 0, 0).unwrap();
        let second = cell(&buffer, 1, 0).unwrap();

        assert_ne!(first.content.style.colors, second.content.style.colors);
    }

    #[test]
    fn geometry_effect_updates_destination_not_source() {
        let theme = Theme::dark();
        let area = Area::new((0, 0).into(), (5, 3).into());
        let ctx = VisualCtx::new(
            0.0,
            area,
            std::time::Instant::now(),
            7,
            1.0,
            MotionPolicy::default(),
            &theme,
            None,
            123,
        );
        let mut sample = CellSample::new(0, 0, Stylized::plain('x'));

        VisualEffect::magic_lamp(VisualAnchor::Bottom)
            .squeeze(0.0)
            .apply(&mut sample, &ctx);

        assert_eq!((sample.source_x, sample.source_y), (0.0, 0.0));
        assert_ne!((sample.dest_x, sample.dest_y), (0.0, 0.0));
    }

    #[test]
    fn visual_config_resolves_theme_default_and_local_override() {
        let theme = Theme::dark().with_effects(ThemeEffects::default().cell_aspect(0.25));

        let themed = visual(label::<()>("x"));
        assert_eq!(themed.resolve_config(&theme).cell_aspect, 0.25);

        let local = visual(label::<()>("x")).config(VisualConfig::default().cell_aspect(0.5));
        assert_eq!(local.resolve_config(&theme).cell_aspect, 0.5);
    }

    #[test]
    fn visual_enter_exit_declare_presence() {
        let widget = visual(label::<()>("x"))
            .enter(VisualEffect::fade_in())
            .exit(VisualEffect::fade_out());
        let presence = widget.presence().unwrap();

        assert!(presence.enter.is_some());
        assert!(presence.exit.is_some());
        assert_eq!(presence.initial, InitialAnimation::Play);
    }

    #[test]
    fn visual_theme_slots_declare_presence_and_resolve_late() {
        let widget = visual(label::<()>("x"))
            .enter_theme(EffectSlot::ToastEnter)
            .exit_theme(EffectSlot::ToastExit);

        assert!(matches!(
            widget.enter_effect,
            Some(LifecycleVisualEffect::Theme(EffectSlot::ToastEnter))
        ));
        assert!(matches!(
            widget.exit_effect,
            Some(LifecycleVisualEffect::Theme(EffectSlot::ToastExit))
        ));
        assert!(widget.presence().unwrap().enter.is_some());
        assert_eq!(
            widget
                .enter_effect
                .as_ref()
                .unwrap()
                .resolve(&Theme::dark()),
            ThemeEffects::default().toast_enter
        );
    }

    #[test]
    fn custom_cell_effect_can_be_applied_by_visual_wrapper() {
        struct Uppercase;

        impl CellEffect for Uppercase {
            fn apply(&self, sample: &mut CellSample, _ctx: VisualCtx<'_>) {
                let text = sample.content.symbol().to_uppercase();
                sample.content.set_grapheme(&text);
            }

            fn estimated_cost(&self) -> VisualEffectCost {
                VisualEffectCost::Cheap
            }
        }

        let widget = visual(label::<()>("ab"))
            .progress(1.0)
            .custom_effect(Uppercase);
        let buffer = render_widget(&widget, 2, 1);

        assert_eq!(cell_char(&buffer, 0, 0), Some('A'));
        assert_eq!(cell_char(&buffer, 1, 0), Some('B'));
        assert_eq!(
            VisualEffect::custom_named("uppercase", Uppercase).estimated_cost(),
            VisualEffectCost::Cheap
        );
    }

    #[test]
    fn visual_profile_hook_receives_render_counts() {
        let profiles = Arc::new(Mutex::new(Vec::<VisualProfile>::new()));
        let sink = Arc::clone(&profiles);
        let widget = visual(label::<()>("profile"))
            .progress(1.0)
            .fade_in()
            .profile(move |profile| sink.lock().unwrap().push(profile));

        let _ = render_widget(&widget, 12, 1);

        let profiles = profiles.lock().unwrap();
        assert_eq!(profiles.len(), 1);
        assert!(profiles[0].processed_cells > 0);
        assert_eq!(profiles[0].degradation, VisualDegradation::default());
    }

    #[test]
    fn shatter_uses_cell_aspect_for_horizontal_offset() {
        let theme = Theme::dark();
        let area = Area::new((0, 0).into(), (10, 4).into());
        let square_ctx = VisualCtx::new(
            0.5,
            area,
            std::time::Instant::now(),
            1,
            1.0,
            MotionPolicy::default(),
            &theme,
            None,
            123,
        );
        let narrow_ctx = VisualCtx::new(
            0.5,
            area,
            std::time::Instant::now(),
            1,
            0.5,
            MotionPolicy::default(),
            &theme,
            None,
            123,
        );
        let effect = VisualEffect::Shatter {
            seed: 0x1234,
            spread_x: 8.0,
            spread_y: 0.0,
            fade: false,
        };
        let mut square = CellSample::new(3, 1, Stylized::plain('x'));
        let mut narrow = CellSample::new(3, 1, Stylized::plain('x'));

        effect.apply(&mut square, &square_ctx);
        effect.apply(&mut narrow, &narrow_ctx);

        assert_ne!(square.dest_x, narrow.dest_x);
        assert_eq!(square.dest_y, narrow.dest_y);
        assert!((narrow.dest_x - narrow.source_x).abs() < (square.dest_x - square.source_x).abs());
    }

    #[test]
    fn magic_lamp_uses_cell_aspect_for_horizontal_bow() {
        let theme = Theme::dark();
        let area = Area::new((0, 0).into(), (5, 3).into());
        let square_ctx = VisualCtx::new(
            0.0,
            area,
            std::time::Instant::now(),
            1,
            1.0,
            MotionPolicy::default(),
            &theme,
            None,
            123,
        );
        let narrow_ctx = VisualCtx::new(
            0.0,
            area,
            std::time::Instant::now(),
            1,
            0.5,
            MotionPolicy::default(),
            &theme,
            None,
            123,
        );
        let effect = VisualEffect::magic_lamp(VisualAnchor::Bottom).squeeze(0.0);
        let mut square = CellSample::new(0, 0, Stylized::plain('x'));
        let mut narrow = CellSample::new(0, 0, Stylized::plain('x'));

        effect.apply(&mut square, &square_ctx);
        effect.apply(&mut narrow, &narrow_ctx);

        assert_ne!(square.dest_x, narrow.dest_x);
        assert_eq!(square.dest_y, narrow.dest_y);
    }

    #[test]
    fn disabled_motion_samples_final_visual_progress() {
        assert_eq!(effective_progress(0.25, MotionPolicy::disabled()), 1.0);
        assert_eq!(effective_progress(0.25, MotionPolicy::default()), 0.25);
    }

    #[test]
    fn reduced_motion_replaces_spatial_effects_with_fades() {
        let theme = Theme::dark();
        let area = Area::new((0, 0).into(), (8, 2).into());
        let ctx = VisualCtx::new(
            0.5,
            area,
            std::time::Instant::now(),
            1,
            1.0,
            MotionPolicy::reduced_motion(),
            &theme,
            None,
            123,
        );

        let effects = effective_effects(&[VisualEffect::shatter()], &ctx);
        assert!(matches!(
            effects[0],
            VisualEffect::Fade { from: 1.0, to: 0.0 }
        ));

        let mut sample = CellSample::new(3, 1, Stylized::plain('x'));
        effects[0].apply(&mut sample, &ctx);

        assert_eq!((sample.dest_x, sample.dest_y), (3.0, 1.0));
    }

    #[test]
    fn large_areas_use_reduced_effects() {
        let theme = Theme::dark();
        let area = Area::new((0, 0).into(), (120, 30).into());
        let ctx = VisualCtx::new(
            0.5,
            area,
            std::time::Instant::now(),
            1,
            1.0,
            MotionPolicy::default(),
            &theme,
            None,
            123,
        );

        let effects = effective_effects(&[VisualEffect::magic_lamp(VisualAnchor::Bottom)], &ctx);
        assert!(matches!(
            effects[0],
            VisualEffect::Fade { from: 0.0, to: 1.0 }
        ));
    }

    #[test]
    fn large_area_policy_can_skip_effects_and_report_degradation() {
        let profiles = Arc::new(Mutex::new(Vec::<VisualProfile>::new()));
        let sink = Arc::clone(&profiles);
        let performance = VisualPerformanceConfig::default()
            .large_area_threshold(1)
            .large_area_policy(LargeAreaPolicy::SkipEffects);
        let widget = visual(label::<()>("wide"))
            .seed(0x5A1F)
            .progress(0.5)
            .performance(performance)
            .shatter()
            .profile(move |profile| sink.lock().unwrap().push(profile));

        let buffer = render_widget(&widget, 4, 1);

        assert_eq!(cell_char(&buffer, 0, 0), Some('w'));
        let profile = profiles.lock().unwrap()[0];
        assert!(profile.degradation.large_area);
        assert!(profile.degradation.skipped_effects);
    }

    #[test]
    fn sequence_applies_completed_then_active_effects() {
        let theme = Theme::dark();
        let area = Area::new((0, 0).into(), (2, 1).into());
        let early_ctx = VisualCtx::new(
            0.25,
            area,
            std::time::Instant::now(),
            1,
            1.0,
            MotionPolicy::default(),
            &theme,
            None,
            123,
        );
        let late_ctx = early_ctx.with_progress(0.75);
        let effect = VisualEffect::sequence(vec![
            VisualEffect::gradient(
                Color::Rgb(255, 0, 0),
                Color::Rgb(255, 0, 0),
                GradientDirection::Horizontal,
            ),
            VisualEffect::gradient(
                Color::Rgb(0, 0, 255),
                Color::Rgb(0, 0, 255),
                GradientDirection::Horizontal,
            ),
        ]);
        let mut early = CellSample::new(0, 0, Stylized::plain('x'));
        let mut late = CellSample::new(0, 0, Stylized::plain('x'));

        effect.apply(&mut early, &early_ctx);
        effect.apply(&mut late, &late_ctx);

        assert_ne!(early.content.style.colors, late.content.style.colors);
    }

    #[test]
    fn parallel_applies_later_effects_last() {
        let theme = Theme::dark();
        let area = Area::new((0, 0).into(), (2, 1).into());
        let ctx = VisualCtx::new(
            1.0,
            area,
            std::time::Instant::now(),
            1,
            1.0,
            MotionPolicy::default(),
            &theme,
            None,
            123,
        );
        let effect = VisualEffect::parallel(vec![
            VisualEffect::gradient(
                Color::Rgb(255, 0, 0),
                Color::Rgb(255, 0, 0),
                GradientDirection::Horizontal,
            ),
            VisualEffect::gradient(
                Color::Rgb(0, 0, 255),
                Color::Rgb(0, 0, 255),
                GradientDirection::Horizontal,
            ),
        ]);
        let mut parallel = CellSample::new(0, 0, Stylized::plain('x'));
        let mut blue = CellSample::new(0, 0, Stylized::plain('x'));

        effect.apply(&mut parallel, &ctx);
        VisualEffect::gradient(
            Color::Rgb(0, 0, 255),
            Color::Rgb(0, 0, 255),
            GradientDirection::Horizontal,
        )
        .apply(&mut blue, &ctx);

        assert_eq!(parallel.content.style.colors, blue.content.style.colors);
    }

    #[test]
    fn stagger_rows_delays_later_rows() {
        let theme = Theme::dark();
        let area = Area::new((0, 0).into(), (4, 3).into());
        let ctx = VisualCtx::new(
            0.5,
            area,
            std::time::Instant::now(),
            1,
            1.0,
            MotionPolicy::default(),
            &theme,
            None,
            123,
        );
        let top = CellSample::new(0, 0, Stylized::plain('x'));
        let bottom = CellSample::new(0, 2, Stylized::plain('x'));

        assert!(
            staggered_progress(&top, &ctx, 0.2, StaggerMode::Rows)
                > staggered_progress(&bottom, &ctx, 0.2, StaggerMode::Rows)
        );
    }

    #[test]
    fn center_wipe_uses_aspect_corrected_distance() {
        let theme = Theme::dark();
        let area = Area::new((0, 0).into(), (7, 3).into());
        let square_ctx = VisualCtx::new(
            0.88,
            area,
            std::time::Instant::now(),
            1,
            1.0,
            MotionPolicy::default(),
            &theme,
            None,
            123,
        );
        let narrow_ctx = VisualCtx::new(
            0.88,
            area,
            std::time::Instant::now(),
            1,
            0.5,
            MotionPolicy::default(),
            &theme,
            None,
            123,
        );
        let effect = VisualEffect::reveal(WipeDirection::CenterOut);
        let mut square = CellSample::new(6, 1, Stylized::plain('x'));
        let mut narrow = CellSample::new(6, 1, Stylized::plain('x'));

        effect.apply(&mut square, &square_ctx);
        effect.apply(&mut narrow, &narrow_ctx);

        assert!(!square.visible);
        assert!(narrow.visible);
    }

    #[test]
    fn dissolve_is_stable_for_same_seed() {
        let theme = Theme::dark();
        let area = Area::new((0, 0).into(), (8, 2).into());
        let ctx = VisualCtx::new(
            0.5,
            area,
            std::time::Instant::now(),
            1,
            1.0,
            MotionPolicy::default(),
            &theme,
            None,
            9001,
        );
        let effect = VisualEffect::dissolve().with_seed(77);
        let mut first = CellSample::new(4, 1, Stylized::plain('x'));
        let mut second = CellSample::new(4, 1, Stylized::plain('x'));

        effect.apply(&mut first, &ctx);
        effect.apply(&mut second, &ctx);

        assert_eq!(first.visible, second.visible);
    }

    #[test]
    fn wave_uses_cell_aspect_for_row_offsets() {
        let theme = Theme::dark();
        let area = Area::new((0, 0).into(), (12, 4).into());
        let square_ctx = VisualCtx::new(
            0.5,
            area,
            std::time::Instant::now(),
            1,
            1.0,
            MotionPolicy::default(),
            &theme,
            None,
            123,
        );
        let narrow_ctx = VisualCtx::new(
            0.5,
            area,
            std::time::Instant::now(),
            1,
            0.5,
            MotionPolicy::default(),
            &theme,
            None,
            123,
        );
        let effect = VisualEffect::wave(WaveAxis::Rows)
            .amplitude(4.0)
            .wavelength(5.0);
        let mut square = CellSample::new(4, 1, Stylized::plain('x'));
        let mut narrow = CellSample::new(4, 1, Stylized::plain('x'));

        effect.apply(&mut square, &square_ctx);
        effect.apply(&mut narrow, &narrow_ctx);

        assert_ne!(square.dest_x, narrow.dest_x);
        assert_eq!(square.dest_y, narrow.dest_y);
        assert!((narrow.dest_x - narrow.source_x).abs() < (square.dest_x - square.source_x).abs());
    }

    #[test]
    fn glitch_is_stable_for_same_seed() {
        let theme = Theme::dark();
        let area = Area::new((0, 0).into(), (8, 2).into());
        let ctx = VisualCtx::new(
            0.5,
            area,
            std::time::Instant::now(),
            1,
            1.0,
            MotionPolicy::default(),
            &theme,
            None,
            42,
        );
        let effect = VisualEffect::glitch().with_seed(123).intensity(1.0);
        let mut first = CellSample::new(3, 1, Stylized::plain('x'));
        let mut second = CellSample::new(3, 1, Stylized::plain('x'));

        effect.apply(&mut first, &ctx);
        effect.apply(&mut second, &ctx);

        assert_eq!(first, second);
    }

    #[test]
    fn scanline_dims_target_rows_without_moving_cells() {
        let theme = Theme::dark();
        let area = Area::new((0, 0).into(), (4, 3).into());
        let ctx = VisualCtx::new(
            1.0,
            area,
            std::time::Instant::now(),
            1,
            1.0,
            MotionPolicy::default(),
            &theme,
            None,
            42,
        );
        let effect = VisualEffect::scanline().density(0.5).intensity(0.5);
        let mut target = CellSample::new(0, 0, Stylized::plain('x'));
        let mut untouched = CellSample::new(0, 1, Stylized::plain('x'));
        let original = untouched.clone();

        effect.apply(&mut target, &ctx);
        effect.apply(&mut untouched, &ctx);

        assert_eq!((target.dest_x, target.dest_y), (0.0, 0.0));
        assert_ne!(target.content.style, Stylized::plain('x').style);
        assert_eq!(untouched, original);
    }

    #[test]
    fn typewriter_reveals_row_major_content() {
        let hidden = render_widget(
            &visual(label::<()>("hello"))
                .progress(0.0)
                .effect(VisualEffect::typewriter()),
            5,
            1,
        );
        let shown = render_widget(
            &visual(label::<()>("hello"))
                .progress(1.0)
                .effect(VisualEffect::typewriter()),
            5,
            1,
        );

        assert_eq!(cell_char(&hidden, 0, 0), Some(' '));
        assert_eq!(cell_char(&shown, 0, 0), Some('h'));
        assert_eq!(cell_char(&shown, 4, 0), Some('o'));
    }

    #[test]
    fn blur_like_in_clears_as_progress_completes() {
        let theme = Theme::dark();
        let area = Area::new((0, 0).into(), (5, 1).into());
        let blurry_ctx = VisualCtx::new(
            0.0,
            area,
            std::time::Instant::now(),
            1,
            1.0,
            MotionPolicy::default(),
            &theme,
            None,
            123,
        );
        let clear_ctx = blurry_ctx.with_progress(1.0);
        let effect = VisualEffect::blur_like()
            .radius(2.0)
            .blur_mode(BlurMode::In);
        let mut blurry = CellSample::new(2, 0, Stylized::plain('x'));
        let mut clear = CellSample::new(2, 0, Stylized::plain('x'));

        effect.apply(&mut blurry, &blurry_ctx);
        effect.apply(&mut clear, &clear_ctx);

        assert_ne!(blurry.content.as_char(), Some('x'));
        assert_eq!(clear.content.as_char(), Some('x'));
    }

    #[test]
    fn highlight_sweep_changes_style_without_geometry() {
        let theme = Theme::dark();
        let area = Area::new((0, 0).into(), (5, 1).into());
        let ctx = VisualCtx::new(
            0.5,
            area,
            std::time::Instant::now(),
            1,
            1.0,
            MotionPolicy::default(),
            &theme,
            None,
            123,
        );
        let mut sample = CellSample::new(2, 0, Stylized::plain('x'));

        VisualEffect::highlight_sweep()
            .width(0.3)
            .color(Color::Rgb(255, 255, 180))
            .apply(&mut sample, &ctx);

        assert_eq!((sample.dest_x, sample.dest_y), (2.0, 0.0));
        assert!(sample.content.has_color() || sample.content.has_attr());
    }

    #[test]
    fn sparkle_is_seeded_and_density_controlled() {
        let theme = Theme::dark();
        let area = Area::new((0, 0).into(), (8, 2).into());
        let ctx = VisualCtx::new(
            1.0,
            area,
            std::time::Instant::now(),
            3,
            1.0,
            MotionPolicy::default(),
            &theme,
            None,
            42,
        );
        let off = VisualEffect::sparkle().density(0.0);
        let on = VisualEffect::sparkle().density(1.0).with_seed(1);
        let mut unchanged = CellSample::new(0, 0, Stylized::plain('x'));
        let original = unchanged.clone();
        let mut first = CellSample::new(0, 0, Stylized::plain('x'));
        let mut second = CellSample::new(0, 0, Stylized::plain('x'));

        off.apply(&mut unchanged, &ctx);
        on.apply(&mut first, &ctx);
        on.apply(&mut second, &ctx);

        assert_eq!(unchanged, original);
        assert_eq!(first, second);
    }

    #[test]
    fn terminal_capabilities_degrade_truecolor_and_unicode() {
        let theme = Theme::dark();
        let area = Area::new((0, 0).into(), (3, 1).into());
        let ctx = VisualCtx::new(
            0.5,
            area,
            std::time::Instant::now(),
            1,
            1.0,
            MotionPolicy::default(),
            &theme,
            None,
            42,
        )
        .with_capabilities(
            TerminalVisualCapabilities::default()
                .truecolor(false)
                .unicode_blocks(false),
        );
        let mut color = CellSample::new(1, 0, Stylized::plain('x'));
        let mut blur = CellSample::new(1, 0, Stylized::plain('x'));

        VisualEffect::gradient(
            Color::Rgb(255, 128, 0),
            Color::Rgb(255, 128, 0),
            GradientDirection::Horizontal,
        )
        .apply(&mut color, &ctx);
        VisualEffect::blur_like().apply(&mut blur, &ctx.with_progress(0.0));

        let foreground = color
            .content
            .style
            .colors
            .and_then(|colors| colors.foreground)
            .unwrap();
        assert!(!matches!(foreground, CrosstermColor::Rgb { .. }));
        assert!(!matches!(blur.content.as_char(), Some('░' | '▒')));
    }

    #[test]
    fn dirty_only_is_used_only_for_static_effects() {
        let static_profiles = Arc::new(Mutex::new(Vec::<VisualProfile>::new()));
        let static_sink = Arc::clone(&static_profiles);
        let static_widget = visual(label::<()>("dirty"))
            .seed(0xD17)
            .progress(1.0)
            .effect(VisualEffect::gradient(
                Color::Rgb(255, 0, 0),
                Color::Rgb(255, 0, 0),
                GradientDirection::Horizontal,
            ))
            .profile(move |profile| static_sink.lock().unwrap().push(profile));

        let _ = render_widget(&static_widget, 8, 1);
        let _ = render_widget(&static_widget, 8, 1);
        assert!(static_profiles.lock().unwrap()[1].clean_cells_skipped > 0);

        let animated_profiles = Arc::new(Mutex::new(Vec::<VisualProfile>::new()));
        let animated_sink = Arc::clone(&animated_profiles);
        let animated_widget = visual(label::<()>("dirty"))
            .seed(0xD18)
            .progress(0.5)
            .effect(VisualEffect::fade_in())
            .profile(move |profile| animated_sink.lock().unwrap().push(profile));

        let _ = render_widget(&animated_widget, 8, 1);
        let _ = render_widget(&animated_widget, 8, 1);
        assert_eq!(animated_profiles.lock().unwrap()[1].clean_cells_skipped, 0);
    }

    #[test]
    fn reduced_motion_replaces_new_spatial_effects() {
        assert!(matches!(
            VisualEffect::reveal(WipeDirection::LeftToRight).reduced(),
            VisualEffect::Fade { from: 0.0, to: 1.0 }
        ));
        assert!(matches!(
            VisualEffect::dissolve_out().reduced(),
            VisualEffect::Fade { from: 1.0, to: 0.0 }
        ));
        assert!(matches!(
            VisualEffect::wave(WaveAxis::Rows).reduced(),
            VisualEffect::Fade { from: 0.0, to: 1.0 }
        ));
        assert!(matches!(
            VisualEffect::glitch().reduced(),
            VisualEffect::Fade { from: 0.0, to: 1.0 }
        ));
        assert!(matches!(
            VisualEffect::typewriter().reduced(),
            VisualEffect::Fade { from: 0.0, to: 1.0 }
        ));
        assert!(matches!(
            VisualEffect::blur_like().reduced(),
            VisualEffect::Fade { from: 0.0, to: 1.0 }
        ));
        assert!(matches!(
            VisualEffect::sparkle().reduced(),
            VisualEffect::Fade { from: 0.0, to: 1.0 }
        ));
        assert_eq!(
            VisualEffect::scanline().estimated_cost(),
            VisualEffectCost::Cheap
        );
        assert_eq!(
            VisualEffect::typewriter().estimated_cost(),
            VisualEffectCost::Cheap
        );
        assert_eq!(
            VisualEffect::highlight_sweep().estimated_cost(),
            VisualEffectCost::Cheap
        );
        assert_eq!(
            VisualEffect::blur_like().estimated_cost(),
            VisualEffectCost::Moderate
        );
        assert_eq!(
            VisualEffect::sparkle().estimated_cost(),
            VisualEffectCost::Moderate
        );
    }

    fn render_widget(widget: &impl Widget<()>, width: u16, height: u16) -> Buffer {
        let mut buffer = Buffer::new((width, height).into());
        let area = Area::new((0, 0).into(), (width, height).into());
        let mut chunk = Chunk::new(&mut buffer, area).unwrap();
        let store = WidgetStore::new();
        let animation_store = AnimationStore::new();
        let theme = Theme::dark();
        let geometry = RefCell::new(HashMap::<WidgetPath, Area>::new());
        let ctx = RenderCtx::new(&store, &animation_store, &theme, None, &geometry);
        widget.render(&mut chunk, &ctx);
        drop(chunk);
        buffer
    }

    fn cell(buffer: &Buffer, x: u16, y: u16) -> Option<&crate::render::buffer::Cell> {
        let index = (y * buffer.size().width + x) as usize;
        buffer.content().get(index)
    }

    fn cell_char(buffer: &Buffer, x: u16, y: u16) -> Option<char> {
        cell(buffer, x, y)?.content.as_char()
    }
}
