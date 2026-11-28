//! Real-time preview graphics engine for all eight `rsille-playground` showcase worlds.

use std::f32::consts::{PI, TAU};

use crate::{
    clockwork::{
        machine::Machine,
        raster::{Camera as ClockCamera, Raster as ClockRaster},
    },
    doctor::{self, Pattern as DoctorPattern},
    function_plotter::{plot::Plot, X_RANGE},
    jellyfish::sea::Sea,
    voxel::{scene as voxel_scene, world::World as VoxelWorld},
};

#[cfg(any(unix, windows))]
use crate::deepseek_pet::animation as pet_anim;

/// Total number of showcase experiences in the interactive launcher.
pub const EXPERIENCE_COUNT: usize = ExperienceId::ALL.len();

/// Identifies one of the eight showcase experiences in the interactive launcher.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExperienceId {
    Voxel,
    Clockwork,
    Desk,
    ShellHud,
    DeepseekPet,
    FunctionPlotter,
    Jellyfish,
    Doctor,
}

impl ExperienceId {
    /// Ordered list of all showcase experiences.
    pub const ALL: [ExperienceId; 8] = [
        Self::Voxel,
        Self::Clockwork,
        Self::Desk,
        Self::ShellHud,
        Self::DeepseekPet,
        Self::FunctionPlotter,
        Self::Jellyfish,
        Self::Doctor,
    ];

    /// Zero-based index into [`ExperienceId::ALL`].
    pub fn index(self) -> usize {
        match self {
            Self::Voxel => 0,
            Self::Clockwork => 1,
            Self::Desk => 2,
            Self::ShellHud => 3,
            Self::DeepseekPet => 4,
            Self::FunctionPlotter => 5,
            Self::Jellyfish => 6,
            Self::Doctor => 7,
        }
    }

    /// Maps an arbitrary index into an experience by wrapping smoothly around [`ExperienceId::ALL`].
    pub fn from_index(index: usize) -> Self {
        Self::ALL[index % Self::ALL.len()]
    }

    /// Primary luminous accent color for UI chrome, borders, and transition gleams.
    pub fn accent_rgb(self) -> [u8; 3] {
        match self {
            Self::Voxel => [116, 226, 134],
            Self::Clockwork => [242, 186, 98],
            Self::Desk => [176, 146, 255],
            Self::ShellHud => [96, 224, 198],
            Self::DeepseekPet => [86, 178, 255],
            Self::FunctionPlotter => [130, 198, 255],
            Self::Jellyfish => [72, 224, 255],
            Self::Doctor => [255, 148, 196],
        }
    }

    /// Secondary complementary highlight color for chromatic gradients and aurora strips.
    pub fn secondary_rgb(self) -> [u8; 3] {
        match self {
            Self::Voxel => [94, 206, 248],
            Self::Clockwork => [236, 118, 86],
            Self::Desk => [108, 232, 214],
            Self::ShellHud => [128, 172, 255],
            Self::DeepseekPet => [108, 242, 232],
            Self::FunctionPlotter => [255, 184, 96],
            Self::Jellyfish => [255, 108, 152],
            Self::Doctor => [108, 232, 255],
        }
    }

