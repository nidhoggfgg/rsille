use crossterm::style::Attribute;

use crate::tui::style::Color;
use crate::tui::visual_engine::color::{
    add_attribute, apply_color_capable, blend_foreground, dim_foreground,
};
use crate::tui::visual_engine::math::{
    ease_out_cubic, effect_seed, finite_or, gradient_position, hash_pair, lerp, noise,
};
use crate::tui::visual_engine::{
    BlurMode, CellSample, DissolveMode, GradientDirection, GradientTarget, StaggerMode,
    TypewriterMode, VisualCtx, VisualEffect, WaveAxis, WipeDirection, WipeMode,
};

impl VisualEffect {
    pub(crate) fn apply(&self, sample: &mut CellSample, ctx: &VisualCtx<'_>) {
        let progress = ctx.progress;
        match self {
            Self::Fade { from, to } => {
                let alpha = lerp(*from, *to, progress).clamp(0.0, 1.0);
                let threshold = noise(sample.dest_x as u16, sample.dest_y as u16, 0xFAD3);
                sample.visible = threshold <= alpha;
            }
            Self::Gradient {
                start,
                end,
                direction,
                target,
                phase,
            } => {
                let shifted =
                    gradient_position(sample.dest_x, sample.dest_y, ctx.area, *direction) + *phase;
                let t = if (0.0..=1.0).contains(&shifted) {
                    shifted
                } else {
                    shifted.rem_euclid(1.0)
                };
                let color = start.interpolate(*end, t);
                apply_color_capable(&mut sample.content, color, *target, ctx);
            }
            Self::Shatter {
                seed,
                spread_x,
                spread_y,
                fade,
            } => {
                let t = ease_out_cubic(progress);
                let jitter = hash_pair(sample.source_x as u16, sample.source_y as u16, *seed);
                let angle = jitter.0 * std::f64::consts::TAU;
                let force = 0.35 + jitter.1 * 0.95;
                let lift = 0.3
                    + noise(
                        sample.source_y as u16,
                        sample.source_x as u16,
                        *seed ^ 0xBEEF,
                    ) * 0.8;
                sample.dest_x += ctx.aspect_adjusted_x_offset(angle.cos() * *spread_x * force * t);
                sample.dest_y += (angle.sin() * *spread_y * force - *spread_y * lift) * t;

                if *fade {
                    let threshold =
                        noise(sample.dest_x as u16, sample.dest_y as u16, *seed ^ 0xFADE);
                    sample.visible = threshold > progress;
                }
            }
            Self::MagicLamp { anchor, squeeze } => {
                let (anchor_x, anchor_y) = anchor.resolve(ctx.area.width(), ctx.area.height());
                let open = ease_out_cubic(progress);
                let pull = 1.0 - open;
                let scale = open + pull * *squeeze;
                let row = if ctx.area.height() <= 1 {
                    0.5
                } else {
                    sample.dest_y / ctx.area.height().saturating_sub(1) as f64
                };
                let bow = ctx.aspect_adjusted_x_offset(
                    ((row - 0.5) * std::f64::consts::PI).sin()
                        * pull
                        * ctx.area.width() as f64
                        * 0.16,
                );

                sample.dest_x = anchor_x
                    + ctx.aspect_adjusted_x_offset((sample.dest_x - anchor_x) * scale)
                    + bow;
                sample.dest_y =
                    anchor_y + (sample.dest_y - anchor_y) * (open * open + pull * *squeeze);
            }
            Self::Wipe {
                direction,
                mode,
                softness,
            } => {
                let position = wipe_position(sample.source_x, sample.source_y, ctx, *direction);
                sample.visible = wipe_visible(
                    position,
                    progress,
                    *mode,
                    *softness,
                    ctx.seed ^ 0xA11C_E5,
                    sample.source_x as u16,
                    sample.source_y as u16,
                );
            }
            Self::Dissolve { seed, mode } => {
                let threshold = noise(
                    sample.source_x as u16,
                    sample.source_y as u16,
                    effect_seed(ctx, *seed),
                );
                sample.visible = match mode {
                    DissolveMode::In => threshold <= progress,
                    DissolveMode::Out => threshold > progress,
                };
            }
            Self::Wave {
                axis,
                amplitude,
                wavelength,
                phase,
            } => {
                let envelope = (progress * std::f64::consts::PI).sin().max(0.0);
                if envelope > 0.0 && *amplitude > 0.0 {
                    let wave = match axis {
                        WaveAxis::Rows => {
                            (sample.source_y / wavelength.max(f64::EPSILON) + *phase + progress)
                                * std::f64::consts::TAU
                        }
                        WaveAxis::Cols => {
                            (ctx.logical_x(sample.source_x) / wavelength.max(f64::EPSILON)
                                + *phase
                                + progress)
                                * std::f64::consts::TAU
                        }
                    };
                    let offset = wave.sin() * *amplitude * envelope;
                    match axis {
                        WaveAxis::Rows => {
                            sample.dest_x += ctx.aspect_adjusted_x_offset(offset);
                        }
                        WaveAxis::Cols => {
                            sample.dest_y += offset;
                        }
                    }
                }
            }
            Self::Glitch { seed, intensity } => {
                apply_glitch(sample, ctx, effect_seed(ctx, *seed), *intensity);
            }
            Self::Scanline {
                density,
                intensity,
                phase,
            } => {
                apply_scanline(sample, ctx, *density, *intensity, *phase);
            }
            Self::Typewriter { mode, cursor } => {
                apply_typewriter(sample, ctx, *mode, *cursor);
            }
            Self::BlurLike { radius, mode } => {
                apply_blur_like(sample, ctx, *radius, *mode);
            }
            Self::HighlightSweep {
                color,
                width,
                direction,
            } => {
                apply_highlight_sweep(sample, ctx, *color, *width, *direction);
            }
            Self::Sparkle {
                seed,
                density,
                color,
            } => {
                apply_sparkle(sample, ctx, effect_seed(ctx, *seed), *density, *color);
            }
            Self::Sequence(effects) => {
                apply_sequence(effects, sample, ctx);
            }
            Self::Parallel(effects) => {
                for effect in effects {
                    effect.apply(sample, ctx);
                }
            }
            Self::Stagger {
                delay,
                mode,
                effect,
            } => {
                let local_progress = staggered_progress(sample, ctx, *delay, *mode);
                effect.apply(sample, &ctx.with_progress(local_progress));
            }
            Self::Custom(effect) => {
                effect.effect.apply(sample, *ctx);
            }
        }
    }
}

