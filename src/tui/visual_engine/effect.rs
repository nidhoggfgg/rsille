//! Built-in effect descriptions, composition, and per-effect configuration.

use crate::tui::style::Color;
use crate::tui::visual_engine::custom::{CellEffect, CustomCellEffect};
use crate::tui::visual_engine::math::{
    fade_for_visibility_mode, finite_or, sanitize_delay, sanitize_softness,
};
use crate::tui::visual_engine::profile::VisualEffectCost;

/// A terminal-cell visual effect.
///
/// Attach effects to a [`Visual`](crate::tui::widgets::Visual) wrapper to
/// transform its rendered cells. Progress is supplied by the wrapper; creating
/// an effect does not start a timer. Geometry changes affect painting only.
///
/// Builder methods configure only the matching variant and otherwise return the
/// effect unchanged. They do not descend into composed effects: configure each
/// child before passing it to [`Self::sequence`] or [`Self::parallel`].
///
/// ```
/// use rsille::tui::prelude::*;
///
/// let effect = VisualEffect::parallel(vec![
///     VisualEffect::reveal(WipeDirection::LeftToRight).softness(0.1),
///     VisualEffect::gradient(Color::Cyan, Color::Blue, GradientDirection::Horizontal),
/// ]);
/// let widget = visual(label::<()>("Loading")).enter(effect);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum VisualEffect {
    /// Reveal or hide cells using a deterministic spatial visibility threshold.
    /// This approximates opacity by selecting whole cells, without alpha blending.
    Fade {
        /// Initial visible coverage, conventionally in `0.0..=1.0`.
        from: f64,
        /// Final visible coverage; interpolated coverage is clamped to `0.0..=1.0`.
        to: f64,
    },
    /// Interpolate a color channel across destination positions, independent of progress.
    Gradient {
        /// Color at the start of the gradient.
        start: Color,
        /// Color at the end of the gradient.
        end: Color,
        /// Spatial direction of interpolation.
        direction: GradientDirection,
        /// Color channel to replace.
        target: GradientTarget,
        /// Offset in gradient cycles; positions outside `0.0..=1.0` wrap.
        phase: f64,
    },
    /// Scatter cells outward as progress increases, optionally hiding them.
    Shatter {
        /// Deterministic seed for displacement and visibility noise.
        seed: u64,
        /// Horizontal spread scale before cell-aspect adjustment.
        spread_x: f64,
        /// Vertical spread scale in cell units.
        spread_y: f64,
        /// Whether scattered cells also disappear as progress increases.
        fade: bool,
    },
    /// Expand cells from a squeezed shape around an anchor as progress increases.
    MagicLamp {
        /// Point toward which the geometry is pulled at low progress.
        anchor: VisualAnchor,
        /// Residual scale at zero progress, normally in `0.0..=1.0`.
        squeeze: f64,
    },
    /// Move a visibility boundary through the source area.
    Wipe {
        /// Direction in which the boundary travels.
        direction: WipeDirection,
        /// Whether cells behind the boundary are shown or hidden.
        mode: WipeMode,
        /// Half-width of the dithered boundary in normalized area units; zero is sharp.
        softness: f64,
    },
    /// Reveal or hide cells in a deterministic random order.
    Dissolve {
        /// Seed combined with the wrapper seed to select cell thresholds.
        seed: u64,
        /// Whether increasing progress reveals or hides cells.
        mode: DissolveMode,
    },
    /// Displace rows or columns sinusoidally, with motion strongest at mid-progress.
    Wave {
        /// Grouping and displacement direction of the wave.
        axis: WaveAxis,
        /// Peak displacement scale; horizontal displacement is aspect-adjusted.
        amplitude: f64,
        /// Distance per cycle in rows or aspect-corrected horizontal units.
        wavelength: f64,
        /// Offset in wave cycles.
        phase: f64,
    },
    /// Temporarily displace, recolor, and replace selected cells near mid-progress.
    Glitch {
        /// Seed combined with the wrapper seed for repeatable corruption.
        seed: u64,
        /// Maximum corruption strength in `0.0..=1.0`.
        intensity: f64,
    },
    /// Dim periodic source rows, independently of progress.
    Scanline {
        /// Row-band density in `0.0..=1.0`; zero disables the effect.
        density: f64,
        /// Dimming strength in `0.0..=1.0`.
        intensity: f64,
        /// Offset in row-band cycles.
        phase: f64,
    },
    /// Reveal source cells in row-major order, optionally displaying a cursor.
    Typewriter {
        /// Cell grouping used for the reveal.
        mode: TypewriterMode,
        /// Whether to replace the next unrevealed sampled cell with a cursor.
        cursor: bool,
    },
    /// Approximate blur with dimming and glyph replacement, without sampling neighbors.
    BlurLike {
        /// Nonnegative pattern parameter; zero disables the effect.
        /// Positive values change the noise pattern, not a convolution radius.
        radius: f64,
        /// Whether the approximation clears or builds as progress increases.
        mode: BlurMode,
    },
    /// Move a colored, bold highlight through destination cells.
    HighlightSweep {
        /// Foreground color toward which highlighted cells are blended.
        color: Color,
        /// Normalized distance from the band center to its edge, in `0.0..=1.0`.
        width: f64,
        /// Direction of the moving band.
        direction: GradientDirection,
    },
    /// Twinkle selected non-whitespace cells using the render frame number.
    Sparkle {
        /// Seed combined with the wrapper seed for spatial and temporal noise.
        seed: u64,
        /// Candidate-cell density, clamped to `0.0..=0.35` when sampled.
        density: f64,
        /// Foreground color of active sparkles.
        color: Color,
    },
    /// Apply children in equal progress slices; see [`Self::sequence`].
    Sequence(Vec<VisualEffect>),
    /// Apply children in list order at shared progress; see [`Self::parallel`].
    Parallel(Vec<VisualEffect>),
    /// Offset a child's progress according to its source position.
    Stagger {
        /// Delay per row, column, or cell as a fraction of total progress.
        /// The accumulated offset is capped at `0.95` so every group can finish.
        delay: f64,
        /// Source-position grouping used to calculate the delay.
        mode: StaggerMode,
        /// Effect sampled with the adjusted local progress.
        effect: Box<VisualEffect>,
    },
    /// A shared user-defined effect; see [`CellEffect`].
    Custom(CustomCellEffect),
}