    /// Number of interactive preview variants supported by this experience.
    pub fn variant_count(self) -> usize {
        match self {
            Self::Voxel => 3,
            Self::Clockwork => 3,
            Self::Desk => 3,
            Self::ShellHud => 3,
            Self::DeepseekPet => 3,
            Self::FunctionPlotter => 4,
            Self::Jellyfish => 3,
            Self::Doctor => 4,
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

/// Stateful real-time graphics engine that renders all eight launcher preview stages into RGB buffers.
pub struct SceneEngine {
    sea: Sea,
    plots: Vec<PlotPreset>,
    voxel_world: VoxelWorld,
    clockwork_raster: ClockRaster,
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
            voxel_world: VoxelWorld::default(),
            clockwork_raster: ClockRaster::default(),
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
            ExperienceId::Voxel => self.render_voxel(width, height, time, interaction, out),
            ExperienceId::Clockwork => self.render_clockwork(width, height, time, interaction, out),
            ExperienceId::Desk => self.render_desk(width, height, time, interaction, out),
            ExperienceId::ShellHud => self.render_hud(width, height, time, interaction, out),
            ExperienceId::DeepseekPet => {
                self.render_deepseek_pet(width, height, time, interaction, out)
            }
            ExperienceId::FunctionPlotter => {
                self.render_function_plotter(width, height, time, interaction, out)
            }
            ExperienceId::Jellyfish => self.render_jellyfish(width, height, time, interaction, out),
            ExperienceId::Doctor => self.render_doctor(width, height, time, interaction, out),
        }
    }

    /// Short display label describing the currently active preview variant.
    pub fn variant_label(exp: ExperienceId, variant: usize) -> &'static str {
        let idx = variant % exp.variant_count();
        match exp {
            ExperienceId::Voxel => match idx {
                0 => "Daylight Meadow",
                1 => "Starry Night",
                _ => "High Orbit",
            },
            ExperienceId::Clockwork => match idx {
                0 => "Strider Walker",
                1 => "Ornithopter",
                _ => "Clockwork Iris",
            },
            ExperienceId::Desk => match idx {
                0 => "Spatial Desk · Moon",
                1 => "Motion Lab · Dusk",
                _ => "Overview · Paper",
            },
            ExperienceId::ShellHud => match idx {
                0 => "Files & cd here",
                1 => "Task Checklist",
                _ => "Quick Macros",
            },
            ExperienceId::DeepseekPet => match idx {
                0 => "Ocean Patrol",
                1 => "Playful Hop",
                _ => "Cozy Nap",
            },
            ExperienceId::FunctionPlotter => match idx {
                0 => "sin(x)·cos(y) 3D",
                1 => "Saddle (x²-y²)/3",
                2 => "Trefoil Knot 3D",
                _ => "Polar Rose + Cardioid",
            },
            ExperienceId::Jellyfish => match idx {
                0 => "Abyssal Cyan",
                1 => "Biolum Violet",
                _ => "Emerald Deep",
            },
            ExperienceId::Doctor => DoctorPattern::ALL[idx].name(),
        }
    }

    /// Concise contextual hint telling the user what `Space` does on the live stage.
    pub fn interaction_hint(exp: ExperienceId) -> &'static str {
        match exp {
            ExperienceId::Voxel => "Space: flyover jump & sky",
            ExperienceId::Clockwork => "Space: magnetic pulse & form",
            ExperienceId::Desk => "Space: light sweep & theme",
            ExperienceId::ShellHud => "Space: run command & tab",
            ExperienceId::DeepseekPet => "Space: splash hop & pet whale",
            ExperienceId::FunctionPlotter => "Space: spin boost & next plot",
            ExperienceId::Jellyfish => "Space: biolum pulse & palette",
            ExperienceId::Doctor => "Space: scan pulse & pattern",
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
                    ExperienceId::Voxel => match variant {
                        1 => "✦ Starlight mode · mine & build",
                        2 => "✦ High-altitude flyover",
                        _ => "✦ Creative flight · 8 block types",
                    },
                    ExperienceId::Clockwork => match variant {
                        1 => "✦ Magnetic pulse → Ornithopter",
                        2 => "✦ Magnetic pulse → Clockwork Iris",
                        _ => "✦ Magnetic pulse → 132-part Strider",
                    },
                    ExperienceId::Desk => match variant {
                        1 => "✦ Highlight sweep · Dusk palette",
                        2 => "✦ Paper studio · floating windows",
                        _ => "✦ Spatial desk · drag & dock",
                    },
                    ExperienceId::ShellHud => match variant {
                        1 => "✦ Checked off task · 2/3 done",
                        2 => "✦ Injected » yes, continue",
                        _ => "✦ cd ~/projects/rsille · 0.1s ✔",
                    },
                    ExperienceId::DeepseekPet => match interaction.action_count % 3 {
                        0 => "~ Splash! Happy whale!",
                        1 => "~ *bubble bubble* ♪",
                        _ => "~ Ready to swim in your shell!",
                    },
                    ExperienceId::FunctionPlotter => {
                        if variant == 3 {
                            "✦ Polar harmonic sweep"
                        } else {
                            "✦ Orbit boost · 3D shaded"
                        }
                    }
                    ExperienceId::Jellyfish => match interaction.action_count % 3 {
                        0 => "✦ Bioluminescent surge",
                        1 => "✦ Deep current shift",
                        _ => "✦ Abyssal ripple",
                    },
                    ExperienceId::Doctor => match variant {
                        1 => "✦ Precision grid calibration",
                        2 => "✦ Neon torus · 4 pipelines",
                        3 => "✦ TrueColor chroma test",
                        _ => "✦ Spectrum orb · 4-way compare",
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
            ExperienceId::ShellHud => Some(match variant {
                1 => "✦ Tasks 1/3 · runs in any terminal",
                2 => "✦ One-click macros over live shell",
                _ => "✦ Draggable tree · click cd here",
            }),
            _ => None,
        }
    }