fn apply_sequence(effects: &[VisualEffect], sample: &mut CellSample, ctx: &VisualCtx<'_>) {
    let len = effects.len();
    if len == 0 {
        return;
    }

    let scaled = ctx.progress.clamp(0.0, 1.0) * len as f64;
    let active = scaled.floor().min((len - 1) as f64) as usize;
    let local = if active == len - 1 && ctx.progress >= 1.0 {
        1.0
    } else {
        scaled - active as f64
    };

    for (index, effect) in effects.iter().enumerate() {
        if index < active {
            effect.apply(sample, &ctx.with_progress(1.0));
        } else if index == active {
            effect.apply(sample, &ctx.with_progress(local));
        } else {
            break;
        }

        if !sample.visible {
            break;
        }
    }
}

fn wipe_position(x: f64, y: f64, ctx: &VisualCtx<'_>, direction: WipeDirection) -> f64 {
    let max_x = ctx.area.width().saturating_sub(1) as f64;
    let max_y = ctx.area.height().saturating_sub(1) as f64;
    let logical_x = ctx.logical_x(x);
    let logical_max_x = ctx.logical_x(max_x);

    let divide = |value: f64, max: f64| {
        if max <= 0.0 {
            0.0
        } else {
            (value / max).clamp(0.0, 1.0)
        }
    };

    match direction {
        WipeDirection::LeftToRight => divide(logical_x, logical_max_x),
        WipeDirection::RightToLeft => divide(logical_max_x - logical_x, logical_max_x),
        WipeDirection::TopToBottom => divide(y, max_y),
        WipeDirection::BottomToTop => divide(max_y - y, max_y),
        WipeDirection::CenterOut => {
            let center_x = logical_max_x * 0.5;
            let center_y = max_y * 0.5;
            let max_distance = [
                (0.0, 0.0),
                (logical_max_x, 0.0),
                (0.0, max_y),
                (logical_max_x, max_y),
            ]
            .into_iter()
            .map(|(corner_x, corner_y)| {
                ((corner_x - center_x).powi(2) + (corner_y - center_y).powi(2)).sqrt()
            })
            .fold(0.0, f64::max);
            let distance = ((logical_x - center_x).powi(2) + (y - center_y).powi(2)).sqrt();
            divide(distance, max_distance)
        }
        WipeDirection::EdgesIn => {
            let distance_to_edge = logical_x
                .min((logical_max_x - logical_x).max(0.0))
                .min(y.min((max_y - y).max(0.0)));
            let max_distance = (logical_max_x * 0.5).min(max_y * 0.5);
            divide(distance_to_edge, max_distance)
        }
        WipeDirection::DiagonalDown => divide(logical_x + y, logical_max_x + max_y),
        WipeDirection::DiagonalUp => divide(logical_x + (max_y - y), logical_max_x + max_y),
    }
}

