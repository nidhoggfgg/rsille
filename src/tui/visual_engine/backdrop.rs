//! Approximate backdrop composition from already encoded terminal cells.
use std::sync::Arc;

use crossterm::style::{Attribute, Color as TerminalColor, Colors};

use crate::render::{area::Size, buffer::Buffer, style::Stylized};
use crate::tui::style::Color;

use super::{CellSample, VisualCtx};

type Rgb = [f32; 3];
type Mask = Arc<dyn Fn(u16, u16, Size) -> f64 + Send + Sync>;

/// A background sampled from previously painted cells in the current render target.
///
/// This approximates the lower layer's colors, blurs them, and mixes in a tint.
/// Child text, borders and their foreground colors stay sharp. It does not use
/// terminal window transparency or require access to Canvas pixels. Half-blocks
/// and Braille contribute colors according to their coverage; ordinary text is
/// approximated rather than rasterized. Terminal-default colors use theme fallbacks,
/// and indexed colors use the standard xterm palette.
///
/// The default footprint is the wrapper's entire rectangle, including empty cells.
/// Child background colors become the tint unless [`Self::tint`] overrides them.
/// A mask can restrict the footprint without changing layout or hit testing.
///
/// ```
/// use rsille::tui::prelude::*;
/// let widget = visual(label::<()>("Hello"))
///     .backdrop(Backdrop::frosted());
/// ```
#[derive(Clone)]
pub struct Backdrop {
    radius: u16,
    opacity: f64,
    tint: Option<Color>,
    mask: Option<Mask>,
}

impl std::fmt::Debug for Backdrop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Backdrop")
            .field("radius", &self.radius)
            .field("opacity", &self.opacity)
            .field("tint", &self.tint)
            .field("mask", &self.mask.is_some())
            .finish()
    }
}

impl Default for Backdrop {
    fn default() -> Self {
        Self::frosted()
    }
}

impl Backdrop {
    /// Frosted glass with a two-cell blur radius and 45% tint opacity.
    pub fn frosted() -> Self {
        Self {
            radius: 2,
            opacity: 0.45,
            tint: None,
            mask: None,
        }
    }

    /// Translucent tint without blur. Defaults to 45% opacity.
    pub fn tinted(color: Color) -> Self {
        Self::frosted().blur_radius(0).tint(color)
    }

    /// Box-blur radius in terminal cells, limited to 16. Zero disables blur.
    pub fn blur_radius(mut self, radius: u16) -> Self {
        self.radius = radius.min(16);
        self
    }

    /// Tint contribution from 0 (sampled scenery only) to 1 (solid tint).
    ///
    /// This does not fade the child's foreground. Blur is independent of opacity.
    /// Non-finite values use the preset opacity of 0.45.
    pub fn opacity(mut self, opacity: f64) -> Self {
        self.opacity = if opacity.is_finite() {
            opacity.clamp(0.0, 1.0)
        } else {
            0.45
        };
        self
    }

    /// Override child background colors with one tint for the entire backdrop.
    pub fn tint(mut self, color: Color) -> Self {
        self.tint = Some(color);
        self
    }

    /// Set per-cell backdrop coverage, from 0 (unprocessed scenery) to 1 (glass).
    ///
    /// Coordinates are zero-based source-cell positions within the wrapper and
    /// `Size` is its allocated size. Out-of-range values are clamped; non-finite
    /// values mean zero coverage. At zero, empty child cells leave the lower
    /// layer untouched, while text/borders receive an untinted scenery background.
    /// This affects painting only, never layout or pointer hit testing.
    pub fn mask(mut self, mask: impl Fn(u16, u16, Size) -> f64 + Send + Sync + 'static) -> Self {
        self.mask = Some(Arc::new(mask));
        self
    }
}

/// Immutable color snapshot, captured before any cells in the wrapper are written.
/// It includes a blur halo outside the wrapper and is rebuilt on each render so
/// lower-layer changes do not depend on the child's dirty state.
pub(super) struct BackdropSnapshot {
    left: usize,
    top: usize,
    width: usize,
    raw: Vec<Rgb>,
    blurred: Vec<Rgb>,
    // Without blur, keep single-column lower glyphs so tint preserves graphics.
    unblurred_cells: Vec<Option<Stylized>>,
}