    fn render_voxel(
        &mut self,
        width: usize,
        height: usize,
        time: f64,
        interaction: PreviewInteraction,
        out: &mut Vec<[u8; 3]>,
    ) {
        let variant = interaction.variant % ExperienceId::Voxel.variant_count();
        let pulse = pulse_age(time, interaction).filter(|&dt| dt < 1.8);
        let jump_boost = pulse.map_or(0.0, |dt| ((dt / 1.8) * std::f64::consts::PI).sin() * 2.8);

        let orbit = time * 0.24;
        let (cx, cz, radius, alt, pitch) = match variant {
            1 => (30.5, 30.5, 11.5, 15.5, -0.24),
            2 => (31.0, 31.0, 14.5, 20.5, -0.44),
            _ => (30.0, 29.0, 10.5, 14.2, -0.20),
        };
        let px = cx + orbit.cos() * radius;
        let pz = cz + orbit.sin() * radius;
        let py = alt + (time * 0.65).sin() * 0.8 + jump_boost;
        let look_dx = cx - px;
        let look_dz = cz - pz;

        self.voxel_world.player = [px, py, pz];
        self.voxel_world.yaw = look_dx.atan2(look_dz);
        self.voxel_world.pitch = pitch;
        self.voxel_world.night = variant == 1;
        self.voxel_world.time = time;

        voxel_scene::render_preview_rgb(&self.voxel_world, width, height, out);

        // Crisp subtle center crosshair + golden pulse burst on Space.
        if width >= 8 && height >= 8 {
            let cx_px = width / 2;
            let cy_px = height / 2;
            for d in -1_isize..=1 {
                let hx = (cx_px as isize + d).clamp(0, width as isize - 1) as usize;
                let hy = (cy_px as isize + d).clamp(0, height as isize - 1) as usize;
                blend_over(&mut out[cy_px * width + hx], [248.0, 250.0, 235.0], 0.65);
                blend_over(&mut out[hy * width + cx_px], [248.0, 250.0, 235.0], 0.65);
            }
        }
        if let Some(dt) = pulse {
            let fade = (1.0 - dt as f32 / 1.8).powi(2);
            draw_soft_sparkle(
                out,
                width,
                height,
                0.5,
                0.5,
                0.18,
                [160, 248, 175],
                fade * 0.65,
            );
        }
    }

    fn render_clockwork(
        &mut self,
        width: usize,
        height: usize,
        time: f64,
        interaction: PreviewInteraction,
        out: &mut [[u8; 3]],
    ) {
        let variant = interaction.variant % ExperienceId::Clockwork.variant_count();
        let pulse = pulse_age(time, interaction).filter(|&dt| dt < 2.0);
        let scatter = pulse.map_or(0.0_f32, |dt| {
            let dt = dt as f32;
            (dt * 3.8).sin().abs() * (-dt * 1.35).exp() * 1.9
        });

        let machine = Machine::snapshot_at(variant, time + 4.5, scatter);
        let camera = ClockCamera {
            yaw: 0.55 + (time as f32 * 0.34).sin() * 0.65 + time as f32 * 0.14,
            pitch: 0.36 + (time as f32 * 0.25).cos() * 0.08,
            distance: match variant {
                1 => 9.6,
                2 => 9.0,
                _ => 9.4,
            },
        };

        let rw = width.min(144);
        let rh = height.min(72);
        self.clockwork_raster.render(rw, rh, &machine, camera);

        for y in 0..height {
            let sy = y * rh / height;
            let row_out = y * width;
            let row_src = sy * rw;
            for x in 0..width {
                let sx = x * rw / width;
                let mut rgb = self.clockwork_raster.pixels[row_src + sx];
                if scatter > 0.01 {
                    rgb[0] += scatter * 0.14;
                    rgb[1] += scatter * 0.08;
                    rgb[2] += scatter * 0.03;
                }
                out[row_out + x] = f32_to_rgb(rgb);
            }
        }

        if let Some(dt) = pulse {
            let fade = (1.0 - dt as f32 / 2.0).powi(2);
            draw_soft_sparkle(
                out,
                width,
                height,
                0.5,
                0.5,
                0.26,
                [255, 206, 118],
                fade * 0.55,
            );
        }
    }

