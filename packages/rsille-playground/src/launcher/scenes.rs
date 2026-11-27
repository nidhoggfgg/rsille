//! Real-time preview graphics engine for all four `rsille-playground` showcase worlds.

use std::f32::consts::{PI, TAU};

use crate::{
    ai_babies::scene as babies_scene,
    function_plotter::{plot::Plot, X_RANGE},
    jellyfish::sea::Sea,
};

#[cfg(any(unix, windows))]
use crate::deepseek_pet::animation as pet_anim;

/// Identifies one of the four showcase experiences in the interactive launcher.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExperienceId {
    Jellyfish,
    FunctionPlotter,
    AiBabies,
    DeepseekPet,
}

impl ExperienceId {
    /// Ordered list of all showcase experiences.
    pub const ALL: [ExperienceId; 4] = [
        Self::Jellyfish,
        Self::FunctionPlotter,
        Self::AiBabies,
        Self::DeepseekPet,
    ];

    /// Zero-based index into [`ExperienceId::ALL`].
    pub fn index(self) -> usize {
        match self {
            Self::Jellyfish => 0,
            Self::FunctionPlotter => 1,
            Self::AiBabies => 2,
            Self::DeepseekPet => 3,
        }
    }

    /// Maps an arbitrary index into an experience by wrapping smoothly around [`ExperienceId::ALL`].
    pub fn from_index(index: usize) -> Self {
        Self::ALL[index % Self::ALL.len()]
    }

    /// Primary luminous accent color for UI chrome, borders, and transition gleams.
    pub fn accent_rgb(self) -> [u8; 3] {
        match self {
            Self::Jellyfish => [72, 224, 255],
            Self::FunctionPlotter => [130, 198, 255],
            Self::AiBabies => [196, 148, 255],
            Self::DeepseekPet => [86, 178, 255],
        }
    }

    /// Secondary complementary highlight color for chromatic gradients and aurora strips.
    pub fn secondary_rgb(self) -> [u8; 3] {
        match self {
            Self::Jellyfish => [255, 108, 152],
            Self::FunctionPlotter => [255, 184, 96],
            Self::AiBabies => [106, 232, 198],
            Self::DeepseekPet => [108, 242, 232],
        }
    }

    /// Number of interactive preview variants supported by this experience.
    pub fn variant_count(self) -> usize {
        match self {
            Self::Jellyfish => 3,
            Self::FunctionPlotter => 4,
            Self::AiBabies => 4,
            Self::DeepseekPet => 3,
        }
    }
}

/// Interactive preview state driven by user pulses and variant cycling.
#[derive(Clone, Copy, Debug, Default)]
pub struct PreviewInteraction {
    pub variant: usize,
    pub action_pulse: f64,
    pub action_count: u64,
}

struct PlotPreset {
    plot: Plot,
    bounds: [f64; 4],
    is_spatial: bool,
    is_curve_3d: bool,
}

const PLOT_PRESET_SOURCES: [&str; 4] = [
    "surface(sin(x)*cos(y), -pi, pi, -pi, pi)",
    "surface((x^2-y^2)/3, -2, 2, -2, 2)",
    "param3((2+cos(3*t))*cos(2*t), (2+cos(3*t))*sin(2*t), sin(3*t), 0, 2*pi)",
    "polar(1+cos(theta), 0, 2*pi); polar(sin(3*theta), 0, 2*pi)",
];

const BABIES_SPOTLIGHT_ACTIONS: [babies_scene::Action; 5] = [
    babies_scene::Action::Wave,
    babies_scene::Action::Hop,
    babies_scene::Action::Think,
    babies_scene::Action::Walk,
    babies_scene::Action::Idle,
];

/// Stateful real-time graphics engine that renders all four launcher preview stages into RGB buffers.
pub struct SceneEngine {
    sea: Sea,
    plots: Vec<PlotPreset>,
}

impl Default for SceneEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneEngine {
    /// Initializes reusable scene renderers and pre-parses all function plotter presets once.
    pub fn new() -> Self {
        let plots = PLOT_PRESET_SOURCES
            .iter()
            .enumerate()
            .map(|(index, source)| {
                let plot = Plot::parse(source).expect("builtin launcher plot preset must parse");
                let bounds = plot.auto_bounds(X_RANGE);
                let is_spatial = plot.is_spatial();
                PlotPreset {
                    plot,
                    bounds,
                    is_spatial,
                    is_curve_3d: index == 2,
                }
            })
            .collect();
        Self {
            sea: Sea::default(),
            plots,
        }
    }

    /// Renders `exp` into `out` as `width * height` row-major top-down TrueColor RGB pixels.
    ///
    /// Never panics for `0x0`, tiny, or unusually large viewports, nor for non-finite timestamps.
    pub fn render_into(
        &mut self,
        exp: ExperienceId,
        width: usize,
        height: usize,
        time: f64,
        interaction: PreviewInteraction,
        out: &mut Vec<[u8; 3]>,
    ) {
        let len = width.saturating_mul(height);
        out.resize(len, [0, 0, 0]);
        if len == 0 {
            return;
        }
        let time = if time.is_finite() { time } else { 0.0 };
        match exp {
            ExperienceId::Jellyfish => self.render_jellyfish(width, height, time, interaction, out),
            ExperienceId::FunctionPlotter => {
                self.render_function_plotter(width, height, time, interaction, out)
            }
            ExperienceId::AiBabies => self.render_ai_babies(width, height, time, interaction, out),
            ExperienceId::DeepseekPet => {
                self.render_deepseek_pet(width, height, time, interaction, out)
            }
        }
    }

