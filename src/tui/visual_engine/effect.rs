use crate::tui::style::Color;
use crate::tui::visual_engine::custom::{CellEffect, CustomCellEffect};
use crate::tui::visual_engine::math::{
    fade_for_visibility_mode, finite_or, sanitize_delay, sanitize_softness,
};
use crate::tui::visual_engine::profile::VisualEffectCost;

/// A terminal-cell visual effect.
#[derive(Debug, Clone, PartialEq)]
pub enum VisualEffect {
    Fade {
        from: f64,
        to: f64,
    },
    Gradient {
        start: Color,
        end: Color,
        direction: GradientDirection,
        target: GradientTarget,
        phase: f64,
    },
    Shatter {
        seed: u64,
        spread_x: f64,
        spread_y: f64,
        fade: bool,
    },
    MagicLamp {
        anchor: VisualAnchor,
        squeeze: f64,
    },
    Wipe {
        direction: WipeDirection,
        mode: WipeMode,
        softness: f64,
    },
    Dissolve {
        seed: u64,
        mode: DissolveMode,
    },
    Wave {
        axis: WaveAxis,
        amplitude: f64,
        wavelength: f64,
        phase: f64,
    },
    Glitch {
        seed: u64,
        intensity: f64,
    },
    Scanline {
        density: f64,
        intensity: f64,
        phase: f64,
    },
    Typewriter {
        mode: TypewriterMode,
        cursor: bool,
    },
    BlurLike {
        radius: f64,
        mode: BlurMode,
    },
    HighlightSweep {
        color: Color,
        width: f64,
        direction: GradientDirection,
    },
    Sparkle {
        seed: u64,
        density: f64,
        color: Color,
    },
    Sequence(Vec<VisualEffect>),
    Parallel(Vec<VisualEffect>),
    Stagger {
        delay: f64,
        mode: StaggerMode,
        effect: Box<VisualEffect>,
    },
    Custom(CustomCellEffect),
}

impl VisualEffect {
    pub fn fade_in() -> Self {
        Self::Fade { from: 0.0, to: 1.0 }
    }

    pub fn fade_out() -> Self {
        Self::Fade { from: 1.0, to: 0.0 }
    }

    pub fn gradient(start: Color, end: Color, direction: GradientDirection) -> Self {
        Self::Gradient {
            start,
            end,
            direction,
            target: GradientTarget::Foreground,
            phase: 0.0,
        }
    }

    pub fn background_gradient(start: Color, end: Color, direction: GradientDirection) -> Self {
        Self::Gradient {
            start,
            end,
            direction,
            target: GradientTarget::Background,
            phase: 0.0,
        }
    }

    pub fn shatter() -> Self {
        Self::Shatter {
            seed: 0x5EED,
            spread_x: 18.0,
            spread_y: 7.0,
            fade: true,
        }
    }

    pub fn magic_lamp(anchor: VisualAnchor) -> Self {
        Self::MagicLamp {
            anchor,
            squeeze: 0.08,
        }
    }

    pub fn wipe(direction: WipeDirection) -> Self {
        Self::Wipe {
            direction,
            mode: WipeMode::Hide,
            softness: 0.0,
        }
    }

    pub fn reveal(direction: WipeDirection) -> Self {
        Self::Wipe {
            direction,
            mode: WipeMode::Reveal,
            softness: 0.0,
        }
    }

    pub fn dissolve() -> Self {
        Self::Dissolve {
            seed: 0xD155_01F3,
            mode: DissolveMode::In,
        }
    }

    pub fn dissolve_out() -> Self {
        Self::Dissolve {
            seed: 0xD155_01F3,
            mode: DissolveMode::Out,
        }
    }

    pub fn wave(axis: WaveAxis) -> Self {
        Self::Wave {
            axis,
            amplitude: 2.0,
            wavelength: 6.0,
            phase: 0.0,
        }
    }

    pub fn glitch() -> Self {
        Self::Glitch {
            seed: 0x6_117C_4,
            intensity: 0.42,
        }
    }