impl VisualEffect {
    /// Reveal cells from zero to full coverage using deterministic dithering.
    pub fn fade_in() -> Self {
        Self::Fade { from: 0.0, to: 1.0 }
    }

    /// Hide cells from full to zero coverage using deterministic dithering.
    pub fn fade_out() -> Self {
        Self::Fade { from: 1.0, to: 0.0 }
    }

    /// Create a foreground gradient with zero phase offset.
    pub fn gradient(start: Color, end: Color, direction: GradientDirection) -> Self {
        Self::Gradient {
            start,
            end,
            direction,
            target: GradientTarget::Foreground,
            phase: 0.0,
        }
    }

    /// Create a background gradient with zero phase offset.
    pub fn background_gradient(start: Color, end: Color, direction: GradientDirection) -> Self {
        Self::Gradient {
            start,
            end,
            direction,
            target: GradientTarget::Background,
            phase: 0.0,
        }
    }

    /// Create a fading shatter with spread scales of 18 horizontally and 7 vertically.
    pub fn shatter() -> Self {
        Self::Shatter {
            seed: 0x5EED,
            spread_x: 18.0,
            spread_y: 7.0,
            fade: true,
        }
    }

    /// Expand from `anchor` with an initial squeeze of `0.08`.
    pub fn magic_lamp(anchor: VisualAnchor) -> Self {
        Self::MagicLamp {
            anchor,
            squeeze: 0.08,
        }
    }

    /// Hide cells along `direction` with a sharp boundary.
    pub fn wipe(direction: WipeDirection) -> Self {
        Self::Wipe {
            direction,
            mode: WipeMode::Hide,
            softness: 0.0,
        }
    }

