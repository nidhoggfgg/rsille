use super::MAX_SIDE;
use std::io::Write;

pub(super) fn index([r, g, b, a]: [u8; 4]) -> u16 {
    if a < 128 {
        256
    } else {
        u16::from((r & 224) | ((g >> 3) & 28) | (b >> 6))
    }
}

/// Write a transparent RGB332 sprite. Palette index 256 marks unpainted pixels.
pub(super) fn draw(col: u16, row: u16, w: usize, rgba: &[[u8; 4]], out: &mut Vec<u8>) {
    let pixels: Vec<_> = rgba.iter().copied().map(index).collect();
    let h = pixels.len() / w;
    let mut palette = [false; 256];
    for &c in &pixels {
        if c != 256 {
            palette[usize::from(c)] = true;
        }
    }
    write!(out, "\x1b[{row};{col}H\x1bP0;1q\"1;1;{w};{h}").unwrap();
    for (c, _) in palette.iter().enumerate().filter(|(_, used)| **used) {
        write!(
            out,
            "#{c};2;{};{};{}",
            (c >> 5) * 100 / 7,
            ((c >> 2) & 7) * 100 / 7,
            (c & 3) * 100 / 3
        )
        .unwrap();
    }
    for y in (0..h).step_by(6) {
        if y != 0 {
            out.push(b'-');
        }
        for (c, _) in palette.iter().enumerate().filter(|(_, used)| **used) {
            let mut band = [0u8; MAX_SIDE];
            for dy in 0..6.min(h - y) {
                for (x, mask) in band[..w].iter_mut().enumerate() {
                    if pixels[(y + dy) * w + x] == c as u16 {
                        *mask |= 1 << dy;
                    }
                }
            }
            if let Some(last) = band[..w].iter().rposition(|v| *v != 0) {
                write!(out, "#{c}").unwrap();
                for run in band[..=last].chunk_by(|a, b| a == b) {
                    let ch = run[0] + 63;
                    if run.len() > 3 {
                        write!(out, "!{}", run.len()).unwrap();
                        out.push(ch);
                    } else {
                        out.resize(out.len() + run.len(), ch);
                    }
                }
                out.push(b'$');
            }
        }
    }
    out.extend_from_slice(b"\x1b\\");
}
