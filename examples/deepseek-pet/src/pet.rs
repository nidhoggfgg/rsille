use std::fmt::Write;

const W: usize = 96;
const H: usize = 48;
const ATLAS: &[u8; 24 * W * H * 4] =
    include_bytes!("../../ai-babies/assets/deepseek-actions-96x48.rgba");

/// Return (column, row, SGR-colored text) runs for a cleared viewport.
/// Transparent cells and the rightmost column are never painted.
pub fn frame(tick: u64, cols: u16, rows: u16) -> Vec<(u16, u16, String)> {
    let usable = usize::from(cols.saturating_sub(1));
    let rows = usize::from(rows);
    let h = H.min(rows.saturating_sub(rows / 6)).min(usable.div_ceil(2));
    let w = (2 * h).min(usable);
    let (action, t, pose) = animation(tick);
    let margin = usable - w;
    spans(action, t, pose, margin, rows, w, h)
}

fn animation(tick: u64) -> (usize, usize, usize) {
    let action = (tick / 64 % 6) as usize;
    let t = (tick % 64) as usize;
    let pose = match action {
        0 => usize::from((48..52).contains(&t)) * 2, // brief blink
        1 => [0, 1, 2, 3, 2, 3, 1, 0][t / 8],
        2 => (t / 4) % 4,
        3 => [0, 1, 2, 2, 2, 2, 3, 0][t % 32 / 4],
        _ => t / 16,
    };
    (action, t, pose)
}

fn spans(
    action: usize,
    t: usize,
    pose: usize,
    margin: usize,
    rows: usize,
    w: usize,
    h: usize,
) -> Vec<(u16, u16, String)> {
    let mut spans = Vec::new();
    let mut left = margin / 2;
    let mut top = rows - h;
    let returning = action == 2 && t >= 32;
    if action == 2 {
        left += (margin - left).min(16) * t.min(63 - t) / 31;
    } else if action == 3 && pose == 2 {
        let flight = t % 32 - 8;
        top -= top.min(5) * (flight * (15 - flight)) / 56;
    }
    for y in 0..h {
        let mut run = String::new();
        let mut start = 0;
        let mut color = None;
        for x in 0..w {
            let sx = (2 * x + 1) * W / (2 * w);
            let sx = if returning { W - 1 - sx } else { sx };
            let sy = (2 * y + 1) * H / (2 * h);
            let i = (((action * 4 + pose) * H + sy) * W + sx) * 4;
            if ATLAS[i + 3] < 128 {
                if !run.is_empty() {
                    run.push_str("\x1b[0m");
                    spans.push((
                        (left + start) as u16,
                        (top + y) as u16,
                        std::mem::take(&mut run),
                    ));
                }
                color = None;
                continue;
            }
            if run.is_empty() {
                start = x;
            }
            let rgb = (ATLAS[i], ATLAS[i + 1], ATLAS[i + 2]);
            if color != Some(rgb) {
                write!(run, "\x1b[38;2;{};{};{}m", rgb.0, rgb.1, rgb.2).unwrap();
                color = Some(rgb);
            }
            run.push('█');
        }
        if !run.is_empty() {
            run.push_str("\x1b[0m");
            spans.push(((left + start) as u16, (top + y) as u16, run));
        }
    }
    spans
}

#[derive(PartialEq)]
struct Image {
    col: u16,
    row: u16,
    cell_w: u16,
    cell_h: u16,
    w: usize,
    pixels: Vec<u16>,
}

#[derive(Default)]
pub struct Pet {
    previous: Option<Image>,
    dirty: bool,
}

