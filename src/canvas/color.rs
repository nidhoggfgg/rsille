use crossterm::style::Color;

// Palette colors depend on the user's terminal theme. Preserve them exactly
// when possible; use the conventional xterm palette only when mixing is needed.
pub(crate) fn rgb(color: Color) -> Option<[u8; 3]> {
    let index = match color {
        Color::Reset => return None,
        Color::Rgb { r, g, b } => return Some([r, g, b]),
        Color::Black => 0,
        Color::DarkRed => 1,
        Color::DarkGreen => 2,
        Color::DarkYellow => 3,
        Color::DarkBlue => 4,
        Color::DarkMagenta => 5,
        Color::DarkCyan => 6,
        Color::Grey => 7,
        Color::DarkGrey => 8,
        Color::Red => 9,
        Color::Green => 10,
        Color::Yellow => 11,
        Color::Blue => 12,
        Color::Magenta => 13,
        Color::Cyan => 14,
        Color::White => 15,
        Color::AnsiValue(index) => index,
    };
    Some(match index {
        0..=15 => [
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
        ][index as usize],
        16..=231 => {
            let n = index - 16;
            let level = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
            [level(n / 36), level(n / 6 % 6), level(n % 6)]
        }
        232..=255 => [8 + (index - 232) * 10; 3],
    })
}

pub(crate) fn distance(a: Color, b: Color) -> u32 {
    match (rgb(a), rgb(b)) {
        (Some(a), Some(b)) => a
            .into_iter()
            .zip(b)
            .map(|(a, b)| (i32::from(a) - i32::from(b)).pow(2) as u32)
            .sum(),
        _ => u32::from(a != b),
    }
}

pub(crate) fn average(colors: impl Iterator<Item = Color> + Clone) -> Option<Color> {
    let first = colors.clone().next()?;
    if colors.clone().all(|color| color == first) {
        return Some(first);
    }
    let mut sum = [0u32; 3];
    let mut count = 0;
    for color in colors {
        // A terminal default has no known RGB value. Use a stable representative
        // rather than guessing the user's foreground color.
        let Some(channels) = rgb(color) else {
            return Some(first);
        };
        for (sum, channel) in sum.iter_mut().zip(channels) {
            *sum += u32::from(channel);
        }
        count += 1;
    }
    let [r, g, b] = sum.map(|sum| ((sum + count / 2) / count) as u8);
    Some(Color::Rgb { r, g, b })
}