    fn render_desk(
        &mut self,
        width: usize,
        height: usize,
        time: f64,
        interaction: PreviewInteraction,
        out: &mut [[u8; 3]],
    ) {
        let variant = interaction.variant % ExperienceId::Desk.variant_count();
        let pulse = pulse_age(time, interaction).filter(|&dt| dt < 2.0);
        let t = time as f32;
        let inv_w = 1.0 / width as f32;
        let inv_h = 1.0 / height as f32;

        // Palettes inspired by Lumen's Moon, Dusk, and Paper themes.
        let (bg_top, bg_bot, accent_a, accent_b, win_bg, win_border) = match variant {
            1 => (
                [0.08, 0.06, 0.14],
                [0.14, 0.10, 0.22],
                [0.75, 0.64, 0.96],
                [0.95, 0.68, 0.60],
                [38.0, 32.0, 56.0],
                [175.0, 145.0, 232.0],
            ),
            2 => (
                [0.86, 0.84, 0.80],
                [0.76, 0.74, 0.70],
                [0.18, 0.52, 0.48],
                [0.44, 0.36, 0.68],
                [244.0, 240.0, 233.0],
                [98.0, 110.0, 134.0],
            ),
            _ => (
                [0.04, 0.06, 0.11],
                [0.07, 0.10, 0.18],
                [0.42, 0.90, 0.80],
                [0.68, 0.56, 0.98],
                [20.0, 28.0, 46.0],
                [112.0, 224.0, 206.0],
            ),
        };

        let sweep_pos = if let Some(dt) = pulse {
            (dt as f32 / 1.4).fract() * 1.6 - 0.3
        } else {
            (t * 0.28).fract() * 1.6 - 0.3
        };
        let sweep_boost = pulse.map_or(0.28_f32, |dt| 0.28 + 0.52 * (-(dt as f32) * 1.8).exp());

        // 1. Background with animated 3D perspective mesh wave.
        for y in 0..height {
            let v = (y as f32 + 0.5) * inv_h;
            let row = y * width;
            for x in 0..width {
                let u = (x as f32 + 0.5) * inv_w;
                let mut rgb = [
                    bg_top[0] + (bg_bot[0] - bg_top[0]) * v,
                    bg_top[1] + (bg_bot[1] - bg_top[1]) * v,
                    bg_top[2] + (bg_bot[2] - bg_top[2]) * v,
                ];
                // Warped topological grid lines
                let gx = (u * 8.0 + (v * 5.0 + t * 0.9).sin() * 0.35).fract().abs();
                let gy = (v * 6.0 + (u * 4.5 - t * 0.7).cos() * 0.30).fract().abs();
                let wire = ((1.0 - (gx - 0.5).abs() * 14.0).max(0.0)
                    + (1.0 - (gy - 0.5).abs() * 14.0).max(0.0))
                .min(1.0)
                    * 0.26;
                let diag_d = ((u * 0.75 + v * 0.55) - sweep_pos).abs();
                let beam = (-(diag_d / 0.085).powi(2)).exp() * sweep_boost;
                for c in 0..3 {
                    let mix_col = accent_a[c] * (1.0 - u) + accent_b[c] * u;
                    rgb[c] = (rgb[c] + mix_col * wire + mix_col * beam * 0.45).clamp(0.0, 1.0);
                }
                out[row + x] = f32_to_rgb(rgb);
            }
        }

        // 2. Two gliding frosted floating windows + bottom Dock bar.
        let drift_x = (t * 0.9).sin() * 0.035;
        let drift_y = (t * 1.15).cos() * 0.03;
        draw_frosted_window(
            out,
            width,
            height,
            0.09 + drift_x,
            0.14 + drift_y,
            0.44,
            0.56,
            win_bg,
            win_border,
            [
                (accent_a[0] * 255.0),
                (accent_a[1] * 255.0),
                (accent_a[2] * 255.0),
            ],
        );
        // Live glowing orbital ring inside Window 1
        let ox = 0.31 + drift_x;
        let oy = 0.44 + drift_y;
        for step in 0..24 {
            let ang = step as f32 / 24.0 * TAU + t * 1.1;
            let px = ox + ang.cos() * 0.13;
            let py = oy + ang.sin() * 0.16;
            let col = [
                (accent_a[0] * 255.0) as u8,
                (accent_a[1] * 255.0) as u8,
                (accent_a[2] * 255.0) as u8,
            ];
            draw_pixel_sparkle(out, width, height, px, py, col, 0.85);
        }

        // Window 2: Live signal waveform floating window
        let w2x = 0.46 - drift_x * 0.8;
        let w2y = 0.24 - drift_y * 0.9;
        draw_frosted_window(
            out,
            width,
            height,
            w2x,
            w2y,
            0.45,
            0.52,
            win_bg,
            win_border,
            [
                (accent_b[0] * 255.0),
                (accent_b[1] * 255.0),
                (accent_b[2] * 255.0),
            ],
        );
        // Animated signal curve inside Window 2
        let curve_steps = (width / 2).max(16);
        for s in 0..curve_steps {
            let f = s as f32 / curve_steps as f32;
            let px = w2x + 0.04 + f * 0.37;
            let wave = (f * 10.0 - t * 3.2).sin() * 0.08 + (f * 21.0 + t * 1.9).cos() * 0.03;
            let py = w2y + 0.30 + wave;
            let col = [
                (accent_b[0] * 255.0) as u8,
                (accent_b[1] * 255.0) as u8,
                (accent_b[2] * 255.0) as u8,
            ];
            draw_soft_sparkle(out, width, height, px, py, 0.022, col, 0.72);
        }

        // Floating Dock pills along bottom
        draw_rect_blend(out, width, height, 0.28, 0.84, 0.44, 0.10, win_bg, 0.82);
        for i in 0..4 {
            let px = 0.32 + i as f32 * 0.10;
            let active = (i == variant) || (pulse.is_some() && i == 3);
            let col = if active {
                [
                    (accent_a[0] * 255.0),
                    (accent_a[1] * 255.0),
                    (accent_a[2] * 255.0),
                ]
            } else {
                win_border
            };
            draw_rect_blend(out, width, height, px, 0.86, 0.065, 0.06, col, 0.85);
        }
    }