fn wipe_visible(
    position: f64,
    progress: f64,
    mode: WipeMode,
    softness: f64,
    seed: u64,
    x: u16,
    y: u16,
) -> bool {
    let position = position.clamp(0.0, 1.0);
    let progress = progress.clamp(0.0, 1.0);
    if progress <= 0.0 {
        return mode == WipeMode::Hide;
    }
    if progress >= 1.0 {
        return mode == WipeMode::Reveal;
    }

    if softness <= 0.0 {
        return match mode {
            WipeMode::Reveal => position <= progress,
            WipeMode::Hide => position > progress,
        };
    }

    let feather = softness.max(f64::EPSILON);
    if position <= progress - feather {
        return mode == WipeMode::Reveal;
    }
    if position >= progress + feather {
        return mode == WipeMode::Hide;
    }

    let reveal_coverage = ((progress + feather - position) / (feather * 2.0)).clamp(0.0, 1.0);
    let coverage = match mode {
        WipeMode::Reveal => reveal_coverage,
        WipeMode::Hide => 1.0 - reveal_coverage,
    };
    noise(x, y, seed) <= coverage
}

fn apply_glitch(sample: &mut CellSample, ctx: &VisualCtx<'_>, seed: u64, intensity: f64) {
    let envelope = (ctx.progress * std::f64::consts::PI).sin().max(0.0);
    let active = finite_or(intensity, 0.0).clamp(0.0, 1.0) * envelope;
    if active <= 0.0 {
        return;
    }

    let x = sample.source_x as u16;
    let y = sample.source_y as u16;
    let gate = noise(x, y, seed ^ 0x61_17C);
    if gate > active {
        return;
    }

    let horizontal = (noise(y, x, seed ^ 0x0FF5) * 3.0).floor() - 1.0;
    let vertical = if noise(x, y, seed ^ 0x5CA1) < active * 0.45 {
        if noise(y, x, seed ^ 0x51DE) < 0.5 {
            -1.0
        } else {
            1.0
        }
    } else {
        0.0
    };
    sample.dest_x += ctx.aspect_adjusted_x_offset(horizontal);
    sample.dest_y += vertical;

    if noise(x, y, seed ^ 0xC0DE) < active * 0.55 {
        let charset = ['#', '%', '&', '+', '*', '!', '?', '/', '\\'];
        let index = (noise(y, x, seed ^ 0xC4A7) * charset.len() as f64) as usize;
        sample
            .content
            .set_char(charset[index.min(charset.len() - 1)]);
    }

    let color = if noise(x, y, seed ^ 0xC010) < 0.5 {
        Color::Rgb(255, 80, 120)
    } else {
        Color::Rgb(80, 220, 255)
    };
    apply_color_capable(&mut sample.content, color, GradientTarget::Foreground, ctx);
}

fn apply_scanline(
    sample: &mut CellSample,
    ctx: &VisualCtx<'_>,
    density: f64,
    intensity: f64,
    phase: f64,
) {
    let density = finite_or(density, 0.5).clamp(0.0, 1.0);
    let intensity = finite_or(intensity, 0.35).clamp(0.0, 1.0);
    if density <= 0.0 || intensity <= 0.0 {
        return;
    }

    let period = (1.0 / density).round().max(1.0);
    let lit_rows = (density * period).ceil().max(1.0);
    let row = (sample.source_y + phase * period)
        .floor()
        .rem_euclid(period);
    if row < lit_rows {
        dim_foreground(sample, ctx, intensity);
    }
}

fn apply_typewriter(
    sample: &mut CellSample,
    ctx: &VisualCtx<'_>,
    mode: TypewriterMode,
    cursor: bool,
) {
    let total = (ctx.area.width() as u32 * ctx.area.height() as u32).max(1) as usize;
    let index = (sample.source_y.round().max(0.0) as usize * ctx.area.width().max(1) as usize)
        + sample.source_x.round().max(0.0) as usize;

    let visible = match mode {
        TypewriterMode::Chars => {
            let visible_count = (ctx.progress.clamp(0.0, 1.0) * total as f64).floor() as usize;
            index < visible_count
        }
        TypewriterMode::Words => {
            let group_size = 6usize;
            let groups = total.div_ceil(group_size).max(1);
            let visible_groups = (ctx.progress.clamp(0.0, 1.0) * groups as f64).floor() as usize;
            index / group_size < visible_groups
        }
    };

    if visible || ctx.progress >= 1.0 {
        return;
    }

    let cursor_index = (ctx.progress.clamp(0.0, 1.0) * total as f64).floor() as usize;
    if cursor && index == cursor_index.min(total.saturating_sub(1)) && ctx.progress > 0.0 {
        sample.visible = true;
        sample.content.set_char(if ctx.capabilities.unicode_blocks {
            '▌'
        } else {
            '|'
        });
        apply_color_capable(
            &mut sample.content,
            ctx.theme
                .styles
                .border_focused
                .fg_color
                .unwrap_or(Color::Cyan),
            GradientTarget::Foreground,
            ctx,
        );
    } else {
        sample.visible = false;
    }
}