    /// Short display label describing the currently active preview variant.
    pub fn variant_label(exp: ExperienceId, variant: usize) -> &'static str {
        let idx = variant % exp.variant_count();
        match exp {
            ExperienceId::Jellyfish => match idx {
                0 => "Abyssal Cyan",
                1 => "Biolum Violet",
                _ => "Emerald Deep",
            },
            ExperienceId::FunctionPlotter => match idx {
                0 => "sin(x)·cos(y) 3D",
                1 => "Saddle (x²-y²)/3",
                2 => "Trefoil Knot 3D",
                _ => "Polar Rose + Cardioid",
            },
            ExperienceId::AiBabies => match idx {
                0 => "All 3 · Studio",
                1 => "ChatGPT · Spotlight",
                2 => "DeepSeek · Spotlight",
                _ => "Claude · Spotlight",
            },
            ExperienceId::DeepseekPet => match idx {
                0 => "Ocean Patrol",
                1 => "Playful Hop",
                _ => "Cozy Nap",
            },
        }
    }

    /// Concise contextual hint telling the user what `Space` does on the live stage.
    pub fn interaction_hint(exp: ExperienceId) -> &'static str {
        match exp {
            ExperienceId::Jellyfish => "Space: biolum pulse & palette",
            ExperienceId::FunctionPlotter => "Space: spin boost & next plot",
            ExperienceId::AiBabies => "Space: celebrate & spotlight",
            ExperienceId::DeepseekPet => "Space: splash hop & pet whale",
        }
    }

    /// Optional live HUD caption or speech reaction overlaid onto the stage.
    pub fn overlay_caption(
        exp: ExperienceId,
        time: f64,
        interaction: PreviewInteraction,
    ) -> Option<&'static str> {
        let time = if time.is_finite() { time } else { 0.0 };
        let variant = interaction.variant % exp.variant_count();

        if let Some(dt) = pulse_age(time, interaction) {
            if dt < 2.2 {
                return Some(match exp {
                    ExperienceId::Jellyfish => match interaction.action_count % 3 {
                        0 => "✦ Bioluminescent surge",
                        1 => "✦ Deep current shift",
                        _ => "✦ Abyssal ripple",
                    },
                    ExperienceId::FunctionPlotter => {
                        if variant == 3 {
                            "✦ Polar harmonic sweep"
                        } else {
                            "✦ Orbit boost · 3D shaded"
                        }
                    }
                    ExperienceId::AiBabies => match interaction.action_count % 3 {
                        0 => "✦ Studio encore hop!",
                        1 => "✦ High-five wave!",
                        _ => "✦ Sparkle burst!",
                    },
                    ExperienceId::DeepseekPet => match interaction.action_count % 3 {
                        0 => "~ Splash! Happy whale!",
                        1 => "~ *bubble bubble* ♪",
                        _ => "~ Ready to swim in your shell!",
                    },
                });
            }
        }

        match exp {
            ExperienceId::DeepseekPet => {
                if variant == 2 {
                    let phase = time.rem_euclid(6.0);
                    if phase < 3.6 {
                        return Some("~ zzz... drifting in warm currents");
                    }
                    return None;
                }
                let phase = time.rem_euclid(12.0);
                if phase < 3.4 {
                    Some("~ patrolling your terminal...")
                } else if (6.0..9.4).contains(&phase) {
                    Some("~ feed me shell output!")
                } else {
                    None
                }
            }
            ExperienceId::AiBabies if variant > 0 => {
                let action = spotlight_action_at(time);
                Some(match (variant, action) {
                    (1, babies_scene::Action::Wave) => "ChatGPT · waving hello",
                    (1, babies_scene::Action::Hop) => "ChatGPT · happy hop",
                    (1, babies_scene::Action::Think) => "ChatGPT · thinking...",
                    (1, babies_scene::Action::Walk) => "ChatGPT · strolling",
                    (1, _) => "ChatGPT · vibing",
                    (2, babies_scene::Action::Wave) => "DeepSeek · waving hello",
                    (2, babies_scene::Action::Hop) => "DeepSeek · happy hop",
                    (2, babies_scene::Action::Think) => "DeepSeek · deep thought...",
                    (2, babies_scene::Action::Walk) => "DeepSeek · strolling",
                    (2, _) => "DeepSeek · vibing",
                    (_, babies_scene::Action::Wave) => "Claude · waving hello",
                    (_, babies_scene::Action::Hop) => "Claude · happy hop",
                    (_, babies_scene::Action::Think) => "Claude · pondering...",
                    (_, babies_scene::Action::Walk) => "Claude · strolling",
                    (_, _) => "Claude · vibing",
                })
            }
            _ => None,
        }
    }

    fn render_jellyfish(
        &mut self,
        width: usize,
        height: usize,
        time: f64,
        interaction: PreviewInteraction,
        out: &mut [[u8; 3]],
    ) {
        let variant = interaction.variant % ExperienceId::Jellyfish.variant_count();
        let pulse = pulse_age(time, interaction).filter(|&dt| dt < 1.8);
        // Gentle motion surge during a bioluminescent pulse.
        let sea_time = time + pulse.map_or(0.0, |dt| 0.55 * (1.0 - (-dt * 2.5).exp()));

        // 2x2 supersampling keeps delicate tentacles silky at terminal HalfBlock resolution;
        // cap only for pathologically huge synthetic test viewports.
        let ss = if width.saturating_mul(height) <= 24_000 {
            2
        } else {
            1
        };
        let sw = width * ss;
        let sh = height * ss;
        self.sea.render(sw, sh, sea_time);

        let inv_w = 1.0 / width as f32;
        let inv_h = 1.0 / height as f32;
        let aspect = (width as f32 * inv_h).max(0.5);

        let ripple_params = pulse.map(|dt| {
            let dt = dt as f32;
            let prog = (dt / 1.8).clamp(0.0, 1.0);
            let ring_r = 0.04 + prog.powf(0.74) * 0.78;
            let fade = (1.0 - prog) * (1.0 - prog) * (dt * 9.0).min(1.0);
            let boost = (-dt * 2.8).exp() * 0.24;
            (dt, ring_r, fade, boost)
        });
        let ring_tint = match variant {
            0 => [0.24, 0.92, 1.00],
            1 => [0.96, 0.74, 1.00],
            _ => [0.35, 1.00, 0.76],
        };

        for y in 0..height {
            let v = (y as f32 + 0.5) * inv_h;
            let row_offset = y * width;
            for x in 0..width {
                let mut rgb = [0.0_f32; 3];
                if ss == 2 {
                    let base_y = y * 2;
                    let base_x = x * 2;
                    let p00 = self.sea.pixels[base_y * sw + base_x];
                    let p10 = self.sea.pixels[base_y * sw + base_x + 1];
                    let p01 = self.sea.pixels[(base_y + 1) * sw + base_x];
                    let p11 = self.sea.pixels[(base_y + 1) * sw + base_x + 1];
                    for c in 0..3 {
                        rgb[c] = (p00[c] + p10[c] + p01[c] + p11[c]) * 0.25;
                    }
                } else {
                    rgb = self.sea.pixels[y * sw + x];
                }

                // Apply color-current palette grading in linear space before clamping.
                rgb = grade_jellyfish(rgb, variant);

                // Expanding bioluminescent ripple ring & luminous bloom on Space pulse.
                if let Some((dt, ring_r, fade, boost)) = ripple_params {
                    let u = (x as f32 + 0.5) * inv_w;
                    let dx = (u - 0.50) * aspect * 0.62;
                    let dy = v - 0.44;
                    let dist = dx.hypot(dy);
                    let angle = dy.atan2(dx);
                    let wobble = 0.016 * (angle * 5.0 + dt * 4.2).sin();
                    let d_outer = (dist - (ring_r + wobble)) / 0.044;
                    let d_inner = (dist - (ring_r * 0.66 - wobble * 0.7)) / 0.036;
                    let outer_ring = (-d_outer * d_outer).exp();
                    let inner_ring = (-d_inner * d_inner).exp() * 0.52;
                    let sparkle = 0.72 + 0.28 * (angle * 11.0 - dt * 6.5).sin().powi(2);
                    let wave = (outer_ring + inner_ring) * fade * sparkle;
                    let luma = rgb[0] * 0.3 + rgb[1] * 0.55 + rgb[2] * 0.15;
                    for c in 0..3 {
                        rgb[c] += ring_tint[c] * wave * 0.72 + rgb[c] * luma * boost;
                    }
                }

                out[row_offset + x] = f32_to_rgb(rgb);
            }
        }
    }

    fn render_function_plotter(
        &mut self,
        width: usize,
        height: usize,
        time: f64,
        interaction: PreviewInteraction,
        out: &mut [[u8; 3]],
    ) {
        let variant = interaction.variant % self.plots.len();
        let preset = &self.plots[variant];

        let pulse = pulse_age(time, interaction).filter(|&dt| dt < 2.2);
        let spin_boost = pulse.map_or(0.0, |dt| 2.4 * (1.0 - (-dt * 2.3).exp()));
        let glow_pulse = pulse.map_or(0.0_f32, |dt| (-(dt as f32) * 1.9).exp());

        let angle = if preset.is_spatial {
            (time * 0.68 + spin_boost).rem_euclid(std::f64::consts::TAU)
        } else {
            0.0
        };

        // Keep plot render size under 160x80 so Plot::View does not reserve wide 60px text-tick
        // margins and frame times remain well below 10ms while achieving 2x2 supersampling on
        // typical terminal stage viewports.
        let (rw, rh) = choose_plot_resolution(width, height);
        let canvas = preset.plot.image_at(preset.bounds, (rw, rh), angle);
        let src = canvas.pixels();
        let rw_usize = rw as usize;
        let rh_usize = rh as usize;

        let inv_w = 1.0 / width as f32;
        let inv_h = 1.0 / height as f32;
        let t = time as f32;

        // First resample the Plot canvas cleanly into `out`, enriching background & vignette.
        for y in 0..height {
            let v = (y as f32 + 0.5) * inv_h;
            let sy0 = y * rh_usize / height;
            let sy1 = ((y + 1) * rh_usize / height).max(sy0 + 1).min(rh_usize);
            let row_offset = y * width;

            for x in 0..width {
                let u = (x as f32 + 0.5) * inv_w;
                let sx0 = x * rw_usize / width;
                let sx1 = ((x + 1) * rw_usize / width).max(sx0 + 1).min(rw_usize);

                let mut sr = 0.0_f32;
                let mut sg = 0.0_f32;
                let mut sb = 0.0_f32;
                let mut count = 0.0_f32;
                for sy in sy0..sy1 {
                    let row = sy * rw_usize;
                    for sx in sx0..sx1 {
                        let px = src[row + sx];
                        sr += f32::from(px.0);
                        sg += f32::from(px.1);
                        sb += f32::from(px.2);
                        count += 1.0;
                    }
                }
                let inv_c = 1.0 / (count * 255.0);
                let mut r = sr * inv_c;
                let mut g = sg * inv_c;
                let mut b = sb * inv_c;

                // Measure how far this pixel is from Plot's flat background RGB(18, 22, 30).
                let bg_dist = ((r - 18.0 / 255.0).abs()
                    + (g - 22.0 / 255.0).abs()
                    + (b - 30.0 / 255.0).abs())
                .min(1.0);

                // Radial optical-lab backdrop vignette behind the math geometry.
                let dx = (u - 0.5) * 1.45;
                let dy = (v - 0.5) * 1.65;
                let radial = (-1.35 * (dx * dx + dy * dy)).exp();
                let vignette = 0.58 + 0.42 * radial;

                // Deepen flat background into rich midnight sapphire with soft radial spotlight.
                let studio_bg = [
                    0.030 + 0.032 * radial + 0.015 * glow_pulse * radial,
                    0.044 + 0.056 * radial + 0.028 * glow_pulse * radial,
                    0.086 + 0.095 * radial + 0.045 * glow_pulse * radial,
                ];

                if bg_dist < 0.025 {
                    r = studio_bg[0];
                    g = studio_bg[1];
                    b = studio_bg[2];
                } else {
                    // Subtle atmospheric blend on grid lines, vibrant contrast lift on surfaces/curves.
                    let fg_strength = ((bg_dist - 0.025) / 0.18).clamp(0.0, 1.0);
                    let lift = 1.0 + 0.16 * fg_strength + 0.22 * glow_pulse * fg_strength;
                    let mut fr = r * lift;
                    let mut fg = g * lift;
                    let mut fb = b * lift;

                    // Subtle luminance-keyed chromatic richness on shaded 3D surfaces so ridges
                    // and valleys pop dramatically in HalfBlock TrueColor.
                    if preset.is_spatial && !preset.is_curve_3d && fg_strength > 0.35 {
                        let luma = (r * 0.25 + g * 0.55 + b * 0.20).clamp(0.0, 1.0);
                        if variant == 0 {
                            // Wave surface: deep sapphire valleys -> luminous aqua-ice peaks.
                            let ridge = ((luma - 0.42) / 0.45).clamp(0.0, 1.0);
                            fr = fr * (0.76 + 0.52 * ridge) + 0.08 * ridge;
                            fg = fg * (0.86 + 0.24 * ridge) + 0.04 * ridge;
                            fb *= 1.06 - 0.04 * ridge;
                        } else {
                            // Hyperbolic saddle: warm violet-indigo shadow -> molten amber-rose crest.
                            let crest = ((luma - 0.36) / 0.48).clamp(0.0, 1.0);
                            fr = (0.34 + 0.72 * crest) * (0.45 + 0.65 * luma);
                            fg = (0.28 + 0.56 * crest) * (0.45 + 0.62 * luma);
                            fb = (0.88 - 0.36 * crest) * (0.50 + 0.58 * luma);
                        }
                    }

                    r = (studio_bg[0] * (1.0 - fg_strength) + fr * fg_strength)
                        * (0.84 + 0.16 * vignette);
                    g = (studio_bg[1] * (1.0 - fg_strength) + fg * fg_strength)
                        * (0.84 + 0.16 * vignette);
                    b = (studio_bg[2] * (1.0 - fg_strength) + fb * fg_strength)
                        * (0.84 + 0.16 * vignette);

                    // Travelling luminous harmonic shimmer on the 2D polar preset so it breathes at 30 FPS.
                    if !preset.is_spatial && fg_strength > 0.35 {
                        let theta = (v - 0.5).atan2((u - 0.5) * (width as f32 * inv_h));
                        let shimmer = ((theta * 2.0 - t * 2.6).sin() * 0.5 + 0.5).powi(3)
                            * (0.24 + 0.35 * glow_pulse);
                        r += 0.30 * shimmer;
                        g += 0.24 * shimmer;
                        b += 0.36 * shimmer;
                    }
                }

                // Subtle expanding pulse wave on Space press for both 3D and 2D presets.
                if let Some(dt) = pulse {
                    let dt = dt as f32;
                    let ring_r = 0.05 + (dt / 2.2).powf(0.75) * 0.85;
                    let dist = ((u - 0.5) * 1.15).hypot(v - 0.5);
                    let ring = (-((dist - ring_r) / 0.042).powi(2)).exp()
                        * (1.0 - dt / 2.2).powi(2)
                        * 0.38;
                    r += 0.45 * ring;
                    g += 0.72 * ring;
                    b += 1.00 * ring;
                }

                out[row_offset + x] = f32_to_rgb([r, g, b]);
            }
        }

        // For the 3D trefoil space curve, add a light micro-bloom halo so thin neon ribbons pop
        // dramatically in HalfBlock resolution.
        if preset.is_curve_3d && width >= 4 && height >= 4 {
            apply_subtle_bloom(out, width, height, 0.28);
        }

        // For the 2D polar rose & cardioid preset, trace a luminous orbiting bead along the
        // cardioid curve so the viewer sees live harmonic motion immediately.
        if !preset.is_spatial && width >= 8 && height >= 8 {
            let aspect = width as f32 / height as f32;
            let theta = t * 1.35 + spin_boost as f32;
            let radius = (1.0 + theta.cos()) * 0.22;
            let cx = 0.42 + (radius * theta.cos()) / aspect.max(0.8);
            let cy = 0.50 - radius * theta.sin();
            draw_soft_sparkle(out, width, height, cx, cy, 0.065, [150, 236, 255], 0.85);
            let theta2 = -t * 1.85;
            let r2 = (3.0 * theta2).sin() * 0.25;
            let cx2 = 0.42 + (r2 * theta2.cos()) / aspect.max(0.8);
            let cy2 = 0.50 - r2 * theta2.sin();
            draw_soft_sparkle(out, width, height, cx2, cy2, 0.055, [255, 196, 118], 0.78);
        }
    }

    fn render_ai_babies(
        &mut self,
        width: usize,
        height: usize,
        time: f64,
        interaction: PreviewInteraction,
        out: &mut [[u8; 3]],
    ) {
        let variant = interaction.variant % ExperienceId::AiBabies.variant_count();
        let pulse = pulse_age(time, interaction).filter(|&dt| dt < 1.65);
        let t = time as f32;
        let floor_y = (height as f32 * 0.77).clamp(1.0, height as f32);
        let inv_w = 1.0 / width as f32;
        let inv_h = 1.0 / height as f32;

        // Determine spotlight character(s) and their neon brand aura colors.
        let spotlight_aura = match variant {
            1 => [0.25, 0.91, 0.73], // ChatGPT emerald-mint
            2 => [0.31, 0.68, 1.00], // DeepSeek sapphire-cyan
            3 => [1.00, 0.64, 0.42], // Claude coral-amber
            _ => [0.76, 0.58, 1.00], // Studio violet-lavender
        };
        let pulse_flare = pulse.map_or(0.0_f32, |dt| {
            let dt = dt as f32;
            (-dt * 2.2).exp() * (dt * 12.0).min(1.0)
        });

        // 1. Rich midnight-indigo studio backdrop + volumetric overhead spotlights + neon floor.
        for y in 0..height {
            let v = (y as f32 + 0.5) * inv_h;
            let row_offset = y * width;
            let on_floor = (y as f32 + 0.5) >= floor_y;

            for x in 0..width {
                let u = (x as f32 + 0.5) * inv_w;
                let mut rgb = if !on_floor {
                    // Velvet midnight-indigo studio wall warming softly toward horizon.
                    let h_t = (v / 0.75).clamp(0.0, 1.0);
                    let vig = (1.0 - (u - 0.5).powi(2) * 1.15).clamp(0.45, 1.0);
                    [
                        (0.036 + h_t * 0.048) * vig,
                        (0.042 + h_t * 0.038) * vig,
                        (0.112 + h_t * 0.088) * vig,
                    ]
                } else {
                    // Glossy dark neon studio dancefloor with subtle perspective grid.
                    let depth = ((v - 0.75) / 0.25).clamp(0.0, 1.0);
                    let persp_x = (u - 0.5) / (0.38 + depth * 1.15);
                    let grid_x = ((persp_x * 6.0).fract().abs() - 0.5).abs();
                    let line_x = (1.0 - (grid_x * 16.0).min(1.0)).powi(2);
                    let grid_z = ((depth * 4.5 - t * 0.35).rem_euclid(1.0) - 0.5).abs();
                    let line_z = (1.0 - (grid_z * 14.0).min(1.0)).powi(2);
                    let grid = (line_x * 0.55 + line_z * 0.45) * (0.28 + 0.35 * (1.0 - depth));
                    let horizon_band = (-((v - 0.75) / 0.035).powi(2)).exp();
                    [
                        0.045 + spotlight_aura[0] * (grid * 0.22 + horizon_band * 0.28),
                        0.048 + spotlight_aura[1] * (grid * 0.22 + horizon_band * 0.28),
                        0.125 + spotlight_aura[2] * (grid * 0.25 + horizon_band * 0.30),
                    ]
                };

                // Soft volumetric spotlight beams & neon floor pools.
                if variant == 0 {
                    const TRIO: [(f32, [f32; 3]); 3] = [
                        (0.20, [0.24, 0.90, 0.72]),
                        (0.50, [0.32, 0.70, 1.00]),
                        (0.80, [1.00, 0.64, 0.42]),
                    ];
                    for (i, &(cx, aura)) in TRIO.iter().enumerate() {
                        let sway = 0.025 * (t * 0.7 + i as f32 * 2.1).sin();
                        let cone_w = 0.11 + v * 0.13;
                        let beam = (-((u - (cx + sway)) / cone_w).powi(2)).exp()
                            * (1.0 - v * 0.45)
                            * (0.14 + 0.14 * pulse_flare);
                        let pool = if on_floor {
                            let dx = (u - cx) / 0.15;
                            let dy = (v - 0.80) / 0.085;
                            (-(dx * dx + dy * dy)).exp() * (0.34 + 0.22 * pulse_flare)
                        } else {
                            0.0
                        };
                        for c in 0..3 {
                            rgb[c] += aura[c] * (beam + pool);
                        }
                    }
                } else {
                    let sway = 0.03 * (t * 0.8).sin();
                    let cone_w = 0.16 + v * 0.24;
                    let beam = (-((u - (0.50 + sway)) / cone_w).powi(2)).exp()
                        * (1.0 - v * 0.35)
                        * (0.22 + 0.22 * pulse_flare);
                    let halo_dx = (u - 0.50) * 1.4;
                    let halo_dy = (v - 0.44) * 1.3;
                    let radial = (-(halo_dx * halo_dx + halo_dy * halo_dy) * 3.0).exp()
                        * (0.20 + 0.28 * pulse_flare);
                    let pool = if on_floor {
                        let dx = (u - 0.50) / 0.28;
                        let dy = (v - 0.81) / 0.10;
                        (-(dx * dx + dy * dy)).exp() * (0.42 + 0.25 * pulse_flare)
                    } else {
                        0.0
                    };
                    for c in 0..3 {
                        rgb[c] += spotlight_aura[c] * (beam + radial + pool);
                    }
                }

                out[row_offset + x] = f32_to_rgb(rgb);
            }
        }

        // 2. Composite character sprites (with subtle floor reflection directly underneath!).
        if variant == 0 {
            // All 3 AI Babies performing together on the studio stage.
            let sprite_side = ((height as f32 * 0.72).min(width as f32 * 0.36)).max(4.0) as usize;
            let trio = [
                (babies_scene::Character::ChatGpt, 0.20_f32, 0.0_f64),
                (babies_scene::Character::DeepSeek, 0.50_f32, 2.3_f64),
                (babies_scene::Character::Claude, 0.80_f32, 4.7_f64),
            ];
            for (i, &(character, cx_frac, phase_offset)) in trio.iter().enumerate() {
                let (action, action_time) = if let Some(dt) = pulse {
                    (babies_scene::Action::Hop, dt)
                } else {
                    let action = trio_action_at(time + phase_offset, i);
                    (action, time + phase_offset)
                };
                let bob = if let Some(dt) = pulse {
                    let p = (dt as f32 / 0.85).min(1.0);
                    -((p * PI).sin() * (height as f32 * 0.075))
                } else {
                    ((t * 3.2 + i as f32 * 1.9).sin() * 0.45).round()
                };
                let center_x = (width as f32 * cx_frac).round() as isize;
                let foot_y = (floor_y + 1.0 + bob).max(sprite_side as f32).round() as isize;
                blit_ai_baby_with_reflection(
                    out,
                    width,
                    height,
                    character,
                    action,
                    action_time,
                    center_x,
                    foot_y,
                    floor_y.round() as isize,
                    sprite_side,
                );
            }
        } else {
            let character = match variant {
                1 => babies_scene::Character::ChatGpt,
                2 => babies_scene::Character::DeepSeek,
                _ => babies_scene::Character::Claude,
            };
            let sprite_side = ((height as f32 * 0.78).min(width as f32 * 0.74)).max(4.0) as usize;
            let (action, action_time) = if let Some(dt) = pulse {
                (babies_scene::Action::Hop, dt)
            } else {
                (spotlight_action_at(time), time)
            };
            let lift = pulse.map_or(0.0_f32, |dt| {
                let p = (dt as f32 / 0.9).min(1.0);
                -((p * PI).sin() * (height as f32 * 0.075))
            });
            let center_x = (width / 2) as isize;
            let foot_y = (floor_y + 2.0 + lift).max(sprite_side as f32 + 1.0).round() as isize;
            blit_ai_baby_with_reflection(
                out,
                width,
                height,
                character,
                action,
                action_time,
                center_x,
                foot_y,
                floor_y.round() as isize,
                sprite_side,
            );
        }

        // 3. Rising pixel sparkles + celebratory sparkle burst on Space pulse.
        const SPARKLE_COLORS: [[u8; 3]; 4] = [
            [115, 250, 215],
            [135, 215, 255],
            [232, 178, 255],
            [255, 214, 138],
        ];
        for i in 0..18 {
            let speed = 0.08 + hash01(i + 11) * 0.10;
            let u = (hash01(i + 37) + 0.025 * (t * 1.4 + i as f32).sin()).rem_euclid(1.0);
            let v = (hash01(i + 73) - t * speed).rem_euclid(0.86);
            let twinkle = (0.35 + 0.65 * (t * 3.1 + i as f32 * 1.7).sin().powi(2))
                * (1.0 - (v / 0.86).powi(3));
            let color = SPARKLE_COLORS[i % SPARKLE_COLORS.len()];
            draw_pixel_sparkle(out, width, height, u, v, color, twinkle * 0.85);
        }

        if let Some(dt) = pulse {
            let dt = dt as f32;
            let fade = (1.0 - dt / 1.65).powi(2);
            for i in 0..14 {
                let angle = (i as f32 / 14.0) * TAU + hash01(i + 211) * 0.4;
                let speed = 0.18 + hash01(i + 251) * 0.24;
                let u = 0.50 + angle.cos() * speed * dt;
                let v = 0.44 + angle.sin() * speed * dt * 0.85 - dt * dt * 0.06;
                let color = SPARKLE_COLORS[(i + interaction.action_count as usize) % 4];
                draw_soft_sparkle(out, width, height, u, v, 0.036, color, fade * 0.95);
            }
        }
    }

    fn render_deepseek_pet(
        &mut self,
        width: usize,
        height: usize,
        time: f64,
        interaction: PreviewInteraction,
        out: &mut [[u8; 3]],
    ) {
        let variant = interaction.variant % ExperienceId::DeepseekPet.variant_count();
        let pulse = pulse_age(time, interaction).filter(|&dt| dt < 2.0);
        let t = time as f32;
        let inv_w = 1.0 / width as f32;
        let inv_h = 1.0 / height as f32;

        // 1. Stylized dark ocean-terminal diorama backdrop with water caustics.
        for y in 0..height {
            let v = (y as f32 + 0.5) * inv_h;
            let row_offset = y * width;
            for x in 0..width {
                let u = (x as f32 + 0.5) * inv_w;
                let vig = (1.0 - (u - 0.5).powi(2) * 1.15).clamp(0.45, 1.0);

                // Shimmering ocean caustics cascading down from the top surface.
                let c1 = (u * 13.5 + v * 4.2 + t * 1.15).sin();
                let c2 = (u * 21.0 - v * 5.8 - t * 0.88 + 1.2).cos();
                let caustic = ((c1 * c2) * 0.5 + 0.5).powi(4) * (1.0 - v).powf(1.45);
                let ray = (u * 8.0 - v * 2.4 + (t * 0.4).sin() * 0.8)
                    .sin()
                    .max(0.0)
                    .powi(6)
                    * (1.0 - v)
                    * 0.65;

                // Subtle deep seabed glow at bottom edge.
                let seabed = ((v - 0.80) / 0.20).clamp(0.0, 1.0).powi(2);

                let rgb = [
                    (0.022 + caustic * 0.045 + ray * 0.015 + seabed * 0.012) * vig,
                    (0.056 + (1.0 - v) * 0.046 + caustic * 0.165 + ray * 0.065 + seabed * 0.032)
                        * vig,
                    (0.118 + (1.0 - v) * 0.072 + caustic * 0.220 + ray * 0.115 + seabed * 0.064)
                        * vig,
                ];
                out[row_offset + x] = f32_to_rgb(rgb);
            }
        }

        // 2. Translucent ocean-terminal code/prompt glow lines floating subtly behind the whale.
        draw_terminal_diorama_lines(out, width, height, t);

        // 3. Compute whale trajectory, facing direction, jump arc, and sprite frame.
        let sprite_side = ((height as f32 * 0.78).min(width as f32 * 0.64)).max(6.0) as usize;
        let max_swim_x = ((width as f32 - sprite_side as f32) * 0.34).max(0.0);
        let min_center_y = sprite_side as f32 * 0.50 + 0.5;
        let max_center_y = (height as f32 - sprite_side as f32 * 0.46).max(min_center_y);

        let (mut whale_x, mut whale_y, leftward, clip_kind, clip_time) = match variant {
            0 => {
                // Ocean Patrol: rich 18s cycle through swim -> fin -> watch -> idle -> hop.
                let cycle = time.rem_euclid(18.0);
                if cycle < 6.0 {
                    let phase = (cycle / 6.0) as f32;
                    let sweep = (phase * TAU).sin();
                    let moving_left = (phase * TAU).cos() < 0.0;
                    let x = width as f32 * 0.52 + sweep * max_swim_x;
                    let y = height as f32 * 0.56 + (t * 2.2).sin() * (height as f32 * 0.05);
                    (x, y, moving_left, PetClip::Swim, cycle)
                } else if cycle < 9.5 {
                    let x = width as f32 * 0.52 + (t * 0.9).sin() * (max_swim_x * 0.18);
                    let y = height as f32 * 0.56 + (t * 1.8).sin() * (height as f32 * 0.035);
                    (x, y, false, PetClip::Fin, cycle - 6.0)
                } else if cycle < 13.0 {
                    let x = width as f32 * 0.52 + (t * 0.7).sin() * (max_swim_x * 0.15);
                    let y = height as f32 * 0.55 + (t * 1.6).sin() * (height as f32 * 0.03);
                    (x, y, false, PetClip::Watch, cycle - 9.5)
                } else if cycle < 15.6 {
                    let x = width as f32 * 0.52;
                    let y = height as f32 * 0.57 + (t * 1.5).sin() * (height as f32 * 0.025);
                    (x, y, false, PetClip::Idle, cycle - 13.0)
                } else {
                    let local = (cycle - 15.6) as f32;
                    let arc = ((local / 2.4).clamp(0.0, 1.0) * PI).sin();
                    let x = width as f32 * 0.52;
                    let y = height as f32 * 0.59 - arc * (height as f32 * 0.13);
                    (x, y, false, PetClip::Hop, cycle - 15.6)
                }
            }
            1 => {
                // Playful Hop: energetic leaps alternating with cheerful fin waves.
                let cycle = time.rem_euclid(4.8);
                if cycle < 2.6 {
                    let p = (cycle as f32 / 2.6).clamp(0.0, 1.0);
                    let arc = (p * PI).sin();
                    let x = width as f32 * 0.52 + (t * 1.2).sin() * (max_swim_x * 0.35);
                    let y = height as f32 * 0.60 - arc * (height as f32 * 0.14);
                    (x, y, (t * 1.2).cos() < 0.0, PetClip::Hop, cycle)
                } else {
                    let x = width as f32 * 0.52 + (t * 1.2).sin() * (max_swim_x * 0.35);
                    let y = height as f32 * 0.58 + (t * 2.4).sin() * (height as f32 * 0.035);
                    (x, y, false, PetClip::Fin, cycle - 2.6)
                }
            }
            _ => {
                // Cozy Nap: serene floating slumber in warm bioluminescent current.
                let x = width as f32 * 0.54 + (t * 0.45).sin() * (max_swim_x * 0.20);
                let y = height as f32 * 0.59 + (t * 1.05).sin() * (height as f32 * 0.04);
                (x, y, false, PetClip::Nap, time)
            }
        };

        let mut active_clip = clip_kind;
        let mut active_clip_time = clip_time;

        // Joyful splash leap + water ripple on Space pulse (replaces ambient hop so arcs never stack).
        if let Some(dt) = pulse {
            let dt_f = dt as f32;
            let jump_p = (dt_f / 1.15).min(1.0);
            let arc = (jump_p * PI).sin();
            whale_y = height as f32 * 0.60 - arc * (height as f32 * 0.15);
            whale_x += (dt_f * 2.5).sin() * (width as f32 * 0.02);
            active_clip = PetClip::Hop;
            active_clip_time = dt;

            // Expanding aqua splash ripple centered beneath the whale.
            let prog = (dt_f / 2.0).clamp(0.0, 1.0);
            let ring_rx = 0.06 + prog.powf(0.72) * 0.40;
            let ring_ry = ring_rx * 0.30;
            let fade = (1.0 - prog).powi(2) * (dt_f * 10.0).min(1.0);
            let rcx = whale_x * inv_w;
            let rcy = (whale_y * inv_h + 0.22).min(0.92);
            for y in 0..height {
                let v = (y as f32 + 0.5) * inv_h;
                let dy = (v - rcy) / ring_ry.max(0.01);
                if dy.abs() > 2.2 {
                    continue;
                }
                let row_offset = y * width;
                for x in 0..width {
                    let u = (x as f32 + 0.5) * inv_w;
                    let dx = (u - rcx) / ring_rx.max(0.01);
                    let dist = dx.hypot(dy);
                    let band = (-((dist - 1.0) / 0.22).powi(2)).exp() * fade;
                    if band > 0.01 {
                        let px = &mut out[row_offset + x];
                        px[0] = add_clamp(px[0], 95.0 * band);
                        px[1] = add_clamp(px[1], 225.0 * band);
                        px[2] = add_clamp(px[2], 255.0 * band);
                    }
                }
            }
        }
        whale_y = whale_y.clamp(min_center_y, max_center_y);

        // Soft underwater aura right behind the whale so its silhouette separates crisply.
        draw_soft_sparkle(
            out,
            width,
            height,
            whale_x * inv_w,
            whale_y * inv_h,
            0.28,
            [52, 165, 245],
            0.24,
        );

        // 4. Composite the whale sprite with area-averaged alpha blending!
        draw_deepseek_whale(
            out,
            width,
            height,
            whale_x.round() as isize,
            whale_y.round() as isize,
            sprite_side,
            leftward,
            active_clip,
            active_clip_time,
        );

        // 5. Rising translucent cyan bubbles + celebratory bubble burst on pulse.
        let bubble_count = if variant == 1 { 18 } else { 13 };
        for i in 0..bubble_count {
            let speed = 0.09 + hash01(i + 401) * 0.11;
            let u = (hash01(i + 433) + 0.03 * (t * 1.5 + i as f32 * 1.3).sin()).rem_euclid(1.0);
            let v = (hash01(i + 467) - t * speed).rem_euclid(1.0);
            let radius = 0.016 + hash01(i + 503) * 0.014;
            let alpha =
                (0.35 + 0.35 * (t * 2.2 + i as f32).sin().powi(2)) * (1.0 - (1.0 - v).powi(4));
            draw_soft_sparkle(out, width, height, u, v, radius, [130, 238, 255], alpha);
        }

        if let Some(dt) = pulse {
            let dt = dt as f32;
            let fade = (1.0 - dt / 2.0).powi(2);
            let origin_u = whale_x * inv_w;
            let origin_v = (whale_y * inv_h - 0.06).clamp(0.05, 0.92);
            for i in 0..12 {
                let spread = (hash01(i + 601) - 0.5) * 0.32;
                let rise = (0.18 + hash01(i + 641) * 0.28) * dt;
                let u = origin_u + spread * (0.4 + dt * 0.9) + 0.015 * (dt * 6.0 + i as f32).sin();
                let v = origin_v - rise;
                draw_soft_sparkle(
                    out,
                    width,
                    height,
                    u,
                    v,
                    0.024 + hash01(i + 677) * 0.012,
                    [175, 248, 255],
                    fade * 0.92,
                );
            }
        }
    }
}