    fn render_hud(
        &mut self,
        width: usize,
        height: usize,
        time: f64,
        interaction: PreviewInteraction,
        out: &mut [[u8; 3]],
    ) {
        let variant = interaction.variant % ExperienceId::ShellHud.variant_count();
        let pulse = pulse_age(time, interaction).filter(|&dt| dt < 2.0);
        let t = time as f32;
        let inv_w = 1.0 / width as f32;
        let inv_h = 1.0 / height as f32;

        // 1. Live terminal shell backdrop with subtle depth gradient.
        for y in 0..height {
            let v = (y as f32 + 0.5) * inv_h;
            let row = y * width;
            for x in 0..width {
                let u = (x as f32 + 0.5) * inv_w;
                let glow = (-((u - 0.74).powi(2) * 4.5 + (v - 0.36).powi(2) * 4.0)).exp();
                let rgb = [
                    0.032 + glow * 0.035,
                    0.044 + glow * 0.075,
                    0.078 + glow * 0.105,
                ];
                out[row + x] = f32_to_rgb(rgb);
            }
        }

        // 2. Animated terminal shell output lines on the left side.
        draw_terminal_diorama_lines(out, width, height, t);

        // 3. Floating translucent Shell HUD panel in top-right corner!
        let (accent, tab_idx) = match variant {
            1 => ([110.0, 232.0, 165.0], 1usize),
            2 => ([245.0, 195.0, 102.0], 2usize),
            _ => ([96.0, 224.0, 198.0], 0usize),
        };
        let bob = (t * 1.4).sin() * 0.012;
        let hx = 0.47;
        let hy = 0.09 + bob;
        let hw = 0.47;
        let hh = 0.74;
        draw_frosted_window(
            out,
            width,
            height,
            hx,
            hy,
            hw,
            hh,
            [18.0, 25.0, 42.0],
            accent,
            accent,
        );

        // Status timer strip inside HUD
        let running = pulse.is_some() || (t * 0.45).sin() > 0.25;
        let status_col = if running {
            [248.0, 204.0, 96.0]
        } else {
            [110.0, 232.0, 165.0]
        };
        let status_bar_w = if running {
            0.14 + 0.22 * ((t * 1.6).sin() * 0.5 + 0.5)
        } else {
            0.34
        };
        draw_rect_blend(
            out,
            width,
            height,
            hx + 0.04,
            hy + 0.13,
            status_bar_w,
            0.055,
            status_col,
            0.85,
        );

        // 3 Tab pills
        for i in 0..3 {
            let tx = hx + 0.04 + i as f32 * 0.135;
            let active = i == tab_idx;
            let col = if active { accent } else { [56.0, 70.0, 102.0] };
            draw_rect_blend(
                out,
                width,
                height,
                tx,
                hy + 0.22,
                0.115,
                0.06,
                col,
                if active { 0.90 } else { 0.65 },
            );
        }

        // Tab content rows (Files / Tasks / Quick macros)
        for row_i in 0..4 {
            let ry = hy + 0.34 + row_i as f32 * 0.095;
            let indent = if variant == 0 && row_i > 0 { 0.04 } else { 0.0 };
            let bar_w = match variant {
                0 => [0.24, 0.28, 0.20, 0.26][row_i],
                1 => [0.30, 0.25, 0.32, 0.18][row_i],
                _ => 0.36,
            };
            let checked = (variant == 1 && (row_i == 0 || (pulse.is_some() && row_i == 1)))
                || (variant == 2 && row_i == 0 && pulse.is_some());
            let row_col = if checked {
                [110.0, 232.0, 165.0]
            } else if row_i == ((t * 0.9) as usize % 4) {
                accent
            } else {
                [128.0, 146.0, 182.0]
            };
            // Checkbox / folder glyph dot
            draw_rect_blend(
                out,
                width,
                height,
                hx + 0.04 + indent,
                ry,
                0.032,
                0.055,
                row_col,
                0.92,
            );
            // Label bar
            draw_rect_blend(
                out,
                width,
                height,
                hx + 0.09 + indent,
                ry + 0.01,
                bar_w - indent,
                0.038,
                row_col,
                0.62,
            );
        }

        if let Some(dt) = pulse {
            let fade = (1.0 - dt as f32 / 2.0).powi(2);
            draw_soft_sparkle(
                out,
                width,
                height,
                hx + hw * 0.5,
                hy + hh * 0.5,
                0.22,
                [115, 245, 210],
                fade * 0.55,
            );
        }
    }

