use crossterm::style::{Color, Colors};

use super::color::{average, distance, rgb};
use super::sample::Samples;
use super::Encoding;
use crate::render::style::{Style, Stylized};

pub(crate) fn encode(encoding: Encoding, samples: &Samples, background: Option<Color>) -> Stylized {
    match encoding {
        Encoding::Braille => braille(samples, background),
        Encoding::HalfBlock => half_block(samples, background),
    }
}

fn cell(symbol: char, foreground: Option<Color>, background: Option<Color>) -> Stylized {
    let style = if foreground.is_some() || background.is_some() {
        Style::with_colors(Colors {
            foreground,
            background,
        })
    } else {
        Style::default()
    };
    Stylized::new(symbol, style)
}

fn half_block(samples: &Samples, background: Option<Color>) -> Stylized {
    match (samples.0[0], samples.0[1]) {
        (None, None) => cell(' ', None, background),
        (Some(bottom), None) => cell('▄', bottom.color, background),
        (None, Some(top)) => cell('▀', top.color, background),
        (Some(bottom), Some(top)) if bottom.color == top.color => cell('█', top.color, background),
        (Some(bottom), Some(top)) if bottom.color.is_none() => cell('▄', None, top.color),
        (Some(bottom), Some(top)) => cell('▀', top.color, bottom.color),
    }
}

fn braille(samples: &Samples, background: Option<Color>) -> Stylized {
    // Cartesian order: bottom left/right, then the three rows above them.
    const BITS: [u8; 8] = [0x40, 0x80, 0x04, 0x20, 0x02, 0x10, 0x01, 0x08];
    let mut mask = 0;
    for (sample, bit) in samples.0.iter().zip(BITS) {
        if sample.is_some() {
            mask |= bit;
        }
    }
    if mask == 0 {
        return cell(' ', None, background);
    }
    // Uncovered samples reserve the background. Preserve the exact geometry and
    // reduce occupied colors to one foreground, independently of drawing order.
    let foreground = average(samples.0.iter().flatten().filter_map(|s| s.color));
    if mask != 255 || samples.0.iter().any(|s| s.unwrap().color.is_none()) {
        return cell(
            char::from_u32(0x2800 + u32::from(mask)).unwrap(),
            foreground,
            background,
        );
    }

    // A fully painted RGB/palette cell can use both terminal colors. Fit two
    // clusters to its eight samples instead of discarding their color contrast.
    let colors = samples.0.map(|s| s.unwrap().color.unwrap());
    if colors.iter().all(|&color| color == colors[0]) {
        return cell('\u{28ff}', foreground, foreground);
    }
    let mut low = colors[0];
    let mut high = colors[0];
    let mut farthest = 0;
    for (index, a) in colors.into_iter().enumerate() {
        for &b in &colors[index + 1..] {
            let d = distance(a, b);
            if d > farthest {
                (low, high, farthest) = (a, b, d);
            }
        }
    }
    if farthest == 0 {
        return cell('\u{28ff}', foreground, foreground);
    }
    for _ in 0..4 {
        let bright = colors.map(|c| distance(c, high) <= distance(c, low));
        let next_low = average(
            colors
                .iter()
                .zip(bright)
                .filter_map(|(&c, bright)| (!bright).then_some(c)),
        )
        .unwrap_or(low);
        let next_high = average(
            colors
                .iter()
                .zip(bright)
                .filter_map(|(&c, bright)| bright.then_some(c)),
        )
        .unwrap_or(high);
        if next_low == low && next_high == high {
            break;
        }
        (low, high) = (next_low, next_high);
    }
    let luminance = |c| {
        rgb(c).map_or(0, |[r, g, b]| {
            u32::from(r) * 2126 + u32::from(g) * 7152 + u32::from(b) * 722
        })
    };
    if luminance(low) > luminance(high) {
        std::mem::swap(&mut low, &mut high);
    }
    mask = 0;
    for (color, bit) in colors.into_iter().zip(BITS) {
        if distance(color, high) <= distance(color, low) {
            mask |= bit;
        }
    }
    cell(
        char::from_u32(0x2800 + u32::from(mask)).unwrap(),
        Some(high),
        Some(low),
    )
}