fn apply_blur_like(sample: &mut CellSample, ctx: &VisualCtx<'_>, radius: f64, mode: BlurMode) {
    let radius = finite_or(radius, 2.0).max(0.0);
    if radius <= 0.0 {
        return;
    }

    let amount = match mode {
        BlurMode::In => 1.0 - ctx.progress,
        BlurMode::Out => ctx.progress,
    }
    .clamp(0.0, 1.0);
    if amount <= 0.02 {
        return;
    }

    dim_foreground(sample, ctx, amount * 0.6);
    if sample.content.is_empty() || sample.content.is_whitespace() {
        return;
    }

    let x = sample.source_x as u16;
    let y = sample.source_y as u16;
    let seed = effect_seed(ctx, 0xB1_102E) ^ radius.to_bits();
    let n = noise(x, y, seed);

    if amount > 0.66 && n < amount * 0.55 {
        sample.content.set_char(' ');
    } else if amount > 0.34 && n < amount {
        let chars: &[char] = if ctx.capabilities.unicode_blocks {
            &['░', '▒', '.', ':']
        } else {
            &['.', ':', ' ']
        };
        let index = (noise(y, x, seed.rotate_left(9)) * chars.len() as f64) as usize;
        sample.content.set_char(chars[index.min(chars.len() - 1)]);
    }
}

fn apply_highlight_sweep(
    sample: &mut CellSample,
    ctx: &VisualCtx<'_>,
    color: Color,
    width: f64,
    direction: GradientDirection,
) {
    let width = finite_or(width, 0.18).clamp(f64::EPSILON, 1.0);
    let position = gradient_position(sample.dest_x, sample.dest_y, ctx.area, direction);
    let coverage = 1.0 - ((position - ctx.progress).abs() / width);
    if coverage <= 0.0 {
        return;
    }

    blend_foreground(sample, ctx, color, coverage.clamp(0.0, 1.0));
    add_attribute(&mut sample.content, Attribute::Bold);
}

fn apply_sparkle(
    sample: &mut CellSample,
    ctx: &VisualCtx<'_>,
    seed: u64,
    density: f64,
    color: Color,
) {
    let density = finite_or(density, 0.08).clamp(0.0, 0.35);
    if density <= 0.0 || sample.content.is_whitespace() {
        return;
    }

    let x = sample.source_x as u16;
    let y = sample.source_y as u16;
    let spatial = noise(x, y, seed ^ 0x5FA2_C1E);
    if spatial > density {
        return;
    }

    let frame_seed = seed ^ (ctx.frame / 3).wrapping_mul(0x9E37_79B9);
    let twinkle = noise(y, x, frame_seed);
    if twinkle > 0.55 {
        return;
    }

    apply_color_capable(&mut sample.content, color, GradientTarget::Foreground, ctx);
    add_attribute(&mut sample.content, Attribute::Bold);
    if twinkle < 0.22 {
        let chars = ['*', '+', '.'];
        let index = (noise(x, y, frame_seed.rotate_left(13)) * chars.len() as f64) as usize;
        sample.content.set_char(chars[index.min(chars.len() - 1)]);
    }
}

pub(crate) fn staggered_progress(
    sample: &CellSample,
    ctx: &VisualCtx<'_>,
    delay: f64,
    mode: StaggerMode,
) -> f64 {
    if delay <= 0.0 {
        return ctx.progress;
    }

    let (index, max_index) = match mode {
        StaggerMode::Rows => (
            sample.source_y.round().max(0.0),
            ctx.area.height().saturating_sub(1) as f64,
        ),
        StaggerMode::Cols => (
            sample.source_x.round().max(0.0),
            ctx.area.width().saturating_sub(1) as f64,
        ),
        StaggerMode::Chars => {
            let width = ctx.area.width().max(1) as f64;
            (
                sample.source_y.round().max(0.0) * width + sample.source_x.round().max(0.0),
                (ctx.area.width() as u32 * ctx.area.height() as u32).saturating_sub(1) as f64,
            )
        }
    };

    let max_offset = (delay * max_index).min(0.95);
    let offset = (delay * index).min(max_offset);
    let duration = (1.0 - max_offset).max(f64::EPSILON);
    ((ctx.progress - offset) / duration).clamp(0.0, 1.0)
}