    /// Reveal cells along `direction` with a sharp boundary.
    pub fn reveal(direction: WipeDirection) -> Self {
        Self::Wipe {
            direction,
            mode: WipeMode::Reveal,
            softness: 0.0,
        }
    }

    /// Reveal cells in a repeatable random order.
    pub fn dissolve() -> Self {
        Self::Dissolve {
            seed: 0xD155_01F3,
            mode: DissolveMode::In,
        }
    }

    /// Hide cells in a repeatable random order.
    pub fn dissolve_out() -> Self {
        Self::Dissolve {
            seed: 0xD155_01F3,
            mode: DissolveMode::Out,
        }
    }

    /// Create a wave with amplitude `2.0`, wavelength `6.0`, and zero phase.
    pub fn wave(axis: WaveAxis) -> Self {
        Self::Wave {
            axis,
            amplitude: 2.0,
            wavelength: 6.0,
            phase: 0.0,
        }
    }

    /// Create a deterministic glitch with intensity `0.42`.
    pub fn glitch() -> Self {
        Self::Glitch {
            seed: 0x6_117C_4,
            intensity: 0.42,
        }
    }

    /// Create scanlines with density `0.5`, intensity `0.35`, and zero phase.
    pub fn scanline() -> Self {
        Self::Scanline {
            density: 0.5,
            intensity: 0.35,
            phase: 0.0,
        }
    }

    /// Reveal individual source cells in row-major order, without a cursor.
    pub fn typewriter() -> Self {
        Self::Typewriter {
            mode: TypewriterMode::Chars,
            cursor: false,
        }
    }

    /// Reveal groups of six source cells, without a cursor.
    ///
    /// Groups approximate words without examining text or word boundaries.
    pub fn typewriter_words() -> Self {
        Self::Typewriter {
            mode: TypewriterMode::Words,
            cursor: false,
        }
    }

    /// Create a blur approximation that clears with progress, using radius `2.0`.
    pub fn blur_like() -> Self {
        Self::BlurLike {
            radius: 2.0,
            mode: BlurMode::In,
        }
    }

    /// Create a pale-yellow horizontal highlight with normalized half-width `0.18`.
    pub fn highlight_sweep() -> Self {
        Self::HighlightSweep {
            color: Color::Rgb(255, 255, 180),
            width: 0.18,
            direction: GradientDirection::Horizontal,
        }
    }

    /// Create pale-yellow sparkles with candidate density `0.08`.
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

    /// Wrap a user-defined cell effect with a static diagnostic name.
    pub fn custom_named(name: &'static str, effect: impl CellEffect) -> Self {
        Self::Custom(CustomCellEffect::new(name, effect))
    }

    /// Run child effects one after another with equal duration slices.
    ///
    /// Completed children are sampled at `1.0`; the active child receives its
    /// local progress; pending children are skipped. Sampling stops if a child
    /// hides the cell. An empty sequence leaves the cell unchanged.
    pub fn sequence(effects: Vec<VisualEffect>) -> Self {
        Self::Sequence(effects)
    }

    /// Run child effects over the same progress range.
    ///
    /// Children are applied in list order. Later children see earlier geometry
    /// and color changes and may overwrite visibility set by earlier children.
    /// An empty list leaves the cell unchanged.
    pub fn parallel(effects: Vec<VisualEffect>) -> Self {
        Self::Parallel(effects)
    }

    /// Delay a child effect by local row.
    ///
    /// `delay` is a fraction of total progress per row, clamped to `0.0..=1.0`;
    /// non-finite values become zero. The total offset is capped at `0.95`, and
    /// the child's duration is shortened so every row completes by progress `1.0`.
    pub fn stagger_rows(delay: f64, effect: VisualEffect) -> Self {
        Self::stagger(delay, StaggerMode::Rows, effect)
    }

    /// Delay a child effect by local column.
    ///
    /// Uses the same delay normalization and completion rules as [`Self::stagger_rows`].
    pub fn stagger_cols(delay: f64, effect: VisualEffect) -> Self {
        Self::stagger(delay, StaggerMode::Cols, effect)
    }