    pub fn scanline() -> Self {
        Self::Scanline {
            density: 0.5,
            intensity: 0.35,
            phase: 0.0,
        }
    }

    pub fn typewriter() -> Self {
        Self::Typewriter {
            mode: TypewriterMode::Chars,
            cursor: false,
        }
    }

    pub fn typewriter_words() -> Self {
        Self::Typewriter {
            mode: TypewriterMode::Words,
            cursor: false,
        }
    }

    pub fn blur_like() -> Self {
        Self::BlurLike {
            radius: 2.0,
            mode: BlurMode::In,
        }
    }

    pub fn highlight_sweep() -> Self {
        Self::HighlightSweep {
            color: Color::Rgb(255, 255, 180),
            width: 0.18,
            direction: GradientDirection::Horizontal,
        }
    }

    pub fn sparkle() -> Self {
        Self::Sparkle {
            seed: 0x5_9A4C_13,
            density: 0.08,
            color: Color::Rgb(255, 255, 200),
        }
    }

    /// Wrap a user-defined cell effect so it can be composed with built-ins.
    pub fn custom(effect: impl CellEffect) -> Self {
        Self::custom_named("custom", effect)
    }

    /// Wrap a user-defined cell effect with a stable debug/profiling name.
    pub fn custom_named(name: &'static str, effect: impl CellEffect) -> Self {
        Self::Custom(CustomCellEffect::new(name, effect))
    }

    /// Run child effects one after another with equal duration slices.
    ///
    /// Completed children are sampled at `1.0`; the active child receives its
    /// local progress; pending children are skipped.
    pub fn sequence(effects: Vec<VisualEffect>) -> Self {
        Self::Sequence(effects)
    }

    /// Run child effects over the same progress range.
    ///
    /// Children are applied in list order. Later children see earlier geometry
    /// and color changes; visibility remains hidden once any child hides a cell.
    pub fn parallel(effects: Vec<VisualEffect>) -> Self {
        Self::Parallel(effects)
    }

    /// Delay a child effect by local row.
    pub fn stagger_rows(delay: f64, effect: VisualEffect) -> Self {
        Self::stagger(delay, StaggerMode::Rows, effect)
    }

    /// Delay a child effect by local column.
    pub fn stagger_cols(delay: f64, effect: VisualEffect) -> Self {
        Self::stagger(delay, StaggerMode::Cols, effect)
    }

    /// Delay a child effect by source character index, row-major.
    pub fn stagger_chars(delay: f64, effect: VisualEffect) -> Self {
        Self::stagger(delay, StaggerMode::Chars, effect)
    }

    fn stagger(delay: f64, mode: StaggerMode, effect: VisualEffect) -> Self {
        Self::Stagger {
            delay: sanitize_delay(delay),
            mode,
            effect: Box::new(effect),
        }
    }

    /// Return the reduced-motion form of this effect.
    ///
    /// Spatial motion is replaced with fades, while color-only effects remain
    /// unchanged. Composition is preserved with zero stagger delay.
    pub fn reduced(&self) -> Self {
        match self {
            Self::Fade { .. } | Self::Gradient { .. } | Self::Scanline { .. } => self.clone(),
            Self::Shatter { .. } => Self::fade_out(),
            Self::MagicLamp { .. } => Self::fade_in(),
            Self::Wipe { mode, .. } => fade_for_visibility_mode(*mode),
            Self::Dissolve { mode, .. } => match mode {
                DissolveMode::In => Self::fade_in(),
                DissolveMode::Out => Self::fade_out(),
            },
            Self::Wave { .. } | Self::Glitch { .. } => Self::fade_in(),
            Self::Typewriter { .. } | Self::BlurLike { .. } | Self::Sparkle { .. } => {
                Self::fade_in()
            }
            Self::HighlightSweep {
                color,
                width,
                direction,
            } => Self::HighlightSweep {
                color: *color,
                width: (*width * 0.5).clamp(0.02, 1.0),
                direction: *direction,
            },
            Self::Sequence(effects) => Self::Sequence(
                effects
                    .iter()
                    .map(VisualEffect::reduced)
                    .collect::<Vec<_>>(),
            ),
            Self::Parallel(effects) => Self::Parallel(
                effects
                    .iter()
                    .map(VisualEffect::reduced)
                    .collect::<Vec<_>>(),
            ),
            Self::Stagger { effect, mode, .. } => Self::Stagger {
                delay: 0.0,
                mode: *mode,
                effect: Box::new(effect.reduced()),
            },
            Self::Custom(effect) => Self::Custom(effect.clone()),
        }
    }

