use crossterm::style::Color;
use std::f64::consts::{PI, TAU};

pub const PERIOD: f64 = 8.0;
pub const BACKGROUND: [u8; 3] = [30, 30, 36];
const WIDTH: usize = 40;
const HEIGHT: usize = 48;
// Fixed-size types catch truncated or incorrectly packed assets at compile time.
const SPRITES: [&[u8; WIDTH * HEIGHT * 4]; 3] = [
    include_bytes!("../assets/deepseek.rgba"),
    include_bytes!("../assets/claude.rgba"),
    include_bytes!("../assets/chatgpt.rgba"),
];
const NAMES: [&str; 3] = ["DEEPSEEK", "CLAUDE", "CHATGPT"];
const ACCENTS: [[u8; 3]; 3] = [[135, 163, 255], [255, 186, 110], [216, 199, 233]];
// Eye rectangles in the packed sprite coordinate system (x, y, width, height).
const EYES: [[(usize, usize, usize, usize); 2]; 3] = [
    [(14, 20, 3, 4), (22, 20, 4, 4)],
    [(13, 17, 4, 4), (22, 17, 4, 4)],
    [(14, 20, 3, 4), (22, 20, 4, 4)],
];
const SKIN: [[u8; 4]; 3] = [
    [254, 228, 208, 255],
    [254, 228, 208, 255],
    [254, 228, 208, 255],
];

pub fn color([r, g, b]: [u8; 3]) -> Color {
    Color::Rgb { r, g, b }
}

fn sample(id: usize, x: usize, y: usize, blink: bool) -> [u8; 4] {
    if blink {
        for (ex, ey, ew, eh) in EYES[id] {
            if (ex..ex + ew).contains(&x) && (ey..ey + eh).contains(&y) {
                return if y == ey + eh / 2 {
                    [49, 31, 29, 255]
                } else {
                    SKIN[id]
                };
            }
        }
    }
    let offset = (y * WIDTH + x) * 4;
    SPRITES[id][offset..offset + 4].try_into().unwrap()
}

pub fn render(w: usize, h: usize, time: f64, focus: Option<usize>) -> Vec<[u8; 3]> {
    let mut pixels = vec![BACKGROUND; w * h];
    if w == 0 || h < 10 {
        return pixels;
    }
    let count = if focus.is_some() { 1 } else { 3 };
    let column = w as f64 / count as f64;
    let scale = ((column - 3.0) / WIDTH as f64)
        // Leave headroom for the upward idle motion even in focused mode.
        .min((h as f64 - 12.0) / (HEIGHT as f64 + 4.0))
        .clamp(0.0, 2.0);
    if scale <= 0.0 {
        return pixels;
    }
    // Size and source sampling depend only on the viewport. Changing height
    // during a breath duplicates/drops pixel rows, visibly stretching faces.
    let dw = (WIDTH as f64 * scale).round().max(1.0) as usize;
    let dh = (HEIGHT as f64 * scale).round().max(1.0) as usize;
    for slot in 0..count {
        let id = focus.unwrap_or(slot);
        let phase = (time + id as f64 * 2.0).rem_euclid(PERIOD);
        let blink = (2.7..2.88).contains(&phase) || (6.1..6.24).contains(&phase);
        // Small independent idle hops. There is no shared interaction timeline.
        let hop = if (4.0..4.8).contains(&phase) {
            ((phase - 4.0) / 0.8 * PI).sin() * 2.5 * scale
        } else {
            0.0
        };
        // A gentle one-pixel rise replaces full-body breathing deformation.
        // Snap the combined movement once so every source pixel moves together.
        let breath = (1.0 - (phase * TAU / 4.0).cos()) * 0.5 * scale;
        let lift = (hop + breath).round() as isize;
        let center = column * (slot as f64 + 0.5);
        let left = (center - dw as f64 * 0.5).round() as isize;
        let top = h as isize - 9 - dh as isize - lift;
        for dy in 0..dh {
            for dx in 0..dw {
                // Nearest-neighbour sampling, never vector reconstruction.
                let x = (dx * WIDTH / dw).min(WIDTH - 1);
                let y = (dy * HEIGHT / dh).min(HEIGHT - 1);
                let rgba = sample(id, x, y, blink);
                let tx = left + dx as isize;
                let ty = top + dy as isize;
                if rgba[3] != 0 && tx >= 0 && ty >= 0 && (tx as usize) < w && (ty as usize) < h {
                    pixels[ty as usize * w + tx as usize] = [rgba[0], rgba[1], rgba[2]];
                }
            }
        }
        let name = if column >= (NAMES[id].len() * 4 + 1) as f64 {
            NAMES[id]
        } else {
            ["DS", "CL", "GPT"][id]
        };
        label(&mut pixels, w, h, center, name, ACCENTS[id]);
    }
    pixels
}

fn label(pixels: &mut [[u8; 3]], w: usize, h: usize, center: f64, name: &str, color: [u8; 3]) {
    let left = (center - (name.len() * 4 - 1) as f64 / 2.0).floor() as isize;
    for (i, ch) in name.bytes().enumerate() {
        let rows = match ch {
            b'A' => [2, 5, 7, 5, 5],
            b'C' => [3, 4, 4, 4, 3],
            b'D' => [6, 5, 5, 5, 6],
            b'E' => [7, 4, 6, 4, 7],
            b'G' => [3, 4, 5, 5, 3],
            b'H' => [5, 5, 7, 5, 5],
            b'K' => [5, 5, 6, 5, 5],
            b'L' => [4, 4, 4, 4, 7],
            b'P' => [6, 5, 6, 4, 4],
            b'S' => [3, 4, 2, 1, 6],
            b'T' => [7, 2, 2, 2, 2],
            b'U' => [5, 5, 5, 5, 7],
            _ => [0; 5],
        };
        for (dy, bits) in rows.into_iter().enumerate() {
            for dx in 0..3 {
                let x = left + (i * 4 + dx) as isize;
                if bits & (4 >> dx) != 0 && x >= 0 && (x as usize) < w {
                    pixels[(h - 6 + dy) * w + x as usize] = color;
                }
            }
        }
    }
}
