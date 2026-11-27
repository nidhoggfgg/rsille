//! Ambient color math, animated header emblem, and diagonal wave-wipe stage transitions.

use std::f32::consts::TAU;

use crossterm::style::Color;

/// Linearly interpolates between two RGB colors with `t` clamped to `[0.0, 1.0]`.
pub fn lerp_rgb(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = sanitize_unit(t);
    [
        lerp_channel(a[0], b[0], t),
        lerp_channel(a[1], b[1], t),
        lerp_channel(a[2], b[2], t),
    ]
}

/// Returns a `crossterm` TrueColor RGB value interpolated along `a -> b`.
pub fn gradient_color(t: f32, a: [u8; 3], b: [u8; 3]) -> Color {
    let [r, g, b] = lerp_rgb(a, b, t);
    Color::Rgb { r, g, b }
}

/// Evaluates a smooth, iridescent aurora shimmer color along normalized position `pos` at `time`.
pub fn aurora_color(pos: f32, time: f64, primary: [u8; 3], secondary: [u8; 3]) -> [u8; 3] {
    let p = if pos.is_finite() { pos } else { 0.0 };
    let t = if time.is_finite() { time as f32 } else { 0.0 };

    let wave_a = (p * 4.2 - t * 1.45).sin() * 0.5 + 0.5;
    let wave_b = (p * 7.6 + t * 0.92 + 1.35).sin() * 0.5 + 0.5;
    let mix = (0.64 * wave_a + 0.36 * wave_b).clamp(0.0, 1.0);
    let base = lerp_rgb(primary, secondary, mix);

    // Subtle travelling pearl-cyan specular crest keeps header bands luminous.
    let crest = ((p * 3.1 - t * 2.15 + 0.4).sin() * 0.5 + 0.5).powi(6) * 0.24;
    lerp_rgb(base, [236, 250, 255], crest)
}

/// Blends `prev` into `next` with a diagonal wave-wipe, luminous frontier gleam, and cubic ease-out.
///
/// Writes exactly `width * height` row-major top-down RGB pixels into `out`.
pub fn blend_transition(
    prev: &[[u8; 3]],
    next: &[[u8; 3]],
    width: usize,
    height: usize,
    progress: f32,
    accent: [u8; 3],
    out: &mut Vec<[u8; 3]>,
) {
    let len = width.saturating_mul(height);
    out.resize(len, [0, 0, 0]);
    if len == 0 {
        return;
    }

    let p = if progress.is_finite() {
        progress.clamp(0.0, 1.0)
    } else {
        1.0
    };

    if p <= 0.0 {
        copy_padded(prev, out);
        return;
    }
    if p >= 1.0 {
        copy_padded(next, out);
        return;
    }

    // Cubic ease-out for silky crossfade; balanced quadratic ease-out for the diagonal wave
    // frontier so at progress=0.5 the luminous wave cuts dramatically across the stage center.
    let inv = 1.0 - p;
    let eased_fade = 1.0 - inv * inv * inv;
    let wave_t = p * 0.55 + (1.0 - inv * inv) * 0.45;
    let front = -0.18 + wave_t * 1.36;
    let band = 0.16;
    // Smoothly ramps frontier luminescence in and out so endpoints match frames cleanly.
    let gleam_envelope = (p * inv * 4.0).clamp(0.0, 1.0).powf(0.75);
    let gleam_tint = lerp_rgb(accent, [170, 245, 255], 0.38);

    let inv_w = 1.0 / width as f32;
    let inv_h = 1.0 / height as f32;

    for y in 0..height {
        let v = (y as f32 + 0.5) * inv_h;
        let row_wave = 0.052 * (v * 9.5 - p * 8.2).sin();
        let row_offset = y * width;
        for x in 0..width {
            let idx = row_offset + x;
            let u = (x as f32 + 0.5) * inv_w;
            let ripple = row_wave + 0.022 * (u * 13.0 + p * 6.4).cos();
            let diag = u * 0.78 + v * 0.22 + ripple;

            let local = smoothstep(-1.0, 1.0, (front - diag) / band);
            let mix = (local * 0.88 + eased_fade * 0.12).clamp(0.0, 1.0);

            let prev_px = prev.get(idx).copied().unwrap_or([0, 0, 0]);
            let next_px = next.get(idx).copied().unwrap_or([0, 0, 0]);
            let base = lerp_rgb(prev_px, next_px, mix);

            let dist = (front - diag) / 0.054;
            let edge = (-dist * dist).exp() * gleam_envelope;
            if edge > 0.004 {
                let halo = edge * 0.48;
                let core = edge * edge * edge * 0.24;
                out[idx] = [
                    add_light(base[0], gleam_tint[0], halo, core),
                    add_light(base[1], gleam_tint[1], halo, core),
                    add_light(base[2], gleam_tint[2], halo, core),
                ];
            } else {
                out[idx] = base;
            }
        }
    }
}

