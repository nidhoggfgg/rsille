use crate::render::area::Area;

use crate::tui::visual_engine::{GradientDirection, VisualCtx, VisualEffect, WipeMode};

pub(crate) fn sanitize_cell_aspect(cell_aspect: f64) -> f64 {
    if cell_aspect.is_finite() {
        cell_aspect.max(f64::EPSILON)
    } else {
        1.0
    }
}

pub(crate) fn sanitize_delay(delay: f64) -> f64 {
    if delay.is_finite() {
        delay.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

pub(crate) fn sanitize_softness(softness: f64) -> f64 {
    finite_or(softness, 0.0).clamp(0.0, 1.0)
}

pub(crate) fn finite_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

pub(crate) fn fade_for_visibility_mode(mode: WipeMode) -> VisualEffect {
    match mode {
        WipeMode::Reveal => VisualEffect::fade_in(),
        WipeMode::Hide => VisualEffect::fade_out(),
    }
}

pub(crate) fn effect_seed(ctx: &VisualCtx<'_>, seed: u64) -> u64 {
    ctx.seed ^ seed
}

pub(crate) fn stable_seed(channel: &str, widget_key: Option<&str>) -> u64 {
    let mut seed = 0xCBF2_9CE4_8422_2325;
    for byte in channel
        .bytes()
        .chain([0])
        .chain(widget_key.unwrap_or("").bytes())
    {
        seed ^= byte as u64;
        seed = seed.wrapping_mul(0x0000_0100_0000_01B3);
    }
    seed
}

pub(crate) fn gradient_position(x: f64, y: f64, area: Area, direction: GradientDirection) -> f64 {
    let width = area.width().saturating_sub(1) as f64;
    let height = area.height().saturating_sub(1) as f64;
    match direction {
        GradientDirection::Horizontal => {
            if width <= 0.0 {
                1.0
            } else {
                x / width
            }
        }
        GradientDirection::Vertical => {
            if height <= 0.0 {
                1.0
            } else {
                y / height
            }
        }
        GradientDirection::Diagonal => {
            if width + height <= 0.0 {
                1.0
            } else {
                (x + y) / (width + height)
            }
        }
    }
    .clamp(0.0, 1.0)
}

pub(crate) fn lerp(start: f64, end: f64, progress: f64) -> f64 {
    start + (end - start) * progress
}

pub(crate) fn ease_out_cubic(t: f64) -> f64 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

pub(crate) fn hash_pair(x: u16, y: u16, seed: u64) -> (f64, f64) {
    (noise(x, y, seed), noise(y, x, seed.rotate_left(17)))
}

pub(crate) fn noise(x: u16, y: u16, seed: u64) -> f64 {
    let mut value = seed
        ^ ((x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
        ^ ((y as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9));
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    (value as f64) / (u64::MAX as f64)
}
