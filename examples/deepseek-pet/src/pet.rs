mod behavior;
mod speech;

use crate::{
    animation::{self, SIDE},
    Feature,
};
use behavior::{Life, Mood};
use speech::{Bubble, Speech};
use std::{collections::BTreeSet, fmt::Write, io};

const MAX_SIDE: usize = 192;

/// Return (column, row, SGR-colored text) runs for a cleared viewport.
/// Transparent cells and the rightmost column are never painted.
#[allow(dead_code)] // Text preview; the interactive session uses Sixel.
pub fn frame(tick: u64, cols: u16, rows: u16) -> Vec<(u16, u16, String)> {
    let usable = usize::from(cols.saturating_sub(1));
    let rows = usize::from(rows);
    let h = 64
        .min(rows.saturating_sub(rows / 6))
        .min(usable.div_ceil(2));
    let w = (2 * h).min(usable);
    let clips = [
        &animation::PREVIEW_IDLE,
        &animation::PREVIEW_FIN,
        &animation::SWIM,
        &animation::PREVIEW_HOP,
        &animation::PREVIEW_WATCH,
        &animation::PREVIEW_NAP,
    ];
    let action = (tick / 64 % clips.len() as u64) as usize;
    let t = (tick % 64) as usize;
    let tile = animation::pixels(clips[action].frame(t as u16));
    let margin = usable - w;
    let mut spans = Vec::new();
    let mut left = margin / 2;
    let mut top = rows - h;
    let returning = action == 2 && t >= 32;
    if action == 2 {
        left += (margin - left).min(16) * t.min(63 - t) / 31;
    } else if action == 3 && (8..24).contains(&(t % 32)) {
        let flight = t % 32 - 8;
        top -= top.min(5) * (flight * (15 - flight)) / 56;
    }
    for y in 0..h {
        let mut run = String::new();
        let mut start = 0;
        let mut color = None;
        for x in 0..w {
            let sx = (2 * x + 1) * SIDE / (2 * w);
            let sx = if returning { SIDE - 1 - sx } else { sx };
            let sy = (2 * y + 1) * SIDE / (2 * h);
            let i = (sy * SIDE + sx) * 4;
            if tile[i + 3] < 128 {
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
            let rgb = (tile[i], tile[i + 1], tile[i + 2]);
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

impl Image {
    fn covers_cell(&self, col: u16, row: u16) -> bool {
        let (Some(dx), Some(dy)) = (col.checked_sub(self.col), row.checked_sub(self.row)) else {
            return false;
        };
        let x = usize::from(dx) * usize::from(self.cell_w);
        let y = usize::from(dy) * usize::from(self.cell_h);
        x < self.w
            && self
                .pixels
                .chunks_exact(self.w)
                .skip(y)
                .take(usize::from(self.cell_h))
                .any(|line| {
                    line[x..]
                        .iter()
                        .take(usize::from(self.cell_w))
                        .any(|&c| c != 256)
                })
    }
}

#[derive(PartialEq)]
struct FrameKey {
    origin: (u16, u16),
    cell_pixels: (u16, u16),
    side: usize,
    frame: usize,
    leftward: bool,
}

pub struct Pet<'a> {
    catalog: &'a crate::dialogue::Catalog,
    features: BTreeSet<Feature>,
    speech: Option<Speech<'a>>,
    bubble: Option<Bubble<'a>>,
    life: Life,
    previous: Option<Image>,
    last_frame: Option<FrameKey>,
    dirty: bool,
}

#[cfg(test)]
impl Default for Pet<'static> {
    fn default() -> Self {
        static CATALOG: std::sync::LazyLock<crate::dialogue::Catalog> =
            std::sync::LazyLock::new(|| crate::dialogue::Catalog::load(None).unwrap());
        Self::new(&CATALOG, crate::Options::default().enabled_features())
    }
}

impl<'a> Pet<'a> {
    pub fn new(catalog: &'a crate::dialogue::Catalog, features: BTreeSet<Feature>) -> Self {
        use std::hash::BuildHasher;
        let choice = std::collections::hash_map::RandomState::new().hash_one(()) as u32;
        Self {
            life: Life::new(choice, &features),
            catalog,
            speech: features.contains(&Feature::Speech).then(Speech::default),
            features,
            bubble: None,
            previous: None,
            last_frame: None,
            dirty: false,
        }
    }

    pub fn notice_command(&mut self, phase: u8, command: usize) {
        if (phase, command) == (b'e', 0) {
            if let Some(speech) = &mut self.speech {
                speech.clear_preview();
            }
            return;
        }
        let Some(event) = self.catalog.event(phase, command) else {
            return;
        };
        if let Some(speech) = &mut self.speech {
            speech.notice_command(event);
        }
        if self.features.contains(&Feature::CommandReactions) {
            self.life.notice_command(event.phase);
        }
    }

    pub fn notice_input(&mut self) {
        if self.features.contains(&Feature::Typing) {
            self.life.notice_input();
        }
    }

    pub fn notice_bell(&mut self) {
        if self.features.contains(&Feature::Bell) {
            self.life.notice_bell();
        }
    }

    pub fn notice_mouse(&mut self, col: u16, row: u16) {
        self.life.notice_mouse(col, row);
    }

    pub fn notice_touch(&mut self, col: u16, row: u16) {
        if self.features.contains(&Feature::Mouse)
            && self.life.mood != Mood::Wave
            && self
                .previous
                .as_ref()
                .is_some_and(|image| image.covers_cell(col, row))
        {
            self.life.react(Mood::Wave);
        }
    }

    /// Replace the latest edible output cell (one-based terminal coordinates).
    pub fn notice_food(&mut self, col: u16, row: u16) {
        if let Some(foraging) = &mut self.life.foraging {
            foraging.target = Some((col, row));
        }
    }

    pub fn forget_food(&mut self) {
        if let Some(foraging) = &mut self.life.foraging {
            foraging.target = None;
        }
    }
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }

    /// Remove the visible sprite while its original coordinates are still valid.
    pub fn hide(&mut self, cols: u16, rows: u16, out: &mut Vec<u8>) {
        use std::io::Write;
        if let Some(old) = self.bubble.take() {
            old.erase(cols, rows, out);
        }
        let Some(old) = self.previous.take() else {
            return;
        };
        self.last_frame = None;
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
    ) -> io::Result<()> {
        use rsille::render::raster_output::CellPixels;
        use std::io::Write;
        // A PTY can report rows and columns without pixel dimensions.
        if cell_w == 0 || cell_h == 0 {
            return Ok(());
        }
        let width = usize::from(cols.saturating_sub(1)) * usize::from(cell_w);
        let height = usize::from(rows) * usize::from(cell_h);
        // Even a partial final Sixel band must fit without scrolling.
        let h = (height / 3)
            .clamp(48, MAX_SIDE)
            .min(width)
            .min(height / 6 * 6);
        if h == 0 {
            self.invalidate();
            return Ok(());
        }
        let w = h;
        let metrics = CellPixels {
            width: cell_w,
            height: cell_h,
        };
        let size = metrics
            .size_for_pixels(w as u32, h.next_multiple_of(6) as u32)
            .unwrap();
        let travel = cols - 1 - size.width;
        let rise = rows - size.height;
        if let Some((c, r)) = self
            .life
            .update(tick, travel, rise, size.width, size.height)
        {
            write!(out, "\x1b[49m\x1b[{r};{c}H\x1b[X").unwrap();
            self.dirty = true;
        }
        let frame = self.life.frame();
        let col = 1 + self.life.x;
        let row = 1 + (rise - self.life.altitude).saturating_sub(self.life.lift());
        let bubble = self
            .speech
            .as_mut()
            .and_then(|speech| speech.place(col, row, (size.width, size.height), cols, rows));
        if self.bubble != bubble {
            if let Some(old) = self.bubble.take() {
                old.erase(cols, rows, out);
            }
            self.dirty = true;
        }
        let leftward = self.life.leftward;
        let key = FrameKey {
            origin: (col, row),
            cell_pixels: (cell_w, cell_h),
            side: w,
            frame,
            leftward,
        };
        if !self.dirty && self.last_frame.as_ref() == Some(&key) {
            return Ok(());
        }
        self.last_frame = Some(key);
        let mut pixels = Vec::with_capacity(w * h);
        let mut palette = [false; 256];
        let tile = animation::pixels(frame);
        for y in 0..h {
            let sy = (2 * y + 1) * SIDE / (2 * h);
            for x in 0..w {
                let x = if leftward { w - 1 - x } else { x };
                let sx = (2 * x + 1) * SIDE / (2 * w);
                let i = (sy * SIDE + sx) * 4;
                let c = if tile[i + 3] < 128 {
                    256
                } else {
                    let c = (tile[i] & 224) | ((tile[i + 1] >> 3) & 28) | (tile[i + 2] >> 6);
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
            return Ok(());
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
                let mut band = [0u8; MAX_SIDE];
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
        if let Some(b) = bubble {
            b.render(out)?;
        }
        self.bubble = bubble;
        Ok(())
    }
}

#[test]
fn unknown_cell_size_emits_nothing() {
    let mut out = Vec::new();
    for (w, h) in [(0, 0), (0, 20), (10, 0)] {
        Pet::default().draw(0, 80, 24, w, h, &mut out).unwrap();
        assert!(out.is_empty());
    }
}

#[test]
fn sixel_redraw_contract() {
    let mut pet = Pet::default();
    pet.life.choice = 0;
    let mut out = Vec::new();
    pet.draw(0, 80, 24, 10, 20, &mut out).unwrap();
    let first = std::mem::take(&mut out);
    assert!(first.starts_with(b"\x1b[16;64H\x1bP0;1q\"1;1;160;160"));
    assert!(first.ends_with(b"\x1b\\"));
    pet.draw(1, 80, 24, 10, 20, &mut out).unwrap();
    assert!(out.is_empty());
    pet.invalidate();
    pet.draw(1, 80, 24, 10, 20, &mut out).unwrap();
    assert_eq!(out, first);
    out.clear();
    pet.draw(32, 80, 24, 10, 20, &mut out).unwrap();
    assert!(!out.is_empty());
    assert_ne!(out, first);
    out.clear();
    pet.draw(128, 80, 24, 10, 20, &mut out).unwrap();
    let erased = out.windows(3).filter(|s| *s == b"\x1b[X").count();
    assert!((1..=16 * 9).contains(&erased));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn speech_and_mouse_work_together_at_a_fixed_position() {
        let catalog = crate::dialogue::Catalog::load(None).unwrap();
        let mut pet = Pet::new(&catalog, [Feature::Speech, Feature::Mouse].into());
        let mut out = Vec::new();
        pet.draw(0, 80, 24, 10, 20, &mut out).unwrap();
        let (col, row) = pet.last_frame.as_ref().unwrap().origin;
        pet.notice_command(b'+', catalog.commands["ls"]);
        pet.notice_input();
        pet.notice_bell();
        pet.notice_food(1, 1);
        pet.notice_mouse(1, 1);
        pet.draw(1, 80, 24, 10, 20, &mut out).unwrap();
        assert!(pet.bubble.is_some());
        assert_eq!(pet.life.mood, Mood::Idle);
        assert!(pet.life.foraging.is_none() && pet.life.pointer.is_none());
        pet.notice_touch(col + 6, row + 3);
        assert_eq!(pet.life.mood, Mood::Wave);
        for tick in [2, 120, 240, 360] {
            pet.draw(tick, 80, 24, 10, 20, &mut out).unwrap();
            assert_eq!(pet.last_frame.as_ref().unwrap().origin, (col, row));
        }
        pet.draw(361, 100, 24, 10, 20, &mut out).unwrap();
        assert_eq!(pet.last_frame.as_ref().unwrap().origin, (col + 20, row));
    }

    #[test]
    fn command_gestures_work_without_speech_mouse_or_roaming() {
        let catalog = crate::dialogue::Catalog::load(None).unwrap();
        let mut pet = Pet::new(&catalog, [Feature::CommandReactions].into());
        let mut out = Vec::new();
        pet.draw(0, 80, 24, 10, 20, &mut out).unwrap();
        let (col, row) = pet.last_frame.as_ref().unwrap().origin;
        pet.notice_touch(col + 6, row + 3);
        assert_eq!(pet.life.mood, Mood::Idle);
        pet.notice_command(b'-', catalog.commands["ls"]);
        pet.draw(4, 80, 24, 10, 20, &mut out).unwrap();
        assert_eq!(pet.life.mood, Mood::Startled);
        assert_eq!(pet.last_frame.as_ref().unwrap().origin, (col, row));
        assert!(pet.bubble.is_none());
    }

    #[test]
    fn anchored_foraging_only_eats_characters_within_reach() {
        let catalog = crate::dialogue::Catalog::default();
        let mut pet = Pet::new(&catalog, [Feature::Forage].into());
        pet.notice_food(1, 1);
        pet.life.update(0, 10, 5, 4, 3);
        assert_eq!(pet.life.update(60, 10, 5, 4, 3), None);
        pet.notice_food(13, 7);
        assert_eq!(pet.life.update(61, 10, 5, 4, 3), Some((13, 7)));
        assert_eq!(
            (pet.life.x, pet.life.altitude, pet.life.mood),
            (10, 0, Mood::Snack)
        );
    }

    #[test]
    fn command_bubble_tracks_edits_results_and_available_space() {
        let mut pet = Pet::default();
        let mut out = Vec::new();
        pet.notice_command(b'e', pet.catalog.editing("cargo test").unwrap());
        let edit = pet.speech.as_ref().unwrap().latest_speech.unwrap().0;
        pet.draw(0, 80, 24, 10, 20, &mut out).unwrap();
        let first = pet.bubble.unwrap();
        assert_eq!(first.text, edit);
        assert!(first.area.real_size().width < 80);
        assert_eq!(
            first.area.real_size().height + 1,
            pet.previous.as_ref().unwrap().row
        );
        pet.notice_command(b'e', 0);
        out.clear();
        pet.draw(0, 80, 24, 10, 20, &mut out).unwrap();
        assert_eq!(pet.bubble.unwrap().text, edit);
        pet.speech.as_mut().unwrap().held_speech.as_mut().unwrap().1 = Instant::now();
        pet.draw(0, 80, 24, 10, 20, &mut out).unwrap();
        let mut erase = Vec::new();
        first.erase(80, 24, &mut erase);
        assert!(out.starts_with(&erase));
        assert!(pet.bubble.is_none());
        pet.notice_command(b'r', pet.catalog.editing("cargo test").unwrap());
        pet.notice_command(b'+', pet.catalog.editing("cargo test").unwrap());
        assert_eq!(pet.life.mood, Mood::Wave);
        pet.draw(1, 5, 2, 10, 20, &mut out).unwrap();
        assert!(pet.bubble.is_none());
        pet.draw(1, 80, 24, 10, 20, &mut out).unwrap();
        assert_eq!(
            pet.bubble.unwrap().text,
            pet.speech.as_ref().unwrap().latest_speech.unwrap().0
        );
        assert_ne!(pet.bubble.unwrap().text, edit);
        pet.speech.as_mut().unwrap().held_speech.as_mut().unwrap().1 = Instant::now();
        pet.speech
            .as_mut()
            .unwrap()
            .latest_speech
            .as_mut()
            .unwrap()
            .1 = Instant::now();
        pet.draw(2, 80, 24, 10, 20, &mut out).unwrap();
        assert!(pet.bubble.is_none());
    }

    #[test]
    fn rapid_command_changes_hold_the_visible_line_then_show_the_latest_result() {
        let mut pet = Pet::default();
        let mut out = Vec::new();
        pet.notice_command(b'e', pet.catalog.editing("git").unwrap());
        pet.draw(0, 80, 24, 10, 20, &mut out).unwrap();
        let first = pet.speech.as_ref().unwrap().held_speech.unwrap();
        for (phase, kind) in [
            (b'e', pet.catalog.editing("git status").unwrap()),
            (b'e', 0),
            (b'r', pet.catalog.editing("git status").unwrap()),
            (b'+', pet.catalog.editing("git status").unwrap()),
        ] {
            pet.notice_command(phase, kind);
            pet.hide(80, 24, &mut out);
            pet.draw(1, 80, 24, 10, 20, &mut out).unwrap();
            assert_eq!(pet.bubble.unwrap().text, first.0);
            assert_eq!(pet.speech.as_ref().unwrap().held_speech.unwrap(), first);
        }
        let result = pet.speech.as_ref().unwrap().latest_speech.unwrap().0;
        pet.speech.as_mut().unwrap().held_speech.as_mut().unwrap().1 = Instant::now();
        pet.draw(2, 80, 24, 10, 20, &mut out).unwrap();
        assert_ne!(result, first.0);
        assert_eq!(pet.bubble.unwrap().text, result);
    }

    #[test]
    fn raster_resizes_with_the_terminal_and_fits_the_last_sixel_band() {
        let mut pet = Pet::default();
        for (cols, rows, side) in [(100, 40, 192), (80, 24, 160), (5, 24, 40), (80, 1, 18)] {
            pet.draw(0, cols, rows, 10, 20, &mut Vec::new()).unwrap();
            let image = pet.previous.as_ref().unwrap();
            assert_eq!(image.w, side);
            assert!(usize::from(image.col - 1) * 10 + side <= usize::from(cols - 1) * 10);
            assert!(
                usize::from(image.row - 1) * 20 + side.next_multiple_of(6)
                    <= usize::from(rows) * 20
            );
        }
    }

    #[test]
    fn touch_requires_a_painted_cell_and_finishes_despite_other_events() {
        let mut pet = Pet::default();
        let mut out = Vec::new();
        pet.draw(0, 80, 24, 10, 20, &mut out).unwrap();
        let image = pet.previous.as_ref().unwrap();
        let (col, row) = (image.col, image.row);
        pet.notice_touch(col, row);
        assert_eq!(pet.life.mood, Mood::Idle);
        pet.notice_touch(col + 6, row + 3);
        pet.life.foraging.as_mut().unwrap().hunger = 100;
        pet.notice_food(1, 1);
        let duration = u64::from(animation::WAVE.duration);
        for tick in 1..duration {
            pet.notice_touch(col + 6, row + 3);
            pet.notice_bell();
            pet.life.update(tick, 66, 17, 13, 7);
            assert_eq!((pet.life.mood, pet.life.age), (Mood::Wave, tick as u16));
        }
        pet.life.update(duration, 66, 17, 13, 7);
        assert_eq!(pet.life.mood, Mood::Idle);
    }

    #[test]
    fn missing_command_shows_a_stationary_question_mark_with_its_reply() {
        for leftward in [false, true] {
            let mut pet = Pet::default();
            pet.life.leftward = leftward;
            pet.life.altitude = 4;
            pet.draw(0, 80, 24, 10, 20, &mut Vec::new()).unwrap();
            let origin = pet.last_frame.as_ref().unwrap().origin;
            pet.notice_command(b'n', 0);
            let reply = pet.speech.as_ref().unwrap().latest_speech.unwrap().0;
            for tick in 1..16 {
                pet.draw(tick, 80, 24, 10, 20, &mut Vec::new()).unwrap();
                assert_eq!(pet.last_frame.as_ref().unwrap().origin, origin);
                assert_eq!(pet.bubble.unwrap().text, reply);
                // The upper hook curves to the reader's right, above a separate dot.
                let ink = |x: usize, y: usize| {
                    let x = if leftward { 79 - x } else { 70 + x };
                    let image = pet.previous.as_ref().unwrap();
                    let px = (2 * x + 1) * image.w / (2 * 96);
                    let px = if leftward { image.w - 1 - px } else { px };
                    let py = (2 * y + 1) * image.w / (2 * 48);
                    image.pixels[py * image.w + px] == 0b100_111_11
                };
                assert!(ink(8, 7) && !ink(0, 7));
                assert!(!ink(4, 10) && ink(4, 11));
            }
            pet.draw(16, 80, 24, 10, 20, &mut Vec::new()).unwrap();
            assert_eq!(pet.life.mood, Mood::Idle);
        }
    }

    #[test]
    fn bell_wakes_a_sleeping_pet_and_repeated_bells_allow_it_to_land() {
        let mut pet = Pet::default();
        pet.life.mood = Mood::Nap;
        for tick in 0..=12 {
            pet.notice_bell();
            pet.life.update(tick, 5, 5, 1, 1);
            if tick == 4 {
                assert_eq!(pet.life.lift(), 5);
            }
        }
        assert_eq!(pet.life.mood, Mood::Idle);
        pet.notice_bell();
        assert_eq!(pet.life.mood, Mood::Idle);
        pet.life.update(60, 5, 5, 1, 1);
        pet.notice_bell();
        assert_eq!(pet.life.mood, Mood::Startled);
    }

    #[test]
    fn typing_cannot_restart_or_chain_a_glance() {
        let mut pet = Pet::default();
        pet.notice_input();
        for tick in 0..16 {
            pet.life.update(tick, 20, 0, 1, 1);
            pet.notice_input();
            assert_eq!(pet.life.mood, Mood::Watch);
            assert_eq!(pet.life.age, tick as u16);
        }
        pet.life.update(16, 20, 0, 1, 1);
        pet.notice_input();
        assert_eq!(pet.life.mood, Mood::Idle);
    }

    #[test]
    fn moving_pointer_cannot_extend_pursuit() {
        let mut pet = Pet::default();
        pet.life.pointer.as_mut().unwrap().curiosity = 0;
        for tick in 0..=33 {
            pet.notice_mouse(1 + tick as u16, 1);
            pet.life.update(tick, 200, 0, 1, 1);
        }
        assert!(pet.life.pointer.as_ref().unwrap().target.is_none());
        assert!(pet.life.cooldown > 0);
    }

    #[test]
    fn targets_follow_both_axes_and_stale_food_is_not_eaten() {
        let mut pet = Pet::default();
        pet.life.pointer.as_mut().unwrap().curiosity = 0;
        pet.notice_mouse(12, 2);
        pet.life.update(0, 20, 10, 1, 1);
        pet.life.update(9, 20, 10, 1, 1);
        assert_eq!((pet.life.x, pet.life.altitude), (11, 9));
        pet.life.foraging.as_mut().unwrap().hunger = 100;
        pet.notice_food(18, 8);
        assert_eq!(pet.life.update(12, 20, 10, 1, 1), None);
        pet.forget_food();
        assert_eq!(pet.life.update(45, 20, 10, 1, 1), None);
        pet.notice_food(3, 4);
        assert_eq!(pet.life.update(70, 20, 10, 1, 1), Some((3, 4)));
        pet.notice_food(4, 4);
        assert_eq!(pet.life.update(110, 20, 10, 1, 1), None);
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