impl BackdropSnapshot {
    pub(super) fn capture(buffer: &Buffer, backdrop: &Backdrop, ctx: &VisualCtx<'_>) -> Self {
        let radius = backdrop.radius as usize;
        let area = ctx.area;
        let left = (area.x() as usize).saturating_sub(radius);
        let top = (area.y() as usize).saturating_sub(radius);
        let right =
            (area.x() as usize + area.width() as usize + radius).min(buffer.size().width as usize);
        let bottom = (area.y() as usize + area.height() as usize + radius)
            .min(buffer.size().height as usize);
        let width = right - left;
        let height = bottom - top;
        let (fg, bg) = theme_colors(ctx);
        let mut raw = Vec::with_capacity(width * height);
        let mut unblurred_cells = Vec::new();
        for y in top..bottom {
            for x in left..right {
                let index = y * buffer.size().width as usize + x;
                let cell = &buffer.content()[index];
                let owner = cell.owner.unwrap_or(index);
                raw.push(representative_color(
                    &buffer.content()[owner].content,
                    fg,
                    bg,
                ));
                if radius == 0 {
                    unblurred_cells.push(
                        (!cell.is_occupied && cell.width() == 1).then(|| cell.content.clone()),
                    );
                }
            }
        }
        let blurred = box_blur(&raw, width, height, radius);
        Self {
            left,
            top,
            width,
            raw,
            blurred,
            unblurred_cells,
        }
    }

    pub(super) fn apply(&self, backdrop: &Backdrop, sample: &mut CellSample, ctx: &VisualCtx<'_>) {
        let coverage = backdrop.mask.as_ref().map_or(1.0, |mask| {
            let value = mask(
                sample.source_x as u16,
                sample.source_y as u16,
                ctx.area.size(),
            );
            if value.is_finite() {
                value.clamp(0.0, 1.0)
            } else {
                0.0
            }
        }) as f32;
        if (coverage == 0.0 || (backdrop.radius == 0 && backdrop.opacity == 0.0))
            && (sample.content.is_empty() || sample.content.as_char() == Some(' '))
        {
            sample.visible = false;
            return;
        }
        let x = ctx.area.x() as usize + sample.dest_x.round() as usize;
        let y = ctx.area.y() as usize + sample.dest_y.round() as usize;
        let index = (y - self.top) * self.width + x - self.left;
        let raw = self.raw[index];
        let blurred = self.blurred[index];
        let mut colors = sample.content.style.colors.unwrap_or(Colors {
            foreground: None,
            background: None,
        });
        let tint = backdrop.tint.map(Into::into).or(colors.background);
        let tint = rgb(tint, theme_colors(ctx).1);
        let opacity = backdrop.opacity as f32;
        if sample.content.as_char() == Some(' ') {
            if let Some(Some(original)) = self.unblurred_cells.get(index) {
                let (fg, bg) = theme_colors(ctx);
                sample.content = original.clone();
                let colors = original.style.colors;
                sample.content.style.colors = Some(Colors {
                    foreground: Some(mixed_color(
                        rgb(colors.and_then(|c| c.foreground), fg),
                        tint,
                        opacity * coverage,
                        ctx,
                    )),
                    background: Some(mixed_color(
                        rgb(colors.and_then(|c| c.background), bg),
                        tint,
                        opacity * coverage,
                        ctx,
                    )),
                });
                return;
            }
        }
        let [r, g, b] = std::array::from_fn(|c| {
            let glass = blurred[c] * (1.0 - opacity) + tint[c] * opacity;
            (raw[c] * (1.0 - coverage) + glass * coverage)
                .round()
                .clamp(0.0, 255.0) as u8
        });
        colors.background = Some(super::color::capable_color(Color::Rgb(r, g, b), ctx));
        sample.content.style.colors = Some(colors);
    }
}