#[derive(Clone, Copy)]
enum PetClip {
    Idle,
    Fin,
    Swim,
    Hop,
    Watch,
    Nap,
}

fn pulse_age(time: f64, interaction: PreviewInteraction) -> Option<f64> {
    if interaction.action_count == 0 && interaction.action_pulse <= 0.0 {
        return None;
    }
    let dt = time - interaction.action_pulse;
    if dt.is_finite() && dt >= 0.0 {
        Some(dt)
    } else {
        None
    }
}

fn grade_jellyfish([r, g, b]: [f32; 3], variant: usize) -> [f32; 3] {
    match variant {
        0 => [r * 1.04, g * 1.03, b * 1.05],
        1 => [
            (r * 1.05 + b * 0.36 - g * 0.12).max(0.0),
            (r * 0.62 + g * 0.36 + b * 0.05).max(0.0),
            (g * 0.62 + b * 0.78 - r * 0.28).max(0.0),
        ],
        _ => [
            (r * 0.50 + g * 0.12).max(0.0),
            (r * 0.74 + g * 0.88 + b * 0.12).max(0.0),
            (r * 0.18 + g * 0.34 + b * 0.76).max(0.0),
        ],
    }
}

fn choose_plot_resolution(width: usize, height: usize) -> (u32, u32) {
    // 2x supersampling whenever we remain below Plot's 160x80 tick-label threshold.
    let target_w = (width.saturating_mul(2)).max(1);
    let target_h = (height.saturating_mul(2)).max(1);
    if target_w < 160 && target_h < 80 {
        return (target_w as u32, target_h as u32);
    }
    // Preserve exact stage aspect ratio while staying under (158, 78) for crisp framing and <5ms speed.
    let scale_w = 158.0 / width.max(1) as f64;
    let scale_h = 78.0 / height.max(1) as f64;
    let scale = scale_w.min(scale_h).min(2.0);
    let rw = ((width as f64 * scale).round() as u32).clamp(1, 158);
    let rh = ((height as f64 * scale).round() as u32).clamp(1, 78);
    (rw, rh)
}