    fn render_doctor(
        &mut self,
        width: usize,
        height: usize,
        time: f64,
        interaction: PreviewInteraction,
        out: &mut [[u8; 3]],
    ) {
        let variant = interaction.variant % DoctorPattern::ALL.len();
        let pattern = DoctorPattern::ALL[variant];
        let pulse = pulse_age(time, interaction).filter(|&dt| dt < 2.0);
        let t = time as f32;
        let mid_x = width / 2;
        let mid_y = height / 2;
        let scan_x = pulse.map(|dt| (dt as f32 / 1.4).clamp(0.0, 1.0));

        for y in 0..height {
            let qy = if y < mid_y { 0 } else { 1 };
            let local_h = if qy == 0 {
                mid_y.max(1)
            } else {
                (height - mid_y).max(1)
            };
            let ly = if qy == 0 { y } else { y - mid_y };
            let v = (ly as f32 + 0.5) / local_h as f32 * 2.0 - 1.0;
            let row = y * width;

            for x in 0..width {
                if x == mid_x || y == mid_y {
                    out[row + x] = [56, 72, 108];
                    continue;
                }
                let qx = if x < mid_x { 0 } else { 1 };
                let local_w = if qx == 0 {
                    mid_x.max(1)
                } else {
                    (width - mid_x).max(1)
                };
                let lx = if qx == 0 { x } else { x - mid_x };
                let u = (lx as f32 + 0.5) / local_w as f32 * 2.0 - 1.0;

                let mut rgb = match (qx, qy) {
                    // Top-left: Braille-style crisp luminous dot grid
                    (0, 0) => {
                        let c = doctor::sample_pattern(pattern, u, v, t);
                        if (lx + ly) % 2 == 0 {
                            [
                                c[0].saturating_add(20),
                                c[1].saturating_add(10),
                                c[2].saturating_add(30),
                            ]
                        } else {
                            [c[0] / 3, c[1] / 3, (c[2] / 2).max(16)]
                        }
                    }
                    // Top-right: Smooth HalfBlock truecolor shading
                    (1, 0) => doctor::sample_pattern(pattern, u, v, t),
                    // Bottom-left: Quadrant 2x2 crisp pixel quantization
                    (0, 1) => {
                        let qu = (((lx / 2) * 2) as f32 + 1.0) / local_w as f32 * 2.0 - 1.0;
                        let qv = (((ly / 2) * 2) as f32 + 1.0) / local_h as f32 * 2.0 - 1.0;
                        doctor::sample_pattern(pattern, qu, qv, t)
                    }
                    // Bottom-right: Native pixel RGB high-vibrance reference
                    _ => {
                        let c = doctor::sample_pattern(pattern, u, v, t);
                        [
                            (f32::from(c[0]) * 1.12).min(255.0) as u8,
                            (f32::from(c[1]) * 1.12).min(255.0) as u8,
                            (f32::from(c[2]) * 1.12).min(255.0) as u8,
                        ]
                    }
                };

                if let Some(sx) = scan_x {
                    let fx = x as f32 / width.max(1) as f32;
                    let beam = (-((fx - sx) / 0.045).powi(2)).exp();
                    if beam > 0.02 {
                        rgb[0] = add_clamp(rgb[0], 110.0 * beam);
                        rgb[1] = add_clamp(rgb[1], 210.0 * beam);
                        rgb[2] = add_clamp(rgb[2], 255.0 * beam);
                    }
                }
                out[row + x] = rgb;
            }
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
        let sea_time = time + pulse.map_or(0.0, |dt| 0.55 * (1.0 - (-dt * 2.5).exp()));

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

                rgb = grade_jellyfish(rgb, variant);

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

        let (rw, rh) = choose_plot_resolution(width, height);
        let canvas = preset.plot.image_at(preset.bounds, (rw, rh), angle);
        let src = canvas.pixels();
        let rw_usize = rw as usize;
        let rh_usize = rh as usize;

        let inv_w = 1.0 / width as f32;
        let inv_h = 1.0 / height as f32;
        let t = time as f32;

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

                let bg_dist = ((r - 18.0 / 255.0).abs()
                    + (g - 22.0 / 255.0).abs()
                    + (b - 30.0 / 255.0).abs())
                .min(1.0);

                let dx = (u - 0.5) * 1.45;
                let dy = (v - 0.5) * 1.65;
                let radial = (-1.35 * (dx * dx + dy * dy)).exp();
                let vignette = 0.58 + 0.42 * radial;

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
                    let fg_strength = ((bg_dist - 0.025) / 0.18).clamp(0.0, 1.0);
                    let lift = 1.0 + 0.16 * fg_strength + 0.22 * glow_pulse * fg_strength;
                    let mut fr = r * lift;
                    let mut fg = g * lift;
                    let mut fb = b * lift;

                    if preset.is_spatial && !preset.is_curve_3d && fg_strength > 0.35 {
                        let luma = (r * 0.25 + g * 0.55 + b * 0.20).clamp(0.0, 1.0);
                        if variant == 0 {
                            let ridge = ((luma - 0.42) / 0.45).clamp(0.0, 1.0);
                            fr = fr * (0.76 + 0.52 * ridge) + 0.08 * ridge;
                            fg = fg * (0.86 + 0.24 * ridge) + 0.04 * ridge;
                            fb *= 1.06 - 0.04 * ridge;
                        } else {
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

                    if !preset.is_spatial && fg_strength > 0.35 {
                        let theta = (v - 0.5).atan2((u - 0.5) * (width as f32 * inv_h));
                        let shimmer = ((theta * 2.0 - t * 2.6).sin() * 0.5 + 0.5).powi(3)
                            * (0.24 + 0.35 * glow_pulse);
                        r += 0.30 * shimmer;
                        g += 0.24 * shimmer;
                        b += 0.36 * shimmer;
                    }
                }

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

        if preset.is_curve_3d && width >= 4 && height >= 4 {
            apply_subtle_bloom(out, width, height, 0.28);
        }

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

        for y in 0..height {
            let v = (y as f32 + 0.5) * inv_h;
            let row_offset = y * width;
            for x in 0..width {
                let u = (x as f32 + 0.5) * inv_w;
                let vig = (1.0 - (u - 0.5).powi(2) * 1.15).clamp(0.45, 1.0);

                let c1 = (u * 13.5 + v * 4.2 + t * 1.15).sin();
                let c2 = (u * 21.0 - v * 5.8 - t * 0.88 + 1.2).cos();
                let caustic = ((c1 * c2) * 0.5 + 0.5).powi(4) * (1.0 - v).powf(1.45);
                let ray = (u * 8.0 - v * 2.4 + (t * 0.4).sin() * 0.8)
                    .sin()
                    .max(0.0)
                    .powi(6)
                    * (1.0 - v)
                    * 0.65;

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

        draw_terminal_diorama_lines(out, width, height, t);

        let sprite_side = ((height as f32 * 0.78).min(width as f32 * 0.64)).max(6.0) as usize;
        let max_swim_x = ((width as f32 - sprite_side as f32) * 0.34).max(0.0);
        let min_center_y = sprite_side as f32 * 0.50 + 0.5;
        let max_center_y = (height as f32 - sprite_side as f32 * 0.46).max(min_center_y);

        let (mut whale_x, mut whale_y, leftward, clip_kind, clip_time) = match variant {
            0 => {
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
                let x = width as f32 * 0.54 + (t * 0.45).sin() * (max_swim_x * 0.20);
                let y = height as f32 * 0.59 + (t * 1.05).sin() * (height as f32 * 0.04);
                (x, y, false, PetClip::Nap, time)
            }
        };

        let mut active_clip = clip_kind;
        let mut active_clip_time = clip_time;

        if let Some(dt) = pulse {
            let dt_f = dt as f32;
            let jump_p = (dt_f / 1.15).min(1.0);
            let arc = (jump_p * PI).sin();
            whale_y = height as f32 * 0.60 - arc * (height as f32 * 0.15);
            whale_x += (dt_f * 2.5).sin() * (width as f32 * 0.02);
            active_clip = PetClip::Hop;
            active_clip_time = dt;

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

#[allow(clippy::too_many_arguments)]
fn draw_rect_blend(
    out: &mut [[u8; 3]],
    width: usize,
    height: usize,
    x_frac: f32,
    y_frac: f32,
    w_frac: f32,
    h_frac: f32,
    color: [f32; 3],
    alpha: f32,
) {
    let x0 = ((x_frac * width as f32).round() as isize).clamp(0, width as isize) as usize;
    let y0 = ((y_frac * height as f32).round() as isize).clamp(0, height as isize) as usize;
    let x1 =
        (((x_frac + w_frac) * width as f32).round() as isize).clamp(0, width as isize) as usize;
    let y1 =
        (((y_frac + h_frac) * height as f32).round() as isize).clamp(0, height as isize) as usize;
    for py in y0..y1 {
        let row = py * width;
        for px in x0..x1 {
            blend_over(&mut out[row + px], color, alpha);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_frosted_window(
    out: &mut [[u8; 3]],
    width: usize,
    height: usize,
    x_frac: f32,
    y_frac: f32,
    w_frac: f32,
    h_frac: f32,
    fill: [f32; 3],
    border: [f32; 3],
    title_accent: [f32; 3],
) {
    let x0 = ((x_frac * width as f32).round() as isize).clamp(0, width as isize) as usize;
    let y0 = ((y_frac * height as f32).round() as isize).clamp(0, height as isize) as usize;
    let x1 =
        (((x_frac + w_frac) * width as f32).round() as isize).clamp(0, width as isize) as usize;
    let y1 =
        (((y_frac + h_frac) * height as f32).round() as isize).clamp(0, height as isize) as usize;
    if x1 <= x0 + 1 || y1 <= y0 + 1 {
        return;
    }
    let title_h = ((y1 - y0) / 6).clamp(1, 3);
    for py in y0..y1 {
        let row = py * width;
        for px in x0..x1 {
            let on_edge = px == x0 || px + 1 == x1 || py == y0 || py + 1 == y1;
            if on_edge {
                blend_over(&mut out[row + px], border, 0.85);
            } else if py < y0 + 1 + title_h {
                blend_over(&mut out[row + px], title_accent, 0.30);
                blend_over(&mut out[row + px], fill, 0.65);
            } else {
                blend_over(&mut out[row + px], fill, 0.78);
            }
        }
    }
    // Window control dots
    const DOTS: [[f32; 3]; 3] = [
        [242.0, 110.0, 128.0],
        [248.0, 198.0, 96.0],
        [106.0, 228.0, 154.0],
    ];
    let dy = (y0 + 1).min(y1 - 1);
    for (i, &col) in DOTS.iter().enumerate() {
        let dx = x0 + 2 + i * 2;
        if dx + 1 < x1 {
            blend_over(&mut out[dy * width + dx], col, 0.92);
        }
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
    let target_w = (width.saturating_mul(2)).max(1);
    let target_h = (height.saturating_mul(2)).max(1);
    if target_w < 160 && target_h < 80 {
        return (target_w as u32, target_h as u32);
    }
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
        let _ = (clip, leftward);
        draw_soft_sparkle(
            out,
            width,
            height,
            center_x as f32 / width.max(1) as f32,
            center_y as f32 / height.max(1) as f32,
            (sprite_side as f32 / height.max(1) as f32) * 0.45,
            [86, 178, 255],
            0.9,
        );
        let _ = clip_time;
    }
}

#[allow(clippy::too_many_arguments)]
#[cfg(any(unix, windows))]
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

#[allow(clippy::too_many_arguments)]
#[cfg(any(unix, windows))]
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
            let unique: std::collections::HashSet<_> = a.iter().copied().collect();
            assert!(
                unique.len() > 25,
                "{exp:?} has too few colors: {}",
                unique.len()
            );

            let alt = PreviewInteraction { variant: 1, ..base };
            engine.render_into(exp, w, h, 0.5, alt, &mut b);
            assert_ne!(a, b, "{exp:?} variant 1 must differ from variant 0");

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