fn mixed_color(start: Rgb, tint: Rgb, amount: f32, ctx: &VisualCtx<'_>) -> TerminalColor {
    let [r, g, b] = std::array::from_fn(|c| {
        (start[c] * (1.0 - amount) + tint[c] * amount)
            .round()
            .clamp(0.0, 255.0) as u8
    });
    super::color::capable_color(Color::Rgb(r, g, b), ctx)
}

fn theme_colors(ctx: &VisualCtx<'_>) -> (Rgb, Rgb) {
    let fg = rgb(ctx.theme.styles.text.fg_color.map(Into::into), [255.0; 3]);
    let bg = rgb(ctx.theme.styles.surface.bg_color.map(Into::into), [0.0; 3]);
    (fg, bg)
}

fn representative_color(cell: &Stylized, fg: Rgb, bg: Rgb) -> Rgb {
    let colors = cell.style.colors;
    let mut fg = rgb(colors.and_then(|c| c.foreground), fg);
    let mut bg = rgb(colors.and_then(|c| c.background), bg);
    if cell
        .style
        .attr
        .is_some_and(|attr| attr.has(Attribute::Reverse))
    {
        std::mem::swap(&mut fg, &mut bg);
    }
    let coverage = match cell.as_char() {
        Some(' ' | '\u{2800}') | None => 0.0,
        Some('█') => 1.0,
        Some('▀' | '▄' | '▌' | '▐' | '▒') => 0.5,
        Some('░') => 0.25,
        Some('▓') => 0.75,
        Some(ch @ '\u{2801}'..='\u{28ff}') => (ch as u32 - 0x2800).count_ones() as f32 / 8.0,
        _ => 0.25,
    };
    std::array::from_fn(|c| fg[c] * coverage + bg[c] * (1.0 - coverage))
}

fn rgb(color: Option<TerminalColor>, fallback: Rgb) -> Rgb {
    use TerminalColor::*;
    let index = match color {
        Some(Rgb { r, g, b }) => return [r as f32, g as f32, b as f32],
        Some(AnsiValue(index)) => index,
        Some(Black) => 0,
        Some(DarkRed) => 1,
        Some(DarkGreen) => 2,
        Some(DarkYellow) => 3,
        Some(DarkBlue) => 4,
        Some(DarkMagenta) => 5,
        Some(DarkCyan) => 6,
        Some(Grey) => 7,
        Some(DarkGrey) => 8,
        Some(Red) => 9,
        Some(Green) => 10,
        Some(Yellow) => 11,
        Some(Blue) => 12,
        Some(Magenta) => 13,
        Some(Cyan) => 14,
        Some(White) => 15,
        Some(Reset) | None => return fallback,
    };
    const BASIC: [[u8; 3]; 16] = [
        [0, 0, 0],
        [128, 0, 0],
        [0, 128, 0],
        [128, 128, 0],
        [0, 0, 128],
        [128, 0, 128],
        [0, 128, 128],
        [192, 192, 192],
        [128, 128, 128],
        [255, 0, 0],
        [0, 255, 0],
        [255, 255, 0],
        [0, 0, 255],
        [255, 0, 255],
        [0, 255, 255],
        [255, 255, 255],
    ];
    match index {
        0..=15 => BASIC[index as usize].map(f32::from),
        16..=231 => {
            let i = index - 16;
            let levels = [0.0, 95.0, 135.0, 175.0, 215.0, 255.0];
            [
                levels[(i / 36) as usize],
                levels[(i / 6 % 6) as usize],
                levels[(i % 6) as usize],
            ]
        }
        _ => [8.0 + 10.0 * (index - 232) as f32; 3],
    }
}