fn apply_subtle_bloom(pixels: &mut [[u8; 3]], width: usize, height: usize, strength: f32) {
    let snapshot = pixels.to_vec();
    for y in 1..height - 1 {
        let row = y * width;
        for x in 1..width - 1 {
            let mut acc = [0.0_f32; 3];
            for &(ox, oy, w) in &[
                (-1_isize, 0_isize, 0.20_f32),
                (1, 0, 0.20),
                (0, -1, 0.20),
                (0, 1, 0.20),
                (-1, -1, 0.05),
                (1, -1, 0.05),
                (-1, 1, 0.05),
                (1, 1, 0.05),
            ] {
                let nb = snapshot[(y as isize + oy) as usize * width + (x as isize + ox) as usize];
                // Only bloom genuinely bright curve pixels above the dark studio floor.
                let luma =
                    f32::from(nb[0]) * 0.299 + f32::from(nb[1]) * 0.587 + f32::from(nb[2]) * 0.114;
                if luma > 105.0 {
                    let excess = (luma - 105.0) / 150.0;
                    acc[0] += f32::from(nb[0]) * w * excess;
                    acc[1] += f32::from(nb[1]) * w * excess;
                    acc[2] += f32::from(nb[2]) * w * excess;
                }
            }
            if acc[0] + acc[1] + acc[2] > 1.0 {
                let px = &mut pixels[row + x];
                px[0] = add_clamp(px[0], acc[0] * strength);
                px[1] = add_clamp(px[1], acc[1] * strength);
                px[2] = add_clamp(px[2], acc[2] * strength);
            }
        }
    }
}