    /// Approximate cost used by degradation and profiling hooks.
    pub fn estimated_cost(&self) -> VisualEffectCost {
        match self {
            Self::Fade { .. }
            | Self::Gradient { .. }
            | Self::Scanline { .. }
            | Self::Typewriter { .. }
            | Self::HighlightSweep { .. }
            | Self::Wipe { softness: 0.0, .. } => VisualEffectCost::Cheap,
            Self::Wipe { .. }
            | Self::MagicLamp { .. }
            | Self::Wave { .. }
            | Self::BlurLike { .. }
            | Self::Sparkle { .. } => VisualEffectCost::Moderate,
            Self::Sequence(effects) | Self::Parallel(effects) => effects
                .iter()
                .map(VisualEffect::estimated_cost)
                .fold(VisualEffectCost::Cheap, VisualEffectCost::max),
            Self::Stagger { effect, .. } => effect.estimated_cost().max(VisualEffectCost::Moderate),
            Self::Shatter { .. } | Self::Dissolve { .. } | Self::Glitch { .. } => {
                VisualEffectCost::Expensive
            }
            Self::Custom(effect) => effect.effect.estimated_cost(),
        }
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        match &mut self {
            Self::Shatter { seed: current, .. }
            | Self::Dissolve { seed: current, .. }
            | Self::Glitch { seed: current, .. }
            | Self::Sparkle { seed: current, .. } => {
                *current = seed;
            }
            _ => {}
        }
        self
    }

    pub fn with_spread(mut self, x: f64, y: f64) -> Self {
        if let Self::Shatter {
            spread_x, spread_y, ..
        } = &mut self
        {
            *spread_x = x.max(0.0);
            *spread_y = y.max(0.0);
        }
        self
    }

    pub fn target(mut self, target: GradientTarget) -> Self {
        if let Self::Gradient {
            target: current, ..
        } = &mut self
        {
            *current = target;
        }
        self
    }

    pub fn phase(mut self, phase: f64) -> Self {
        match &mut self {
            Self::Gradient { phase: current, .. }
            | Self::Wave { phase: current, .. }
            | Self::Scanline { phase: current, .. } => {
                *current = finite_or(phase, 0.0);
            }
            _ => {}
        }
        self
    }

    pub fn squeeze(mut self, squeeze: f64) -> Self {
        if let Self::MagicLamp {
            squeeze: current, ..
        } = &mut self
        {
            *current = squeeze.clamp(0.0, 1.0);
        }
        self
    }

    pub fn softness(mut self, softness: f64) -> Self {
        if let Self::Wipe {
            softness: current, ..
        } = &mut self
        {
            *current = sanitize_softness(softness);
        }
        self
    }

    pub fn amplitude(mut self, amplitude: f64) -> Self {
        if let Self::Wave {
            amplitude: current, ..
        } = &mut self
        {
            *current = finite_or(amplitude, 0.0).max(0.0);
        }
        self
    }

    pub fn wavelength(mut self, wavelength: f64) -> Self {
        if let Self::Wave {
            wavelength: current,
            ..
        } = &mut self
        {
            *current = finite_or(wavelength, 6.0).max(f64::EPSILON);
        }
        self
    }

    pub fn intensity(mut self, intensity: f64) -> Self {
        match &mut self {
            Self::Glitch {
                intensity: current, ..
            }
            | Self::Scanline {
                intensity: current, ..
            } => {
                *current = finite_or(intensity, 0.0).clamp(0.0, 1.0);
            }
            _ => {}
        }
        self
    }