/// Renders a compact animated prismatic wave emblem or aurora strip for the launcher header.
///
/// Adapts smoothly to both square-ish icons (e.g. `8x4`) and wide strips (e.g. `40x2`).
/// Writes exactly `width * height` row-major top-down RGB pixels into `out`.
pub fn render_emblem(
    width: usize,
    height: usize,
    time: f64,
    primary: [u8; 3],
    secondary: [u8; 3],
    out: &mut Vec<[u8; 3]>,
) {
    let len = width.saturating_mul(height);
    out.resize(len, [0, 0, 0]);
    if len == 0 {
        return;
    }

    let t = if time.is_finite() { time as f32 } else { 0.0 };
    let inv_w = 1.0 / width as f32;
    let inv_h = 1.0 / height as f32;
    let aspect = width as f32 / height as f32;
    let is_strip = aspect >= 3.0;

    // Travelling specular spark along the main aurora crest.
    let spark_u = (t * 0.32).rem_euclid(1.0);
    let spark_v = 0.5 + 0.24 * (spark_u * TAU * 1.15 - t * 2.1).sin();

    let p_f = rgb_to_f32(primary);
    let s_f = rgb_to_f32(secondary);

    for y in 0..height {
        let v = (y as f32 + 0.5) * inv_h;
        let row_offset = y * width;
        for x in 0..width {
            let u = (x as f32 + 0.5) * inv_w;

            // Deep obsidian-midnight base with subtle soft radial glow.
            let dx = (u - 0.5) * if is_strip { 1.2 } else { 1.75 };
            let dy = (v - 0.5) * 1.85;
            let r2 = dx * dx + dy * dy;
            let ambient = (-r2 * 1.8).exp() * 0.24;

            let wave_freq = if is_strip { 1.45 } else { 1.05 };
            let y1 = 0.50 + 0.25 * (u * TAU * wave_freq - t * 2.1).sin();
            let y2 = 0.50 + 0.23 * (u * TAU * (wave_freq * 1.55) + t * 1.55 + 1.1).cos();

            let d1 = (v - y1) * 4.4;
            let d2 = (v - y2) * 4.8;
            let ribbon1 = (-d1 * d1).exp();
            let ribbon2 = (-d2 * d2).exp();

            // Soft prismatic interference between primary and secondary waves.
            let hue_t = (0.5 + 0.5 * (u * 3.8 - t * 1.2 + (v - 0.5) * 2.0).sin()).clamp(0.0, 1.0);
            let tint1 = [
                p_f[0] + (s_f[0] - p_f[0]) * (hue_t * 0.45),
                p_f[1] + (s_f[1] - p_f[1]) * (hue_t * 0.45),
                p_f[2] + (s_f[2] - p_f[2]) * (hue_t * 0.45),
            ];
            let tint2 = [
                s_f[0] + (p_f[0] - s_f[0]) * ((1.0 - hue_t) * 0.4),
                s_f[1] + (p_f[1] - s_f[1]) * ((1.0 - hue_t) * 0.4),
                s_f[2] + (p_f[2] - s_f[2]) * ((1.0 - hue_t) * 0.4),
            ];

            // Subtle border framing on icon-sized emblems.
            let frame_glow = if is_strip {
                1.0
            } else {
                (1.0 - r2 * 0.32).clamp(0.42, 1.0)
            };

            let sdx = (u - spark_u) * aspect.min(4.0);
            let sdy = v - spark_v;
            let spark = (-(sdx * sdx + sdy * sdy) * 22.0).exp() * 0.36;
            let core_overlap = (ribbon1 * ribbon2).sqrt() * 0.16;

            let bg = [0.028, 0.042, 0.088];
            let mut rgb = [0.0_f32; 3];
            for c in 0..3 {
                let amb = (p_f[c] * 0.55 + s_f[c] * 0.45) * ambient * 0.72;
                let glow = tint1[c] * ribbon1 * 0.62 + tint2[c] * ribbon2 * 0.48;
                let highlight = (core_overlap + spark) * (0.52 + 0.48 * tint1[c]);
                let raw = bg[c] + (amb + glow + highlight) * frame_glow;
                // Gentle shoulder keeps chromatic hues vivid without blowing out to white.
                rgb[c] = (raw / (1.0 + raw * 0.24)).clamp(0.0, 1.0);
            }
            out[row_offset + x] = f32_to_rgb(rgb);
        }
    }
}