fn spotlight_action_at(time: f64) -> babies_scene::Action {
    let total: f64 = BABIES_SPOTLIGHT_ACTIONS.iter().map(|a| a.duration()).sum();
    let mut rem = time.rem_euclid(total);
    for action in BABIES_SPOTLIGHT_ACTIONS {
        if rem + 1e-9 < action.duration() {
            return action;
        }
        rem -= action.duration();
    }
    babies_scene::Action::Wave
}

fn trio_action_at(time: f64, member_index: usize) -> babies_scene::Action {
    const CHOREO: [[babies_scene::Action; 4]; 3] = [
        [
            babies_scene::Action::Wave,
            babies_scene::Action::Hop,
            babies_scene::Action::Walk,
            babies_scene::Action::Think,
        ],
        [
            babies_scene::Action::Hop,
            babies_scene::Action::Wave,
            babies_scene::Action::Think,
            babies_scene::Action::Walk,
        ],
        [
            babies_scene::Action::Walk,
            babies_scene::Action::Wave,
            babies_scene::Action::Hop,
            babies_scene::Action::Idle,
        ],
    ];
    let seq = &CHOREO[member_index % CHOREO.len()];
    let total: f64 = seq.iter().map(|a| a.duration()).sum();
    let mut rem = time.rem_euclid(total);
    for &action in seq {
        if rem + 1e-9 < action.duration() {
            return action;
        }
        rem -= action.duration();
    }
    seq[0]
}