/// Summed-area table makes the blur linear in the captured region's size,
/// independent of radius. Clipped edge windows are normalized by their actual area.
fn box_blur(pixels: &[Rgb], width: usize, height: usize, radius: usize) -> Vec<Rgb> {
    if radius == 0 {
        return pixels.to_vec();
    }
    let stride = width + 1;
    let mut sums = vec![[0.0; 3]; stride * (height + 1)];
    for y in 0..height {
        let mut row = [0.0; 3];
        for x in 0..width {
            for c in 0..3 {
                row[c] += pixels[y * width + x][c];
                sums[(y + 1) * stride + x + 1][c] = sums[y * stride + x + 1][c] + row[c];
            }
        }
    }
    let mut output = vec![[0.0; 3]; pixels.len()];
    for y in 0..height {
        let top = y.saturating_sub(radius);
        let bottom = (y + radius + 1).min(height);
        for x in 0..width {
            let left = x.saturating_sub(radius);
            let right = (x + radius + 1).min(width);
            let count = ((right - left) * (bottom - top)) as f32;
            output[y * width + x] = std::array::from_fn(|c| {
                (sums[bottom * stride + right][c]
                    - sums[top * stride + right][c]
                    - sums[bottom * stride + left][c]
                    + sums[top * stride + left][c])
                    / count
            });
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::HashMap};

    use crate::render::{area::Area, chunk::Chunk, style::Style as RenderStyle};
    use crate::tui::{
        animation::AnimationStore,
        style::{Style, Theme},
        widget::{RenderCtx, Widget, WidgetPath, WidgetStore},
        widgets::{label, spacer, visual},
    };

    use super::*;

    fn paint(buffer: &mut Buffer, x: u16, y: u16, ch: char, fg: [u8; 3], bg: [u8; 3]) {
        let color = |[r, g, b]: [u8; 3]| TerminalColor::Rgb { r, g, b };
        buffer
            .overwrite(
                (x, y).into(),
                Stylized::new(
                    ch,
                    RenderStyle::with_colors(Colors {
                        foreground: Some(color(fg)),
                        background: Some(color(bg)),
                    }),
                ),
            )
            .unwrap();
    }

    fn render(buffer: &mut Buffer, area: Area, widget: &impl Widget<()>) {
        let mut chunk = Chunk::new(buffer, area).unwrap();
        let store = WidgetStore::new();
        let animations = AnimationStore::new();
        let theme = Theme::dark();
        let geometry = RefCell::new(HashMap::<WidgetPath, Area>::new());
        let ctx = RenderCtx::new(&store, &animations, &theme, None, &geometry);
        widget.render(&mut chunk, &ctx);
    }

    fn background(buffer: &Buffer, x: u16, y: u16) -> TerminalColor {
        buffer
            .get((x, y).into())
            .unwrap()
            .style
            .colors
            .unwrap()
            .background
            .unwrap()
    }

    #[test]
    fn tint_covers_empty_cells_and_preserves_text_style_and_outside_cells() {
        let mut buffer = Buffer::new((5, 1).into());
        for x in 0..5 {
            paint(&mut buffer, x, 0, ' ', [80; 3], [80; 3]);
        }
        let untouched = buffer.get((0, 0).into()).unwrap().clone();
        let widget =
            visual(label::<()>("A").style(Style::default().fg(Color::Rgb(240, 200, 160)).bold()))
                .backdrop(Backdrop::tinted(Color::Rgb(20, 20, 20)).opacity(0.5));
        render(
            &mut buffer,
            Area::new((1, 0).into(), (3, 1).into()),
            &widget,
        );
        for x in 1..4 {
            assert_eq!(
                background(&buffer, x, 0),
                TerminalColor::Rgb {
                    r: 50,
                    g: 50,
                    b: 50
                }
            );
        }
        let text = buffer.get((1, 0).into()).unwrap();
        assert_eq!(text.as_char(), Some('A'));
        assert_eq!(
            text.style.colors.unwrap().foreground,
            Some(TerminalColor::Rgb {
                r: 240,
                g: 200,
                b: 160
            })
        );
        assert!(text.style.attr.unwrap().has(Attribute::Bold));
        assert_eq!(buffer.get((0, 0).into()), Some(&untouched));
        assert_eq!(buffer.get((4, 0).into()), Some(&untouched));
    }

    #[test]
    fn unblurred_tint_preserves_graphic_glyphs_and_zero_opacity_is_a_noop() {
        let mut buffer = Buffer::new((3, 1).into());
        for (x, ch) in ['▀', '\u{280f}', 'A'].into_iter().enumerate() {
            paint(&mut buffer, x as u16, 0, ch, [200, 100, 80], [0, 20, 40]);
        }
        let area = Area::new((0, 0).into(), (3, 1).into());
        let original = buffer.content().to_vec();
        let transparent =
            visual(spacer::<()>()).backdrop(Backdrop::tinted(Color::Black).opacity(0.0));
        render(&mut buffer, area, &transparent);
        assert_eq!(buffer.content(), original);
        let tint =
            visual(spacer::<()>()).backdrop(Backdrop::tinted(Color::Rgb(0, 0, 0)).opacity(0.5));
        render(&mut buffer, area, &tint);
        for (x, ch) in ['▀', '\u{280f}', 'A'].into_iter().enumerate() {
            let cell = buffer.get((x as u16, 0).into()).unwrap();
            assert_eq!(cell.as_char(), Some(ch));
            assert_eq!(
                cell.style.colors.unwrap(),
                Colors {
                    foreground: Some(TerminalColor::Rgb {
                        r: 100,
                        g: 50,
                        b: 40
                    }),
                    background: Some(TerminalColor::Rgb { r: 0, g: 10, b: 20 }),
                }
            );
        }
    }

    #[test]
    fn blur_reads_the_halo_and_uses_one_snapshot_before_any_writes() {
        let mut buffer = Buffer::new((5, 3).into());
        for y in 0..3 {
            for x in 0..5 {
                paint(&mut buffer, x, y, ' ', [0; 3], [0; 3]);
            }
        }
        paint(&mut buffer, 2, 1, ' ', [255; 3], [255; 3]);
        let widget =
            visual(spacer::<()>()).backdrop(Backdrop::frosted().blur_radius(1).opacity(0.0));
        render(
            &mut buffer,
            Area::new((1, 1).into(), (3, 1).into()),
            &widget,
        );
        for x in 1..4 {
            assert_eq!(
                background(&buffer, x, 1),
                TerminalColor::Rgb {
                    r: 28,
                    g: 28,
                    b: 28
                }
            );
        }
        assert_eq!(
            background(&buffer, 0, 1),
            TerminalColor::Rgb { r: 0, g: 0, b: 0 }
        );
    }

    #[test]
    fn mask_preserves_cutouts_and_draws_border_over_untinted_scenery() {
        let mut buffer = Buffer::new((3, 1).into());
        for x in 0..3 {
            paint(&mut buffer, x, 0, '▀', [200, 0, 0], [0, 0, 100]);
        }
        let corner = buffer.get((0, 0).into()).unwrap().clone();
        let widget = visual(label::<()>(" ╱ ").style(Style::default().fg(Color::White))).backdrop(
            Backdrop::tinted(Color::Rgb(10, 20, 30))
                .opacity(1.0)
                .mask(|x, _, _| if x < 2 { 0.0 } else { 1.0 }),
        );
        render(
            &mut buffer,
            Area::new((0, 0).into(), (3, 1).into()),
            &widget,
        );
        assert_eq!(buffer.get((0, 0).into()), Some(&corner));
        assert_eq!(buffer.get((1, 0).into()).unwrap().as_char(), Some('╱'));
        assert_eq!(
            background(&buffer, 1, 0),
            TerminalColor::Rgb {
                r: 100,
                g: 0,
                b: 50
            }
        );
        assert_eq!(
            background(&buffer, 2, 0),
            TerminalColor::Rgb {
                r: 10,
                g: 20,
                b: 30
            }
        );
    }

    #[test]
    fn unchanged_children_follow_lower_layer_changes_with_other_visual_effects() {
        let widget = visual(label::<()>("A"))
            .progress(1.0)
            .fade_in()
            .backdrop(Backdrop::tinted(Color::Rgb(0, 0, 0)).opacity(0.5));
        let mut buffer = Buffer::new((1, 1).into());
        for value in [80, 160] {
            paint(&mut buffer, 0, 0, ' ', [value; 3], [value; 3]);
            render(
                &mut buffer,
                Area::new((0, 0).into(), (1, 1).into()),
                &widget,
            );
            let v = value / 2;
            assert_eq!(
                background(&buffer, 0, 0),
                TerminalColor::Rgb { r: v, g: v, b: v }
            );
        }
        let hidden = visual(label::<()>("A"))
            .progress(0.0)
            .fade_in()
            .backdrop(Backdrop::frosted());
        let original = buffer.get((0, 0).into()).unwrap().clone();
        render(
            &mut buffer,
            Area::new((0, 0).into(), (1, 1).into()),
            &hidden,
        );
        assert_eq!(buffer.get((0, 0).into()), Some(&original));
    }

    #[test]
    fn stacked_backdrops_sample_the_previously_composited_layer() {
        let mut buffer = Buffer::new((1, 1).into());
        paint(&mut buffer, 0, 0, ' ', [200; 3], [200; 3]);
        let widget =
            visual(spacer::<()>()).backdrop(Backdrop::tinted(Color::Rgb(0, 0, 0)).opacity(0.5));
        let area = Area::new((0, 0).into(), (1, 1).into());
        render(&mut buffer, area, &widget);
        assert_eq!(
            background(&buffer, 0, 0),
            TerminalColor::Rgb {
                r: 100,
                g: 100,
                b: 100
            }
        );
        render(&mut buffer, area, &widget);
        assert_eq!(
            background(&buffer, 0, 0),
            TerminalColor::Rgb {
                r: 50,
                g: 50,
                b: 50
            }
        );
    }

    #[test]
    fn encoded_graphics_palette_and_default_colors_are_sampled() {
        let make = |ch| {
            Stylized::new(
                ch,
                RenderStyle::with_colors(Colors {
                    foreground: Some(TerminalColor::Rgb {
                        r: 200,
                        g: 100,
                        b: 80,
                    }),
                    background: Some(TerminalColor::Rgb { r: 0, g: 20, b: 40 }),
                }),
            )
        };
        for ch in ['▀', '▄', '\u{280f}'] {
            assert_eq!(
                representative_color(&make(ch), [255.0; 3], [0.0; 3]),
                [100.0, 60.0, 60.0]
            );
        }
        assert_eq!(
            representative_color(&make('█'), [0.0; 3], [0.0; 3]),
            [200.0, 100.0, 80.0]
        );
        assert_eq!(
            rgb(Some(TerminalColor::AnsiValue(196)), [0.0; 3]),
            [255.0, 0.0, 0.0]
        );
        assert_eq!(
            rgb(Some(TerminalColor::AnsiValue(244)), [0.0; 3]),
            [128.0; 3]
        );
        assert_eq!(rgb(Some(TerminalColor::Reset), [12.0; 3]), [12.0; 3]);
        let reversed = make(' ').with_style(RenderStyle::with_both(
            make(' ').style.colors.unwrap(),
            Attribute::Reverse.into(),
        ));
        assert_eq!(
            representative_color(&reversed, [0.0; 3], [0.0; 3]),
            [200.0, 100.0, 80.0]
        );
    }

    #[test]
    fn blur_preserves_constants_and_normalizes_at_terminal_edges() {
        assert_eq!(box_blur(&[[12.0; 3]; 6], 3, 2, 16), vec![[12.0; 3]; 6]);
        let output = box_blur(&[[0.0; 3], [180.0; 3], [0.0; 3]], 3, 1, 1);
        assert_eq!(output, vec![[90.0; 3], [60.0; 3], [90.0; 3]]);
    }

    #[test]
    fn wide_text_empty_frames_and_extreme_settings_remain_valid() {
        let widget = visual(label::<()>("中A")).backdrop(
            Backdrop::frosted()
                .blur_radius(u16::MAX)
                .opacity(f64::NAN)
                .mask(|_, _, _| f64::INFINITY),
        );
        let mut buffer = Buffer::new((3, 1).into());
        render(
            &mut buffer,
            Area::new((0, 0).into(), (3, 1).into()),
            &widget,
        );
        assert_eq!(buffer.get((0, 0).into()).unwrap().as_char(), Some('中'));
        assert!(buffer.get((1, 0).into()).is_none());
        assert_eq!(buffer.get((2, 0).into()).unwrap().as_char(), Some('A'));
        let mut empty = Buffer::new((0, 0).into());
        render(&mut empty, Area::new((0, 0).into(), (0, 0).into()), &widget);
    }
}
