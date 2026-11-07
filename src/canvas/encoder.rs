//! Conversion of sample coverage and colors into styled terminal characters.
//!
//! Half-block cells retain their two sample colors. Braille and Quadrant cells
//! preserve partial coverage using a shared foreground; fully colored cells
//! fit their samples to foreground and background color clusters.

use crossterm::style::{Color, Colors};

use super::color::{average, distance, rgb};
use super::sample::{Sample, Samples};
use super::Encoding;
use crate::render::style::{Style, Stylized};

/// Encodes one cell, using `background` for uncovered samples.
pub(crate) fn encode(encoding: Encoding, samples: &Samples, background: Option<Color>) -> Stylized {
    match encoding {
        Encoding::Braille => braille(samples, background),
        Encoding::HalfBlock => half_block(samples, background),
        Encoding::Quadrant => quadrant(samples, background),
    }
}

/// Builds a styled character, leaving unspecified colors inherited.
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

/// Encodes the bottom and top samples as a space, half block or full block.
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

/// Maps eight Cartesian sample slots to Unicode Braille dot bits.
fn braille(samples: &Samples, background: Option<Color>) -> Stylized {
    // Cartesian order: bottom left/right, then the three rows above them.
    const BITS: [u8; 8] = [0x40, 0x80, 0x04, 0x20, 0x02, 0x10, 0x01, 0x08];
    patterned(
        &samples.0,
        BITS,
        |mask| char::from_u32(0x2800 + u32::from(mask)).unwrap(),
        background,
    )
}

/// Maps four Cartesian sample slots to quadrant-block glyphs.
fn quadrant(samples: &Samples, background: Option<Color>) -> Stylized {
    // Bits 0..4 are bottom left, bottom right, top left, top right.
    const SYMBOLS: [char; 16] = [
        ' ', '▖', '▗', '▄', '▘', '▌', '▞', '▙', '▝', '▚', '▐', '▟', '▀', '▛', '▜', '█',
    ];
    let samples = std::array::from_fn(|i| samples.0[i]);
    patterned(
        &samples,
        [1, 2, 4, 8],
        |mask| SYMBOLS[mask as usize],
        background,
    )
}

/// Preserves partial coverage or fits fully colored samples to two colors.
///
/// `bits` maps sample slots to glyph-mask bits, and `symbol` maps each mask to
/// its character. An empty cell becomes a space. Full cells with explicit
/// colors use up to four clustering iterations, assigning the brighter cluster
/// to the foreground.
fn patterned<const N: usize>(
    samples: &[Option<Sample>; N],
    bits: [u8; N],
    symbol: fn(u8) -> char,
    background: Option<Color>,
) -> Stylized {
    let full_mask = bits.iter().fold(0, |mask, bit| mask | bit);
    let mut mask = 0;
    for (sample, bit) in samples.iter().zip(bits) {
        if sample.is_some() {
            mask |= bit;
        }
    }
    if mask == 0 {
        return cell(' ', None, background);
    }
    // Uncovered samples reserve the background. Preserve the exact geometry and
    // reduce occupied colors to one foreground, independently of drawing order.
    let foreground = average(samples.iter().flatten().filter_map(|s| s.color));
    if mask != full_mask || samples.iter().any(|s| s.unwrap().color.is_none()) {
        return cell(symbol(mask), foreground, background);
    }

    // A fully painted RGB/palette cell can use both terminal colors. Fit two
    // clusters to its samples instead of discarding their color contrast.
    let colors = samples.map(|s| s.unwrap().color.unwrap());
    if colors.iter().all(|&color| color == colors[0]) {
        return cell(symbol(full_mask), foreground, foreground);
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
        return cell(symbol(full_mask), foreground, foreground);
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
    for (color, bit) in colors.into_iter().zip(bits) {
        if distance(color, high) <= distance(color, low) {
            mask |= bit;
        }
    }
    cell(symbol(mask), Some(high), Some(low))
}