    /// Delay a child effect by source cell index, row-major.
    ///
    /// Uses the same delay normalization and completion rules as [`Self::stagger_rows`].
    /// Indices include blank positions in the allocated area, not just text glyphs.
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
    /// Most spatial and glyph-changing effects become fades. Fade, gradient,
    /// and scanline effects remain unchanged; highlight sweeps use a narrower
    /// band. Composition is preserved with zero stagger delay. Custom effects
    /// remain unchanged and must inspect the context's motion policy themselves.
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
    ///
    /// Compositions report the most expensive child, with a minimum of
    /// [`VisualEffectCost::Moderate`] for staggered effects. This is a category,
    /// not a measured duration or a sum of child costs.
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

    /// Set the seed of a shatter, dissolve, glitch, or sparkle effect.
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

    /// Set shatter spread scales, clamping each to at least zero.
    ///
    /// Horizontal displacement also uses the context's cell aspect ratio.
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

    /// Choose the color channel replaced by a gradient.
    pub fn target(mut self, target: GradientTarget) -> Self {
        if let Self::Gradient {
            target: current, ..
        } = &mut self
        {
            *current = target;
        }
        self
    }

    /// Set a gradient, wave, or scanline phase in cycles; non-finite values become zero.
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

    /// Set the magic-lamp residual scale, clamped to `0.0..=1.0`.
    pub fn squeeze(mut self, squeeze: f64) -> Self {
        if let Self::MagicLamp {
            squeeze: current, ..
        } = &mut self
        {
            *current = squeeze.clamp(0.0, 1.0);
        }
        self
    }

    /// Set wipe boundary softness, clamped to `0.0..=1.0`.
    ///
    /// Zero gives a sharp edge; non-finite values become zero.
    pub fn softness(mut self, softness: f64) -> Self {
        if let Self::Wipe {
            softness: current, ..
        } = &mut self
        {
            *current = sanitize_softness(softness);
        }
        self
    }

    /// Set nonnegative wave amplitude; negative or non-finite values become zero.
    pub fn amplitude(mut self, amplitude: f64) -> Self {
        if let Self::Wave {
            amplitude: current, ..
        } = &mut self
        {
            *current = finite_or(amplitude, 0.0).max(0.0);
        }
        self
    }

    /// Set wave wavelength, with a minimum of [`f64::EPSILON`].
    ///
    /// Non-finite values use the default wavelength of `6.0`.
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

    /// Set glitch or scanline intensity, clamped to `0.0..=1.0`.
    /// Non-finite values become zero.
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

    /// Set scanline or sparkle density, clamped to `0.0..=1.0`.
    ///
    /// Non-finite values become zero. Sparkle sampling additionally caps density at `0.35`.
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

    /// Enable or disable the typewriter cursor, using `▌` or an ASCII `|` fallback.
    pub fn cursor(mut self, cursor: bool) -> Self {
        if let Self::Typewriter {
            cursor: current, ..
        } = &mut self
        {
            *current = cursor;
        }
        self
    }

    /// Set the blur approximation's pattern parameter; zero disables it.
    ///
    /// Negative or non-finite values become zero. This does not set a spatial
    /// sampling radius; use [`super::Backdrop::blur_radius`] for backdrop blur.
    pub fn radius(mut self, radius: f64) -> Self {
        if let Self::BlurLike {
            radius: current, ..
        } = &mut self
        {
            *current = finite_or(radius, 0.0).max(0.0);
        }
        self
    }

    /// Choose whether the blur approximation clears or builds with progress.
    pub fn blur_mode(mut self, mode: BlurMode) -> Self {
        if let Self::BlurLike { mode: current, .. } = &mut self {
            *current = mode;
        }
        self
    }

    /// Set highlight half-width in normalized units, clamped to `f64::EPSILON..=1.0`.
    /// Non-finite values use the default of `0.18`.
    pub fn width(mut self, width: f64) -> Self {
        if let Self::HighlightSweep { width: current, .. } = &mut self {
            *current = finite_or(width, 0.18).clamp(f64::EPSILON, 1.0);
        }
        self
    }