impl Pet {
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }

    /// Remove the visible sprite while its original coordinates are still valid.
    pub fn hide(&mut self, out: &mut Vec<u8>) {
        use std::io::Write;
        let Some(old) = self.previous.take() else {
            return;
        };
        let cw = usize::from(old.cell_w);
        let ch = usize::from(old.cell_h);
        let mut cells = vec![false; old.w.div_ceil(cw)];
        out.extend_from_slice(b"\x1b[49m");
        for (dy, band) in old.pixels.chunks(old.w * ch).enumerate() {
            cells.fill(false);
            for (i, &pixel) in band.iter().enumerate() {
                if pixel != 256 {
                    cells[(i % old.w) / cw] = true;
                }
            }
            let mut x = 0;
            for run in cells.chunk_by(|a, b| a == b) {
                if run[0] {
                    write!(
                        out,
                        "\x1b[{};{}H\x1b[{}X",
                        usize::from(old.row) + dy,
                        usize::from(old.col) + x,
                        run.len()
                    )
                    .unwrap();
                }
                x += run.len();
            }
        }
    }

    pub fn draw(
        &mut self,
        tick: u64,
        cols: u16,
        rows: u16,
        cell_w: u16,
        cell_h: u16,
        out: &mut Vec<u8>,
    ) {
        use rsille::render::raster_output::CellPixels;
        use std::io::Write;
        // A PTY can report rows and columns without pixel dimensions.
        if cell_w == 0 || cell_h == 0 {
            return;
        }
        let width = usize::from(cols.saturating_sub(1)) * usize::from(cell_w);
        let height = usize::from(rows) * usize::from(cell_h);
        // Even a partial final Sixel band must fit without scrolling.
        // The atlas compresses square source tiles for text cells, not Sixel pixels.
        const SIDE: usize = 128;
        let h = SIDE.min(width).min(height / 6 * 6);
        if h == 0 {
            self.invalidate();
            return;
        }
        let w = h;
        let metrics = CellPixels {
            width: cell_w,
            height: cell_h,
        };
        let size = metrics
            .size_for_pixels(w as u32, h.next_multiple_of(6) as u32)
            .unwrap();
        // Every few swimming strokes, hover and take a bite. A triangular
        // path turns at the viewport edges without teleporting on wide screens.
        let phase = tick % 40;
        let outing = tick / 40;
        // Each outing covers twelve cells, but alternate gliding with a dash.
        let stride = if outing % 3 == 1 { 1 } else { 2 };
        let strokes = 12 * stride;
        let steps = outing * 12 + phase.min(strokes) / stride;
        let travel = u64::from(cols - size.width);
        let path = (travel / 2 + steps) % (2 * travel).max(1);
        let leftward = path >= travel;
        let col = 1 + path.min(2 * travel - path) as u16;
        let rise = rows - size.height;
        let depth = (steps / 3) % (2 * u64::from(rise)).max(1);
        let mut row = 1 + rise - depth.min(2 * u64::from(rise) - depth) as u16;
        let rest = phase.saturating_sub(strokes);
        let activity = outing % 6;
        let (action, pose) = if phase < strokes {
            (2, (phase / (2 * stride) % 4) as usize)
        } else {
            match activity {
                0 => (1, [0, 1, 3, 2][(rest / 4) as usize]), // reach for a snack
                1 => {
                    // crouch, spring up, hang in the air, then land
                    let key = (rest / 2) as usize;
                    row -= (row - 1).min([0, 0, 0, 1, 2, 3, 3, 3, 2, 1, 0, 0, 0, 0][key]);
                    (3, [0, 1, 1, 2, 2, 2, 2, 2, 2, 2, 3, 3, 1, 0][key])
                }
                2 => (4, [0, 1, 2, 2, 3, 3, 2, 0][(rest / 2) as usize]), // look around
                3 => (1, [0, 1, 2, 3, 2, 3, 1, 0][(rest / 2) as usize]), // wave
                4 => (5, [0, 1, 2, 3, 2, 1, 0][(rest / 4) as usize]),    // nap and wake
                _ => (0, usize::from(matches!(rest, 5 | 6 | 12)) * 2),   // double blink
            }
        };
        if activity == 0 && rest == 8 {
            let mouth = if leftward {
                col.checked_sub(1)
            } else {
                col.checked_add(size.width)
            };
            if let Some(x) = mouth.filter(|x| (1..=cols).contains(x)) {
                let y = row + size.height / 2;
                write!(out, "\x1b[{y};{x}H\x1b[X").unwrap();
            }
        }
        let mut pixels = Vec::with_capacity(w * h);
        let mut palette = [false; 256];
        for y in 0..h {
            for x in 0..w {
                let sx = (2 * x + 1) * W / (2 * w);
                let sx = if leftward { W - 1 - sx } else { sx };
                let sy = (2 * y + 1) * H / (2 * h);
                let i = (((action * 4 + pose) * H + sy) * W + sx) * 4;
                let c = if ATLAS[i + 3] < 128 {
                    256
                } else {
                    let c = (ATLAS[i] & 224) | ((ATLAS[i + 1] >> 3) & 28) | (ATLAS[i + 2] >> 6);
                    palette[c as usize] = true;
                    u16::from(c)
                };
                pixels.push(c);
            }
        }
        let image = Image {
            col,
            row,
            cell_w,
            cell_h,
            w,
            pixels,
        };
        if !self.dirty && self.previous.as_ref() == Some(&image) {
            return;
        }
        self.dirty = false;
        if let Some(old) = &self.previous {
            let mut stale = std::collections::BTreeSet::new();
            for (i, &color) in old.pixels.iter().enumerate() {
                if color == 256 {
                    continue;
                }
                let (x, y) = (i % old.w, i / old.w);
                let c = old.col + (x / usize::from(old.cell_w)) as u16;
                let r = old.row + (y / usize::from(old.cell_h)) as u16;
                if c > cols || r > rows {
                    continue;
                }
                if (old.cell_w, old.cell_h) == (cell_w, cell_h) && c >= col && r >= row {
                    let nx = usize::from(c - col) * usize::from(cell_w) + x % usize::from(cell_w);
                    let ny = usize::from(r - row) * usize::from(cell_h) + y % usize::from(cell_h);
                    if nx < w && ny < h && image.pixels[ny * w + nx] != 256 {
                        continue;
                    }
                }
                stale.insert((r, c));
            }
            // ECH on terminals that erase graphics restores only the default
            // background, not underlying content. Redraw surviving pixels below.
            if !stale.is_empty() {
                out.extend_from_slice(b"\x1b[49m");
                for (r, c) in stale {
                    write!(out, "\x1b[{r};{c}H\x1b[X").unwrap();
                }
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
                let mut band = [0u8; SIDE];
                for dy in 0..6.min(h - y) {
                    for (x, mask) in band[..w].iter_mut().enumerate() {
                        if image.pixels[(y + dy) * w + x] == c as u16 {
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
        self.previous = Some(image);
    }
}

#[test]
fn unknown_cell_size_emits_nothing() {
    let mut out = Vec::new();
    for (w, h) in [(0, 0), (0, 20), (10, 0)] {
        Pet::default().draw(0, 80, 24, w, h, &mut out);
        assert!(out.is_empty());
    }
}

#[test]
fn sixel_redraw_contract() {
    let mut pet = Pet::default();
    let mut out = Vec::new();
    pet.draw(0, 80, 24, 10, 20, &mut out);
    let first = std::mem::take(&mut out);
    assert!(first.starts_with(b"\x1b[18;34H\x1bP0;1q\"1;1;128;128"));
    assert!(first.ends_with(b"\x1b\\"));
    pet.draw(1, 80, 24, 10, 20, &mut out);
    assert!(out.is_empty());
    pet.invalidate();
    pet.draw(1, 80, 24, 10, 20, &mut out);
    assert_eq!(out, first);
    out.clear();
    pet.draw(48, 80, 24, 10, 20, &mut out);
    assert!(!out.is_empty());
    assert_ne!(out, first);
    out.clear();
    pet.draw(128, 80, 24, 10, 20, &mut out);
    let erased = out.windows(3).filter(|s| *s == b"\x1b[X").count();
    assert!((1..=91).contains(&erased));
    out.clear();
    pet.draw(32, 80, 24, 10, 20, &mut out);
    assert!(out.starts_with(b"\x1b[17;59H\x1b[X"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_outings_render_through_takeoff_and_landing() {
        let mut pet = Pet::default();
        let mut out = Vec::new();
        let mut poses = Vec::new();
        for tick in 0..240 {
            out.clear();
            pet.draw(tick, 30, 12, 4, 8, &mut out);
            if tick % 40 == 32 {
                poses.push(pet.previous.as_ref().unwrap().pixels.clone());
            }
        }
        poses.sort();
        poses.dedup();
        assert_eq!(poses.len(), 6);
    }

    #[test]
    fn spans_fit_without_wrapping() {
        for (cols, rows) in [
            (0, 9),
            (1, 8),
            (8, 0),
            (2, 1),
            (19, 7),
            (80, 24),
            (u16::MAX, u16::MAX),
        ] {
            for tick in [0, 95, 159, 176, 208, 288, u64::MAX] {
                let spans = frame(tick, cols, rows);
                if cols < 2 || rows == 0 {
                    assert!(spans.is_empty());
                }
                for (x, y, text) in spans {
                    assert!(y < rows);
                    assert!(usize::from(x) + text.matches('█').count() < usize::from(cols));
                    assert!(text.starts_with("\x1b[38;2;") && text.ends_with("\x1b[0m"));
                }
            }
        }
    }

    #[test]
    fn actions_animate_and_repeat() {
        for (action, phase) in [48, 20, 31, 16, 40, 48].iter().enumerate() {
            let tick = action as u64 * 64;
            let initial = frame(tick, 120, 32);
            assert!(!initial.is_empty());
            assert_ne!(initial, frame(tick + phase, 120, 32));
            assert_eq!(initial, frame(tick + 384, 120, 32));
        }
    }
}