#[allow(clippy::too_many_arguments)]
fn blit_ai_baby_with_reflection(
    out: &mut [[u8; 3]],
    width: usize,
    height: usize,
    character: babies_scene::Character,
    action: babies_scene::Action,
    action_time: f64,
    center_x: isize,
    foot_y: isize,
    floor_y: isize,
    sprite_side: usize,
) {
    // Request the matching atlas from ai_babies::scene. The rectangular 2:1 atlas represents a
    // physically square character (2x horizontal detail), so mapping (src_w, src_h) onto a
    // square HalfBlock box (sprite_side x sprite_side) yields crisp 1:1 physical proportions.
    let (atlas, src_h) = character.atlas(sprite_side);
    let src_w = src_h * 2;
    let frame_idx = babies_scene::current_frame(action_time, Some(action));
    let frame_bytes = src_w * src_h * 4;
    let frame = &atlas[frame_idx * frame_bytes..(frame_idx + 1) * frame_bytes];

    let dest_x0 = center_x - (sprite_side as isize) / 2;
    let dest_y0 = foot_y - sprite_side as isize;

    // 1. Subtle vertically flipped reflection on the glossy studio floor below `floor_y`.
    let refl_h = (sprite_side / 3).max(2);
    for ry in 0..refl_h {
        let py = floor_y + ry as isize;
        if py < 0 || py >= height as isize {
            continue;
        }
        let depth = (ry as f32 + 0.5) / refl_h as f32;
        let opacity = 0.26 * (1.0 - depth).powf(1.55);
        // Sample upwards from the character's lower 45% (feet & legs).
        let src_v = (0.93 - depth * 0.42).clamp(0.0, 0.999);
        let sy = (src_v * src_h as f32) as usize;
        let row_out = py as usize * width;
        for dx in 0..sprite_side {
            let px = dest_x0 + dx as isize;
            if px < 0 || px >= width as isize {
                continue;
            }
            let (r, g, b, a) = sample_rgba_box(
                frame,
                src_w,
                src_h,
                dx as f32 / sprite_side as f32,
                (dx + 1) as f32 / sprite_side as f32,
                sy as f32 / src_h as f32,
                (sy + 1).min(src_h) as f32 / src_h as f32,
                false,
            );
            let alpha = a * opacity;
            if alpha > 0.01 {
                blend_over(&mut out[row_out + px as usize], [r, g, b], alpha);
            }
        }
    }

    // 2. Main character sprite with area-averaged alpha compositing.
    blit_rgba_area_averaged(
        out,
        width,
        height,
        frame,
        src_w,
        src_h,
        dest_x0,
        dest_y0,
        sprite_side,
        sprite_side,
        false,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_deepseek_whale(
    out: &mut [[u8; 3]],
    width: usize,
    height: usize,
    center_x: isize,
    center_y: isize,
    sprite_side: usize,
    leftward: bool,
    clip: PetClip,
    clip_time: f64,
) {
    #[cfg(any(unix, windows))]
    {
        let clip_ref = match clip {
            PetClip::Idle => &pet_anim::PREVIEW_IDLE,
            PetClip::Fin => &pet_anim::PREVIEW_FIN,
            PetClip::Swim => &pet_anim::SWIM,
            PetClip::Hop => &pet_anim::PREVIEW_HOP,
            PetClip::Watch => &pet_anim::PREVIEW_WATCH,
            PetClip::Nap => &pet_anim::PREVIEW_NAP,
        };
        // clips.yaml durations are in 100ms ticks (10 ticks per second).
        let tick =
            ((clip_time.max(0.0) * 10.0).floor() as u64 % u64::from(clip_ref.duration)) as u16;
        let frame_id = clip_ref.frame(tick);
        let rgba = pet_anim::pixels(frame_id);
        let dest_x0 = center_x - (sprite_side as isize) / 2;
        let dest_y0 = center_y - (sprite_side as isize) / 2;
        blit_rgba_area_averaged(
            out,
            width,
            height,
            rgba,
            pet_anim::SIDE,
            pet_anim::SIDE,
            dest_x0,
            dest_y0,
            sprite_side,
            sprite_side,
            leftward,
        );
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = clip;
        draw_procedural_whale_fallback(
            out,
            width,
            height,
            center_x,
            center_y,
            sprite_side,
            leftward,
            clip_time as f32,
        );
    }
}

#[cfg(not(any(unix, windows)))]
#[allow(clippy::too_many_arguments)]
fn draw_procedural_whale_fallback(
    out: &mut [[u8; 3]],
    width: usize,
    height: usize,
    center_x: isize,
    center_y: isize,
    sprite_side: usize,
    leftward: bool,
    t: f32,
) {
    let half = (sprite_side as f32 * 0.44).max(2.0);
    let dir = if leftward { -1.0_f32 } else { 1.0_f32 };
    let tail_wag = (t * 6.0).sin() * 0.18;
    let blink = (t * 0.7).rem_euclid(4.0) > 3.85;

    let y_start = (center_y - sprite_side as isize / 2).max(0) as usize;
    let y_end = ((center_y + sprite_side as isize / 2 + 1) as usize).min(height);
    let x_start = (center_x - sprite_side as isize / 2).max(0) as usize;
    let x_end = ((center_x + sprite_side as isize / 2 + 1) as usize).min(width);

    for py in y_start..y_end {
        let ny = (py as f32 - center_y as f32) / half;
        let row = py * width;
        for px in x_start..x_end {
            let nx = ((px as f32 - center_x as f32) / half) * dir;

            // Rounded whale body + curving tail fluke.
            let body_d = (nx / 0.85).powi(2) + ((ny + 0.04) / 0.68).powi(2);
            let tail_d = ((nx + 0.86) / 0.34).powi(2) + ((ny - 0.16 - tail_wag) / 0.24).powi(2);
            let fluke_d = ((nx + 0.96) / 0.24).powi(2) + ((ny + 0.20 - tail_wag) / 0.26).powi(2);
            let fin_d =
                ((nx - 0.08) / 0.26).powi(2) + ((ny - 0.48 - tail_wag * 0.4) / 0.18).powi(2);

            let inside = body_d < 1.0 || tail_d < 1.0 || fluke_d < 1.0 || fin_d < 1.0;
            if !inside {
                continue;
            }

            let mut color = if ny > 0.22 && body_d < 0.92 {
                [195.0, 232.0, 255.0] // soft pale belly
            } else {
                let shade = (1.0 - (ny + 0.35).clamp(0.0, 1.0) * 0.28).clamp(0.65, 1.1);
                [66.0 * shade, 146.0 * shade, 248.0 * shade]
            };

            // Specular crown sheen.
            let sheen = (-((nx - 0.18) / 0.32).powi(2) - ((ny + 0.34) / 0.16).powi(2)).exp();
            color[0] += 90.0 * sheen;
            color[1] += 100.0 * sheen;
            color[2] += 80.0 * sheen;

            // Cheerful dark eye with white glint.
            let eye_d = ((nx - 0.34) / 0.11).powi(2) + ((ny + 0.06) / 0.11).powi(2);
            if eye_d < 1.0 && (!blink || (ny + 0.06).abs() < 0.03) {
                color = [18.0, 28.0, 52.0];
                if (nx - 0.31).hypot(ny + 0.09) < 0.045 {
                    color = [245.0, 252.0, 255.0];
                }
            }

            blend_over(
                &mut out[row + px],
                [
                    color[0].clamp(0.0, 255.0),
                    color[1].clamp(0.0, 255.0),
                    color[2].clamp(0.0, 255.0),
                ],
                0.96,
            );
        }
    }
}

/// Area-averaged alpha-blended downsampling of an RGBA sprite onto an RGB framebuffer.
#[allow(clippy::too_many_arguments)]
fn blit_rgba_area_averaged(
    out: &mut [[u8; 3]],
    width: usize,
    height: usize,
    rgba: &[u8],
    src_w: usize,
    src_h: usize,
    dest_x0: isize,
    dest_y0: isize,
    dest_w: usize,
    dest_h: usize,
    flip_x: bool,
) {
    if dest_w == 0 || dest_h == 0 || src_w == 0 || src_h == 0 {
        return;
    }
    let inv_dw = 1.0 / dest_w as f32;
    let inv_dh = 1.0 / dest_h as f32;

    let y_min = (-dest_y0).max(0) as usize;
    let y_max = (height as isize - dest_y0).clamp(0, dest_h as isize) as usize;
    let x_min = (-dest_x0).max(0) as usize;
    let x_max = (width as isize - dest_x0).clamp(0, dest_w as isize) as usize;

    for dy in y_min..y_max {
        let py = (dest_y0 + dy as isize) as usize;
        let v0 = dy as f32 * inv_dh;
        let v1 = (dy + 1) as f32 * inv_dh;
        let row = py * width;
        for dx in x_min..x_max {
            let px = (dest_x0 + dx as isize) as usize;
            let u0 = dx as f32 * inv_dw;
            let u1 = (dx + 1) as f32 * inv_dw;
            let (r, g, b, alpha) = sample_rgba_box(rgba, src_w, src_h, u0, u1, v0, v1, flip_x);
            if alpha > 0.015 {
                blend_over(&mut out[row + px], [r, g, b], alpha);
            }
        }
    }
}

/// Stratified 2D area-average over normalized UV box `[u0, u1) x [v0, v1)` with unpremultiplied RGB.
///
/// Uses up to a 4x4 stratified footprint per output pixel so 320x320 sprites stay razor-sharp
/// and smooth without nearest-neighbor aliasing while running in <1ms per frame.
#[allow(clippy::too_many_arguments)]
fn sample_rgba_box(
    rgba: &[u8],
    src_w: usize,
    src_h: usize,
    u0: f32,
    u1: f32,
    v0: f32,
    v1: f32,
    flip_x: bool,
) -> (f32, f32, f32, f32) {
    let (su0, su1) = if flip_x {
        (1.0 - u1, 1.0 - u0)
    } else {
        (u0, u1)
    };
    let fx0 = (su0 * src_w as f32).clamp(0.0, src_w as f32);
    let fx1 = (su1 * src_w as f32).clamp(0.0, src_w as f32);
    let fy0 = (v0 * src_h as f32).clamp(0.0, src_h as f32);
    let fy1 = (v1 * src_h as f32).clamp(0.0, src_h as f32);

    let nx = ((fx1 - fx0).ceil() as usize).clamp(1, 4);
    let ny = ((fy1 - fy0).ceil() as usize).clamp(1, 4);

    let mut sum_r = 0.0_f32;
    let mut sum_g = 0.0_f32;
    let mut sum_b = 0.0_f32;
    let mut sum_a = 0.0_f32;
    let total = (nx * ny) as f32;

    for iy in 0..ny {
        let fy = fy0 + (iy as f32 + 0.5) / ny as f32 * (fy1 - fy0);
        let sy = (fy as usize).min(src_h - 1);
        let row = sy * src_w;
        for ix in 0..nx {
            let fx = fx0 + (ix as f32 + 0.5) / nx as f32 * (fx1 - fx0);
            let sx = (fx as usize).min(src_w - 1);
            let off = (row + sx) * 4;
            let a = f32::from(rgba[off + 3]) / 255.0;
            if a > 0.0 {
                sum_r += f32::from(rgba[off]) * a;
                sum_g += f32::from(rgba[off + 1]) * a;
                sum_b += f32::from(rgba[off + 2]) * a;
                sum_a += a;
            }
        }
    }

    if sum_a <= 1e-5 {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let inv_a = 1.0 / sum_a;
    (
        sum_r * inv_a,
        sum_g * inv_a,
        sum_b * inv_a,
        (sum_a / total).clamp(0.0, 1.0),
    )
}

fn draw_terminal_diorama_lines(out: &mut [[u8; 3]], width: usize, height: usize, t: f32) {
    if width < 14 || height < 10 {
        return;
    }
    // Subtle dark glass terminal chrome header dots in top-left corner.
    let dot_y = (height / 9).clamp(1, 3);
    let dot_x0 = (width / 12).clamp(2, 5);
    const DOTS: [[u8; 3]; 3] = [[210, 95, 110], [225, 185, 95], [85, 205, 145]];
    for (i, &color) in DOTS.iter().enumerate() {
        let px = dot_x0 + i * 2;
        if px < width && dot_y < height {
            blend_over(
                &mut out[dot_y * width + px],
                [
                    f32::from(color[0]),
                    f32::from(color[1]),
                    f32::from(color[2]),
                ],
                0.42,
            );
        }
    }

    // Glowing code/prompt lines softly pulsing like a background terminal session.
    type TerminalSegment = (f32, f32, [u8; 3]);
    type TerminalLine = (f32, &'static [TerminalSegment]);
    let lines: [TerminalLine; 5] = [
        (
            0.24,
            &[
                (0.07, 0.11, [90, 235, 210]),
                (0.14, 0.30, [110, 175, 240]),
                (0.33, 0.45, [155, 130, 235]),
            ],
        ),
        (
            0.36,
            &[(0.10, 0.22, [95, 165, 225]), (0.25, 0.38, [88, 220, 195])],
        ),
        (
            0.48,
            &[
                (0.07, 0.11, [90, 235, 210]),
                (0.14, 0.27, [230, 180, 115]),
                (0.30, 0.41, [115, 175, 235]),
            ],
        ),
        (
            0.60,
            &[(0.10, 0.25, [145, 140, 235]), (0.28, 0.36, [95, 215, 225])],
        ),
        (
            0.74,
            &[(0.07, 0.11, [98, 245, 210]), (0.14, 0.24, [120, 190, 245])],
        ),
    ];

    for (line_idx, &(y_frac, segs)) in lines.iter().enumerate() {
        let py = ((height as f32 * y_frac).round() as usize).min(height - 1);
        let shimmer = 0.22 + 0.10 * (t * 1.4 - line_idx as f32 * 0.9).sin();
        let row = py * width;
        for &(x0f, x1f, color) in segs {
            let x0 = ((width as f32 * x0f) as usize).min(width);
            let x1 = ((width as f32 * x1f) as usize).min(width);
            for x in x0..x1 {
                blend_over(
                    &mut out[row + x],
                    [
                        f32::from(color[0]),
                        f32::from(color[1]),
                        f32::from(color[2]),
                    ],
                    shimmer,
                );
            }
        }
        // Blinking live prompt cursor on the bottom prompt line.
        if line_idx == 4 && (t * 2.2).sin() > -0.15 {
            let cx = ((width as f32 * 0.26) as usize).min(width - 1);
            blend_over(&mut out[row + cx], [125.0, 250.0, 235.0], 0.62);
        }
    }
}

fn draw_pixel_sparkle(
    out: &mut [[u8; 3]],
    width: usize,
    height: usize,
    u: f32,
    v: f32,
    color: [u8; 3],
    alpha: f32,
) {
    if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) || alpha <= 0.01 {
        return;
    }
    let px = (u * width as f32) as usize;
    let py = (v * height as f32) as usize;
    if px < width && py < height {
        let dst = &mut out[py * width + px];
        dst[0] = add_clamp(dst[0], f32::from(color[0]) * alpha);
        dst[1] = add_clamp(dst[1], f32::from(color[1]) * alpha);
        dst[2] = add_clamp(dst[2], f32::from(color[2]) * alpha);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_soft_sparkle(
    out: &mut [[u8; 3]],
    width: usize,
    height: usize,
    u: f32,
    v: f32,
    radius: f32,
    color: [u8; 3],
    strength: f32,
) {
    if strength <= 0.01 || radius <= 0.0 {
        return;
    }
    let cx = u * width as f32;
    let cy = v * height as f32;
    let r_px = (radius * height.min(width) as f32).max(0.75);
    let pad = (r_px * 2.2).ceil() as isize;
    let x0 = (cx.floor() as isize - pad).max(0) as usize;
    let x1 = ((cx.ceil() as isize + pad + 1) as usize).min(width);
    let y0 = (cy.floor() as isize - pad).max(0) as usize;
    let y1 = ((cy.ceil() as isize + pad + 1) as usize).min(height);
    let inv_r2 = 1.0 / (r_px * r_px);

    for py in y0..y1 {
        let dy = py as f32 + 0.5 - cy;
        let row = py * width;
        for px in x0..x1 {
            let dx = px as f32 + 0.5 - cx;
            let glow = (-(dx * dx + dy * dy) * inv_r2).exp() * strength;
            if glow > 0.01 {
                let dst = &mut out[row + px];
                dst[0] = add_clamp(dst[0], f32::from(color[0]) * glow);
                dst[1] = add_clamp(dst[1], f32::from(color[1]) * glow);
                dst[2] = add_clamp(dst[2], f32::from(color[2]) * glow);
            }
        }
    }
}

fn blend_over(dst: &mut [u8; 3], src: [f32; 3], alpha: f32) {
    let a = alpha.clamp(0.0, 1.0);
    let inv = 1.0 - a;
    for c in 0..3 {
        dst[c] = (f32::from(dst[c]) * inv + src[c] * a)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
}

fn add_clamp(base: u8, delta: f32) -> u8 {
    (f32::from(base) + delta).round().clamp(0.0, 255.0) as u8
}

fn hash01(n: usize) -> f32 {
    let mut x = (n as u32)
        .wrapping_mul(747_796_405)
        .wrapping_add(2_891_336_453);
    x = ((x >> ((x >> 28) + 4)) ^ x).wrapping_mul(277_803_737);
    ((x >> 22) ^ x) as f32 / u32::MAX as f32
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
    fn experience_metadata_roundtrips_and_wraps_safely() {
        for (i, exp) in ExperienceId::ALL.into_iter().enumerate() {
            assert_eq!(exp.index(), i);
            assert_eq!(ExperienceId::from_index(i), exp);
            assert_eq!(
                ExperienceId::from_index(i + ExperienceId::ALL.len() * 3),
                exp
            );
            assert!(exp.variant_count() >= 3);
            assert!(!SceneEngine::interaction_hint(exp).is_empty());
            assert_ne!(exp.accent_rgb(), exp.secondary_rgb());
            for v in 0..exp.variant_count() + 2 {
                assert!(!SceneEngine::variant_label(exp, v).is_empty());
            }
        }
    }

    #[test]
    fn all_scenes_render_without_panicking_across_edge_and_standard_sizes() {
        let mut engine = SceneEngine::new();
        let mut out = Vec::new();
        for exp in ExperienceId::ALL {
            for (w, h) in [(0, 0), (0, 8), (8, 0), (1, 1), (3, 2), (24, 14), (72, 36)] {
                engine.render_into(exp, w, h, 1.25, PreviewInteraction::default(), &mut out);
                assert_eq!(out.len(), w * h, "{exp:?} {w}x{h}");
            }
            // Non-finite timestamps must never panic.
            engine.render_into(
                exp,
                20,
                12,
                f64::NAN,
                PreviewInteraction::default(),
                &mut out,
            );
            assert_eq!(out.len(), 240);
            engine.render_into(
                exp,
                20,
                12,
                f64::INFINITY,
                PreviewInteraction::default(),
                &mut out,
            );
            assert_eq!(out.len(), 240);
        }
    }

    #[test]
    fn every_scene_is_alive_across_time_variants_and_action_pulses() {
        let mut engine = SceneEngine::new();
        let mut a = Vec::new();
        let mut b = Vec::new();
        let mut pulsed = Vec::new();
        let (w, h) = (64, 32);

        for exp in ExperienceId::ALL {
            let base = PreviewInteraction::default();
            engine.render_into(exp, w, h, 0.5, base, &mut a);
            engine.render_into(exp, w, h, 2.1, base, &mut b);
            assert_ne!(a, b, "{exp:?} must animate over time");
            // Ensure rich visual diversity (not a flat solid color).
            let unique: std::collections::HashSet<_> = a.iter().copied().collect();
            assert!(
                unique.len() > 25,
                "{exp:?} has too few colors: {}",
                unique.len()
            );

            // Switching variants visibly changes the stage.
            let alt = PreviewInteraction { variant: 1, ..base };
            engine.render_into(exp, w, h, 0.5, alt, &mut b);
            assert_ne!(a, b, "{exp:?} variant 1 must differ from variant 0");

            // Action pulse visibly reacts and surfaces a caption.
            let interaction = PreviewInteraction {
                variant: 0,
                action_pulse: 0.5,
                action_count: 1,
            };
            engine.render_into(exp, w, h, 0.85, base, &mut a);
            engine.render_into(exp, w, h, 0.85, interaction, &mut pulsed);
            assert_ne!(a, pulsed, "{exp:?} must respond visibly to action_pulse");
            assert!(
                SceneEngine::overlay_caption(exp, 0.85, interaction).is_some(),
                "{exp:?} should surface a caption during active pulse"
            );
        }
    }

    #[test]
    fn function_plotter_renders_all_four_presets_including_2d_polar_animation() {
        let mut engine = SceneEngine::new();
        let mut first = Vec::new();
        let mut second = Vec::new();
        for variant in 0..ExperienceId::FunctionPlotter.variant_count() {
            let interaction = PreviewInteraction {
                variant,
                ..PreviewInteraction::default()
            };
            engine.render_into(
                ExperienceId::FunctionPlotter,
                68,
                34,
                0.4,
                interaction,
                &mut first,
            );
            engine.render_into(
                ExperienceId::FunctionPlotter,
                68,
                34,
                1.6,
                interaction,
                &mut second,
            );
            assert_eq!(first.len(), 68 * 34);
            assert_ne!(
                first, second,
                "FunctionPlotter variant {variant} must animate over time"
            );
        }
    }

    #[test]
    fn default_interaction_does_not_trigger_phantom_startup_pulse() {
        assert!(pulse_age(0.25, PreviewInteraction::default()).is_none());
        assert!(pulse_age(0.0, PreviewInteraction::default()).is_none());
        assert_eq!(
            pulse_age(
                0.35,
                PreviewInteraction {
                    variant: 0,
                    action_pulse: 0.0,
                    action_count: 1
                }
            ),
            Some(0.35)
        );
    }

    #[test]
    fn render_is_deterministic_for_identical_inputs() {
        let mut engine = SceneEngine::new();
        let mut first = Vec::new();
        let mut second = Vec::new();
        for exp in ExperienceId::ALL {
            let interaction = PreviewInteraction {
                variant: 1,
                action_pulse: 1.0,
                action_count: 2,
            };
            engine.render_into(exp, 56, 28, 1.4, interaction, &mut first);
            engine.render_into(exp, 56, 28, 1.4, interaction, &mut second);
            assert_eq!(first, second, "{exp:?} render must be deterministic");
        }
    }
}
