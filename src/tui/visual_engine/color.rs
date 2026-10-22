use crate::render::style::Stylized;
use crossterm::style::{Attribute, Attributes, Color as CrosstermColor, Colors};

use crate::tui::style::{Color, Style};
use crate::tui::visual_engine::{CellSample, GradientTarget, VisualCtx};

pub(crate) fn apply_color_capable(
    content: &mut Stylized,
    color: Color,
    target: GradientTarget,
    ctx: &VisualCtx<'_>,
) {
    apply_crossterm_color(content, capable_color(color, ctx), target);
}

fn apply_crossterm_color(content: &mut Stylized, color: CrosstermColor, target: GradientTarget) {
    let mut colors = content.style.colors.unwrap_or(Colors {
        foreground: None,
        background: None,
    });

    match target {
        GradientTarget::Foreground => colors.foreground = Some(color),
        GradientTarget::Background => colors.background = Some(color),
    }
    content.style.colors = Some(colors);
}

fn current_foreground(content: &Stylized) -> Option<CrosstermColor> {
    content.style.colors.and_then(|colors| colors.foreground)
}

pub(crate) fn add_attribute(content: &mut Stylized, attr: Attribute) {
    let mut attrs = content.style.attr.unwrap_or_else(Attributes::default);
    attrs = attrs | attr;
    content.style.attr = Some(attrs);
}

pub(crate) fn dim_foreground(sample: &mut CellSample, ctx: &VisualCtx<'_>, amount: f64) {
    let amount = amount.clamp(0.0, 1.0);
    if let Some(color) = current_foreground(&sample.content) {
        apply_crossterm_color(
            &mut sample.content,
            dim_crossterm_color(color, amount),
            GradientTarget::Foreground,
        );
    } else {
        let fallback = ctx.theme.styles.text_muted.fg_color.unwrap_or(Color::White);
        apply_color_capable(
            &mut sample.content,
            fallback,
            GradientTarget::Foreground,
            ctx,
        );
    }
    if amount > 0.25 {
        add_attribute(&mut sample.content, Attribute::Dim);
    }
}

pub(crate) fn blend_foreground(
    sample: &mut CellSample,
    ctx: &VisualCtx<'_>,
    target: Color,
    amount: f64,
) {
    let target_color = capable_color(target, ctx);
    let blended = current_foreground(&sample.content)
        .map(|start| blend_crossterm_color(start, target_color, amount))
        .unwrap_or(target_color);
    apply_crossterm_color(&mut sample.content, blended, GradientTarget::Foreground);
}

fn capable_color(color: Color, ctx: &VisualCtx<'_>) -> CrosstermColor {
    if ctx.capabilities.truecolor {
        to_crossterm_color(color)
    } else {
        to_crossterm_color(to_basic_color(color))
    }
}

fn to_basic_color(color: Color) -> Color {
    let (r, g, b) = match color {
        Color::Black => return Color::Black,
        Color::Red => return Color::Red,
        Color::Green => return Color::Green,
        Color::Yellow => return Color::Yellow,
        Color::Blue => return Color::Blue,
        Color::Magenta => return Color::Magenta,
        Color::Cyan => return Color::Cyan,
        Color::White => return Color::White,
        Color::Indexed(index) => {
            return match index % 8 {
                0 => Color::Black,
                1 => Color::Red,
                2 => Color::Green,
                3 => Color::Yellow,
                4 => Color::Blue,
                5 => Color::Magenta,
                6 => Color::Cyan,
                _ => Color::White,
            };
        }
        Color::Rgb(r, g, b) => (r, g, b),
    };

    let candidates = [
        (Color::Black, (0u8, 0u8, 0u8)),
        (Color::Red, (205, 49, 49)),
        (Color::Green, (13, 188, 121)),
        (Color::Yellow, (229, 229, 16)),
        (Color::Blue, (36, 114, 200)),
        (Color::Magenta, (188, 63, 188)),
        (Color::Cyan, (17, 168, 205)),
        (Color::White, (229, 229, 229)),
    ];
    candidates
        .into_iter()
        .min_by_key(|(_, (cr, cg, cb))| {
            let dr = *cr as i32 - r as i32;
            let dg = *cg as i32 - g as i32;
            let db = *cb as i32 - b as i32;
            dr * dr + dg * dg + db * db
        })
        .map(|(color, _)| color)
        .unwrap_or(Color::White)
}

fn dim_crossterm_color(color: CrosstermColor, amount: f64) -> CrosstermColor {
    match color {
        CrosstermColor::Rgb { r, g, b } => {
            let scale = (1.0 - amount).clamp(0.0, 1.0);
            CrosstermColor::Rgb {
                r: (r as f64 * scale).round() as u8,
                g: (g as f64 * scale).round() as u8,
                b: (b as f64 * scale).round() as u8,
            }
        }
        color => color,
    }
}

fn blend_crossterm_color(
    start: CrosstermColor,
    target: CrosstermColor,
    amount: f64,
) -> CrosstermColor {
    let amount = amount.clamp(0.0, 1.0);
    match (start, target) {
        (
            CrosstermColor::Rgb {
                r: sr,
                g: sg,
                b: sb,
            },
            CrosstermColor::Rgb {
                r: tr,
                g: tg,
                b: tb,
            },
        ) => CrosstermColor::Rgb {
            r: lerp_u8_local(sr, tr, amount),
            g: lerp_u8_local(sg, tg, amount),
            b: lerp_u8_local(sb, tb, amount),
        },
        (_, target) => {
            if amount < 0.5 {
                start
            } else {
                target
            }
        }
    }
}

fn lerp_u8_local(start: u8, end: u8, progress: f64) -> u8 {
    (start as f64 + (end as f64 - start as f64) * progress)
        .round()
        .clamp(0.0, 255.0) as u8
}

fn to_crossterm_color(color: Color) -> CrosstermColor {
    let render_style = Style::default().fg(color).to_render_style();
    render_style
        .colors
        .and_then(|colors| colors.foreground)
        .unwrap_or(CrosstermColor::Reset)
}