    /// Set the foreground color used by a highlight sweep or sparkle effect.
    pub fn color(mut self, color: Color) -> Self {
        match &mut self {
            Self::HighlightSweep { color: current, .. } | Self::Sparkle { color: current, .. } => {
                *current = color;
            }
            _ => {}
        }
        self
    }

    /// Set the direction of a highlight sweep.
    pub fn direction(mut self, direction: GradientDirection) -> Self {
        if let Self::HighlightSweep {
            direction: current, ..
        } = &mut self
        {
            *current = direction;
        }
        self
    }

    /// Whether unchanged source cells may be skipped when no backdrop is active.
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
    /// Delay each successive source row.
    Rows,
    /// Delay each successive source column.
    Cols,
    /// Delay each successive source cell in row-major order, including blank positions.
    Chars,
}

/// Direction used by gradients and highlight sweeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientDirection {
    /// Interpolate from left to right.
    Horizontal,
    /// Interpolate from top to bottom.
    Vertical,
    /// Interpolate from top-left to bottom-right.
    Diagonal,
}

/// Which color channel a gradient should override.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientTarget {
    /// Replace the glyph color while preserving its background.
    Foreground,
    /// Replace the background color while preserving the glyph color.
    Background,
}

/// Direction used by [`VisualEffect::Wipe`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WipeDirection {
    /// Advance from the left edge to the right edge.
    LeftToRight,
    /// Advance from the right edge to the left edge.
    RightToLeft,
    /// Advance from the top edge to the bottom edge.
    TopToBottom,
    /// Advance from the bottom edge to the top edge.
    BottomToTop,
    /// Expand radially from the center, accounting for cell aspect ratio.
    CenterOut,
    /// Advance inward from the nearest rectangular edge.
    EdgesIn,
    /// Advance diagonally from top-left to bottom-right.
    DiagonalDown,
    /// Advance diagonally from bottom-left to top-right.
    DiagonalUp,
}

/// Visibility mode for wipe-style effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WipeMode {
    /// Show cells behind the advancing boundary.
    Reveal,
    /// Hide cells behind the advancing boundary.
    Hide,
}

/// Visibility mode for dissolve effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DissolveMode {
    /// Increase visible coverage as progress increases.
    In,
    /// Decrease visible coverage as progress increases.
    Out,
}

/// Visibility grouping used by [`VisualEffect::Typewriter`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypewriterMode {
    /// Reveal individual source-cell positions in row-major order.
    Chars,
    /// Reveal groups of six source-cell positions without detecting word boundaries.
    Words,
}

/// Direction of terminal-cell blur approximation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlurMode {
    /// Start blurred and clear the approximation as progress increases.
    In,
    /// Start clear and increase the blur approximation with progress.
    Out,
}

/// Axis used by [`VisualEffect::Wave`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveAxis {
    /// Vary the wave by source row and displace cells horizontally.
    Rows,
    /// Vary the wave by aspect-corrected source column and displace cells vertically.
    Cols,
}

/// Anchor point for spatial effects such as magic-lamp minimization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VisualAnchor {
    /// Top-left cell of the area.
    TopLeft,
    /// Midpoint of the top edge.
    Top,
    /// Top-right cell of the area.
    TopRight,
    /// Midpoint of the left edge.
    Left,
    /// Center of the area.
    Center,
    /// Midpoint of the right edge.
    Right,
    /// Bottom-left cell of the area.
    BottomLeft,
    /// Midpoint of the bottom edge.
    Bottom,
    /// Bottom-right cell of the area.
    BottomRight,
    /// Normalized coordinates where (0, 0) is top-left and (1, 1) is bottom-right.
    /// Each component is clamped to `0.0..=1.0` when resolved.
    Relative(f64, f64),
}

impl VisualAnchor {
    /// Resolve the anchor to local cell coordinates, using zero for an empty axis.
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