fn sanitize_unit(t: f32) -> f32 {
    if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn lerp_channel(a: u8, b: u8, t: f32) -> u8 {
    let a = f32::from(a);
    let b = f32::from(b);
    (a + (b - a) * t).round().clamp(0.0, 255.0) as u8
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn add_light(base: u8, tint: u8, halo: f32, core: f32) -> u8 {
    let v = f32::from(base) + f32::from(tint) * halo + 255.0 * core;
    v.round().clamp(0.0, 255.0) as u8
}

fn copy_padded(src: &[[u8; 3]], out: &mut [[u8; 3]]) {
    let n = src.len().min(out.len());
    out[..n].copy_from_slice(&src[..n]);
    for px in &mut out[n..] {
        *px = [0, 0, 0];
    }
}

fn rgb_to_f32([r, g, b]: [u8; 3]) -> [f32; 3] {
    [
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0,
    ]
}

fn f32_to_rgb([r, g, b]: [f32; 3]) -> [u8; 3] {
    [
        (r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (b.clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerp_and_gradient_handle_endpoints_midpoints_and_nonfinite() {
        assert_eq!(lerp_rgb([10, 20, 30], [210, 120, 230], 0.0), [10, 20, 30]);
        assert_eq!(lerp_rgb([10, 20, 30], [210, 120, 230], 1.0), [210, 120, 230]);
        assert_eq!(lerp_rgb([0, 100, 200], [100, 200, 0], 0.5), [50, 150, 100]);
        assert_eq!(lerp_rgb([10, 20, 30], [200, 200, 200], -2.0), [10, 20, 30]);
        assert_eq!(lerp_rgb([10, 20, 30], [200, 200, 200], 4.5), [200, 200, 200]);
        assert_eq!(
            lerp_rgb([10, 20, 30], [200, 200, 200], f32::NAN),
            [10, 20, 30]
        );
        assert_eq!(
            gradient_color(0.5, [0, 100, 200], [100, 200, 0]),
            Color::Rgb {
                r: 50,
                g: 150,
                b: 100,
            }
        );
    }

    #[test]
    fn aurora_color_varies_smoothly_across_space_and_time() {
        let p = [72, 224, 255];
        let s = [255, 108, 152];
        let a = aurora_color(0.0, 0.0, p, s);
        let b = aurora_color(0.45, 0.0, p, s);
        let c = aurora_color(0.0, 1.3, p, s);
        assert_ne!(a, b);
        assert_ne!(a, c);
        // Non-finite inputs never panic.
        let _ = aurora_color(f32::NAN, f64::INFINITY, p, s);
    }

    #[test]
    fn blend_transition_matches_endpoints_gleams_midway_and_handles_edge_sizes() {
        let prev = vec![[12, 24, 48]; 24 * 12];
        let next = vec![[40, 18, 36]; 24 * 12];
        let accent = [72, 224, 255];
        let mut out = Vec::new();

        blend_transition(&prev, &next, 24, 12, 0.0, accent, &mut out);
        assert_eq!(out, prev);

        blend_transition(&prev, &next, 24, 12, 1.0, accent, &mut out);
        assert_eq!(out, next);

        blend_transition(&prev, &next, 24, 12, 0.5, accent, &mut out);
        assert_eq!(out.len(), 24 * 12);
        // Luminous frontier gleam should brighten pixels along the diagonal wave beyond both inputs.
        assert!(out.iter().any(|px| px[2] > 90 && px[1] > 60));

        // Edge cases: empty, mismatched slice lengths, NaN progress.
        blend_transition(&[], &[], 0, 10, 0.5, accent, &mut out);
        assert!(out.is_empty());
        blend_transition(&prev[..5], &next[..3], 8, 4, 0.5, accent, &mut out);
        assert_eq!(out.len(), 32);
        blend_transition(&prev, &next, 24, 12, f32::NAN, accent, &mut out);
        assert_eq!(out, next);
    }

    #[test]
    fn render_emblem_animates_across_icon_and_strip_shapes() {
        let mut a = Vec::new();
        let mut b = Vec::new();
        for (w, h) in [(0, 0), (1, 1), (8, 4), (36, 2)] {
            render_emblem(w, h, 0.2, [72, 224, 255], [255, 108, 152], &mut a);
            render_emblem(w, h, 1.4, [72, 224, 255], [255, 108, 152], &mut b);
            assert_eq!(a.len(), w * h);
            assert_eq!(b.len(), w * h);
            if w * h > 1 {
                assert_ne!(a, b, "emblem should animate for {w}x{h}");
            }
        }
    }
}