    pub fn density(mut self, density: f64) -> Self {
        match &mut self {
            Self::Scanline {
                density: current, ..
            }
            | Self::Sparkle {
                density: current, ..
            } => {
                *current = finite_or(density, 0.0).clamp(0.0, 1.0);
            }
            _ => {}
        }
        self
    }

    pub fn cursor(mut self, cursor: bool) -> Self {
        if let Self::Typewriter {
            cursor: current, ..
        } = &mut self
        {
            *current = cursor;
        }
        self
    }

    pub fn radius(mut self, radius: f64) -> Self {
        if let Self::BlurLike {
            radius: current, ..
        } = &mut self
        {
            *current = finite_or(radius, 0.0).max(0.0);
        }
        self
    }

    pub fn blur_mode(mut self, mode: BlurMode) -> Self {
        if let Self::BlurLike { mode: current, .. } = &mut self {
            *current = mode;
        }
        self
    }

    pub fn width(mut self, width: f64) -> Self {
        if let Self::HighlightSweep { width: current, .. } = &mut self {
            *current = finite_or(width, 0.18).clamp(f64::EPSILON, 1.0);
        }
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        match &mut self {
            Self::HighlightSweep { color: current, .. } | Self::Sparkle { color: current, .. } => {
                *current = color;
            }
            _ => {}
        }
        self
    }

    pub fn direction(mut self, direction: GradientDirection) -> Self {
        if let Self::HighlightSweep {
            direction: current, ..
        } = &mut self
        {
            *current = direction;
        }
        self
    }

    pub(crate) fn can_use_dirty_only(&self) -> bool {
        match self {
            Self::Gradient { .. } | Self::Scanline { .. } => true,
            Self::Sequence(effects) | Self::Parallel(effects) => {
                effects.iter().all(VisualEffect::can_use_dirty_only)
            }
            Self::Stagger { effect, .. } => effect.can_use_dirty_only(),
            Self::Custom(effect) => effect.effect.can_use_dirty_only(),
            _ => false,
        }
    }
}

/// Spatial axis used by staggered visual effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaggerMode {
    Rows,
    Cols,
    Chars,
}

/// Direction used by [`VisualEffect::Gradient`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientDirection {
    Horizontal,
    Vertical,
    Diagonal,
}

/// Which color channel a gradient should override.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientTarget {
    Foreground,
    Background,
}

/// Direction used by [`VisualEffect::Wipe`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WipeDirection {
    LeftToRight,
    RightToLeft,
    TopToBottom,
    BottomToTop,
    CenterOut,
    EdgesIn,
    DiagonalDown,
    DiagonalUp,
}

/// Visibility mode for wipe-style effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WipeMode {
    Reveal,
    Hide,
}

/// Visibility mode for dissolve effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DissolveMode {
    In,
    Out,
}

/// Visibility grouping used by [`VisualEffect::Typewriter`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypewriterMode {
    Chars,
    /// Approximate word grouping that keeps the effect cell-local.
    Words,
}

/// Direction of terminal-cell blur approximation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlurMode {
    In,
    Out,
}

/// Axis used by [`VisualEffect::Wave`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveAxis {
    Rows,
    Cols,
}

/// Anchor point for spatial effects such as magic-lamp minimization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VisualAnchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
    /// Normalized coordinates where (0, 0) is top-left and (1, 1) is bottom-right.
    Relative(f64, f64),
}

impl VisualAnchor {
    pub(crate) fn resolve(self, width: u16, height: u16) -> (f64, f64) {
        let w = width.saturating_sub(1) as f64;
        let h = height.saturating_sub(1) as f64;
        let (x, y) = match self {
            Self::TopLeft => (0.0, 0.0),
            Self::Top => (0.5, 0.0),
            Self::TopRight => (1.0, 0.0),
            Self::Left => (0.0, 0.5),
            Self::Center => (0.5, 0.5),
            Self::Right => (1.0, 0.5),
            Self::BottomLeft => (0.0, 1.0),
            Self::Bottom => (0.5, 1.0),
            Self::BottomRight => (1.0, 1.0),
            Self::Relative(x, y) => (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)),
        };
        (x * w, y * h)
    }
}
