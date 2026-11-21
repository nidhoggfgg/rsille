mod behavior;
mod fish;
mod sixel;
mod speech;

use crate::{
    animation::{self, SIDE},
    food::Treat,
    mouse::MouseKind,
    Feature,
};
use behavior::{Life, Mood};
use fish::FishIcon;
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
    /// Erase old ink that the new sprite no longer covers, in horizontal cell runs.
    fn clear_uncovered(
        &self,
        next: Option<&Image>,
        cols: u16,
        rows: u16,
        screen: Option<&crate::screen::Screen>,
        out: &mut Vec<u8>,
    ) {
        if self.col > cols || self.row > rows {
            return;
        }
        let cw = usize::from(self.cell_w);
        let ch = usize::from(self.cell_h);
        let same_grid = next.filter(|n| (n.cell_w, n.cell_h) == (self.cell_w, self.cell_h));
        let width = self.w.div_ceil(cw).min(usize::from(cols - self.col) + 1);
        let mut dirty = vec![false; width];
        let start = out.len();
        for (dy, band) in self
            .pixels
            .chunks(self.w * ch)
            .take(usize::from(rows - self.row) + 1)
            .enumerate()
        {
            let row = self.row + dy as u16;
            dirty.fill(false);
            for (py, line) in band.chunks_exact(self.w).enumerate() {
                for (px, &color) in line.iter().take(width * cw).enumerate() {
                    let dx = px / cw;
                    if color == 256 || dirty[dx] {
                        continue;
                    }
                    let col = self.col + dx as u16;
                    let covered = same_grid.is_some_and(|n| {
                        if col < n.col || row < n.row {
                            return false;
                        }
                        let x = usize::from(col - n.col) * cw + px % cw;
                        let y = usize::from(row - n.row) * ch + py;
                        x < n.w && y < n.w && n.pixels[y * n.w + x] != 256
                    });
                    dirty[dx] = !covered;
                }
            }
            let mut col = usize::from(self.col);
            for run in dirty.chunk_by(|a, b| a == b) {
                if run[0] {
                    if out.len() == start {
                        out.extend_from_slice(b"\x1b[49m");
                    }
                    crate::screen::clear_span(screen, col as u16, row, run.len() as u16, out);
                }
                col += run.len();
            }
        }
    }

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
    pub(crate) screen: Option<crate::screen::Screen>,
    catalog: &'a crate::dialogue::Catalog,
    features: BTreeSet<Feature>,
    speech: Option<Speech<'a>>,
    bubble: Option<Bubble<'a>>,
    fish_icon: Option<FishIcon>,
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
            screen: None,
            life: Life::new(choice, &features),
            catalog,
            speech: features.contains(&Feature::Speech).then(Speech::default),
            features,
            bubble: None,
            fish_icon: None,
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

    pub fn mouse_event(&mut self, kind: MouseKind, col: u16, row: u16) {
        if !self.features.contains(&Feature::Mouse) {
            return;
        }
        match kind {
            MouseKind::LeftPress => {
                let on_fish = self.fish_icon.is_some_and(|fish| fish.contains(col, row));
                if !on_fish
                    && self
                        .previous
                        .as_ref()
                        .is_some_and(|image| image.covers_cell(col, row))
                {
                    if self.life.mood != Mood::Wave {
                        self.life.react(Mood::Wave);
                    }
                } else {
                    self.life.treat = Some(Treat {
                        cell: (col, row),
                        held: true,
                    });
                }
            }
            MouseKind::LeftDrag | MouseKind::LeftRelease => {
                if let Some(treat) = self.life.treat.as_mut().filter(|t| t.held) {
                    treat.cell = (col, row);
                    treat.held = kind == MouseKind::LeftDrag;
                } else if kind == MouseKind::LeftDrag {
                    self.life.notice_mouse(col, row);
                }
            }
            MouseKind::Motion => self.life.notice_mouse(col, row),
            MouseKind::RightPress => self.put_away_treat(),
            MouseKind::Other => {}
        }
    }

    pub fn put_away_treat(&mut self) {
        self.life.treat = None;
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
        if let Some(old) = self.fish_icon.take() {
            old.erase(cols, rows, self.screen.as_ref(), out);
        }
        if let Some(old) = self.bubble.take() {
            old.erase(cols, rows, self.screen.as_ref(), out);
        }
        let Some(old) = self.previous.take() else {
            return;
        };
        self.last_frame = None;
        old.clear_uncovered(None, cols, rows, self.screen.as_ref(), out);
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
            self.put_away_treat();
            self.hide(cols, rows, out);
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
            let start = out.len();
            write!(out, "\x1b[49m\x1b[{r};{c}H\x1b[X").unwrap();
            if let Some(screen) = &mut self.screen {
                screen.consume(&out[start..]);
            }
            self.dirty = true;
        }
        let frame = self.life.frame();
        let col = 1 + self.life.x;
        let row = 1 + (rise - self.life.altitude).saturating_sub(self.life.lift());
        let bubble = self
            .speech
            .as_mut()
            .and_then(|speech| speech.place(col, row, (size.width, size.height), cols, rows));
        let fish = self
            .life
            .treat
            .map(|t| FishIcon::place(t.cell, cols, rows, metrics));
        if self.fish_icon != fish {
            if let Some(old) = self.fish_icon.take() {
                old.erase(cols, rows, self.screen.as_ref(), out);
            }
            self.dirty = true;
        }
        if self.bubble != bubble {
            if let Some(old) = self.bubble.take() {
                old.erase(cols, rows, self.screen.as_ref(), out);
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
            old.clear_uncovered(Some(&image), cols, rows, self.screen.as_ref(), out);
        }
        sixel::draw(col, row, w, &image.pixels, out);
        self.previous = Some(image);
        if let Some(b) = bubble {
            b.render(out)?;
        }
        self.bubble = bubble;
        if let Some(icon) = fish {
            icon.draw(out);
        }
        self.fish_icon = fish;
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
    let before_sprite = out.split(|&byte| byte == b'P').next().unwrap();
    let erased = before_sprite.iter().filter(|&&byte| byte == b'X').count();
    assert!((1..=16 * 9).contains(&erased));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn moving_and_hiding_the_pet_and_bubble_restores_shell_text() {
        let catalog = crate::dialogue::Catalog::load(None).unwrap();
        let mut pet = Pet::new(&catalog, [Feature::Speech, Feature::Roam].into());
        let backing = crate::screen::Screen::new(24, 80, (1, 1)).unwrap();
        pet.screen = Some(backing);
        let mut visible = vt100::Parser::new(24.try_into().unwrap(), 80.try_into().unwrap(), 0);
        let mut shell = Vec::new();
        for row in 1..=24 {
            use std::io::Write;
            write!(shell, "\x1b[{row};1H\x1b[32m{}", "x鲸".repeat(26)).unwrap();
        }
        visible.process(&shell);
        pet.screen.as_mut().unwrap().feed(&shell);
        let expected = visible.screen().clone();
        pet.notice_command(b'+', catalog.commands["ls"]);
        let mut bytes = Vec::new();
        for x in [63, 60, 55, 50] {
            pet.life.x = x;
            pet.draw(0, 80, 24, 10, 20, &mut bytes).unwrap();
            visible.process(&bytes);
            bytes.clear();
        }
        assert!(pet.bubble.is_some());
        pet.hide(80, 24, &mut bytes);
        visible.process(&bytes);
        for row in 0..24 {
            for col in 0..80 {
                assert_eq!(
                    visible.screen().cell(row, col),
                    expected.cell(row, col),
                    "({col},{row})"
                );
            }
        }
    }

    #[test]
    #[ignore = "release-mode timing comparison; run with --ignored --nocapture"]
    fn measure_swimming_over_text() {
        let catalog = crate::dialogue::Catalog::default();
        let mut shell = String::new();
        for row in 1..=40 {
            write!(shell, "\x1b[{row};1H\x1b[36m{}", "鲸abc".repeat(23)).unwrap();
        }
        let mut samples = [Vec::new(), Vec::new()];
        for first in [false, true, false, true, false] {
            for restore in [first, !first] {
                let mut pet = Pet::new(&catalog, [Feature::Roam].into());
                pet.life.choice = 0;
                if restore {
                    let mut screen = crate::screen::Screen::new(40, 120, (1, 1)).unwrap();
                    screen.feed(shell.as_bytes());
                    pet.screen = Some(screen);
                }
                let mut out = Vec::new();
                pet.draw(0, 120, 40, 10, 20, &mut out).unwrap();
                let start = Instant::now();
                for step in 0..400_u16 {
                    pet.life.x = (step % 180).abs_diff(90);
                    pet.life.mood = Mood::Swim;
                    pet.life.age = step % animation::SWIM.duration;
                    out.clear();
                    pet.draw(0, 120, 40, 10, 20, &mut out).unwrap();
                    std::hint::black_box(&out);
                }
                samples[usize::from(restore)].push(start.elapsed() / 400);
            }
        }
        for (restore, mut times) in [false, true].into_iter().zip(samples) {
            times.sort_unstable();
            eprintln!("swim render, restore={restore}: {:?}/frame", times[2]);
        }
    }

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
        pet.mouse_event(MouseKind::Motion, 1, 1);
        pet.draw(1, 80, 24, 10, 20, &mut out).unwrap();
        assert!(pet.bubble.is_some());
        assert_eq!(pet.life.mood, Mood::Idle);
        assert!(pet.life.foraging.is_none() && pet.life.pointer.is_none());
        pet.mouse_event(MouseKind::LeftPress, col + 6, row + 3);
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
        pet.mouse_event(MouseKind::LeftPress, col + 6, row + 3);
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
    fn whale_follows_a_held_treat_then_eats_it_without_erasing_shell_text() {
        let mut pet = Pet::default();
        let mut screen = crate::screen::Screen::new(24, 80, (1, 1)).unwrap();
        let shell = "\x1b[8;20H\x1b[32m鲸鱼\x1b[0m";
        screen.feed(shell.as_bytes());
        pet.screen = Some(screen);
        let mut visible = vt100::Parser::new(24.try_into().unwrap(), 80.try_into().unwrap(), 0);
        visible.process(shell.as_bytes());
        let expected = visible.screen().clone();
        let mut draw = |pet: &mut Pet<'_>, tick| {
            let mut out = Vec::new();
            pet.draw(tick, 80, 24, 10, 20, &mut out).unwrap();
            visible.process(&out);
        };
        draw(&mut pet, 0);
        pet.mouse_event(MouseKind::LeftPress, 40, 8);
        draw(&mut pet, 1);
        assert!(pet.fish_icon.unwrap().contains(40, 8));
        pet.mouse_event(MouseKind::LeftDrag, 21, 8);
        pet.notice_food(1, 1);
        draw(&mut pet, 65);
        assert_eq!(pet.life.mood, Mood::Watch);
        assert!(pet.previous.as_ref().unwrap().covers_cell(21, 8));
        assert!(pet.fish_icon.unwrap().contains(21, 8));
        pet.mouse_event(MouseKind::LeftRelease, 21, 8);
        draw(&mut pet, 66);
        assert_eq!(pet.life.mood, Mood::Snack);
        assert!(pet.fish_icon.is_none());
        assert_eq!(pet.life.foraging.as_mut().unwrap().tick(), None);
        let mut out = Vec::new();
        pet.hide(80, 24, &mut out);
        visible.process(&out);
        for col in 19..23 {
            assert_eq!(visible.screen().cell(7, col), expected.cell(7, col));
        }
    }

    #[test]
    fn fixed_whale_can_be_hand_fed_and_distant_treats_can_be_retrieved() {
        let catalog = crate::dialogue::Catalog::default();
        let mut pet = Pet::new(&catalog, [Feature::Mouse].into());
        let mut out = Vec::new();
        pet.draw(0, 80, 24, 10, 20, &mut out).unwrap();
        let origin = pet.last_frame.as_ref().unwrap().origin;
        pet.mouse_event(MouseKind::LeftPress, 1, 1);
        pet.mouse_event(MouseKind::LeftRelease, 1, 1);
        pet.draw(40, 80, 24, 10, 20, &mut out).unwrap();
        assert!(pet.fish_icon.unwrap().contains(1, 1));
        assert_eq!(pet.last_frame.as_ref().unwrap().origin, origin);
        pet.mouse_event(MouseKind::RightPress, 10, 2);
        pet.draw(41, 80, 24, 10, 20, &mut out).unwrap();
        assert!(pet.fish_icon.is_none());
        pet.mouse_event(MouseKind::LeftPress, 2, 2);
        pet.mouse_event(MouseKind::LeftRelease, origin.0 + 6, origin.1 + 3);
        pet.draw(42, 80, 24, 10, 20, &mut out).unwrap();
        assert_eq!(pet.life.mood, Mood::Snack);
        assert_eq!(pet.last_frame.as_ref().unwrap().origin, origin);
        assert!(pet.life.treat.is_none());
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
        first.erase(80, 24, None, &mut erase);
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
        pet.mouse_event(MouseKind::LeftPress, col, row);
        assert_eq!(pet.life.mood, Mood::Idle);
        pet.mouse_event(MouseKind::LeftPress, col + 6, row + 3);
        pet.life.foraging.as_mut().unwrap().hunger = 100;
        pet.notice_food(1, 1);
        let duration = u64::from(animation::WAVE.duration);
        for tick in 1..duration {
            pet.mouse_event(MouseKind::LeftPress, col + 6, row + 3);
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
            pet.mouse_event(MouseKind::Motion, 1 + tick as u16, 1);
            pet.life.update(tick, 200, 0, 1, 1);
        }
        assert!(pet.life.pointer.as_ref().unwrap().target.is_none());
        assert!(pet.life.cooldown > 0);
    }

    #[test]
    fn targets_follow_both_axes_and_stale_food_is_not_eaten() {
        let mut pet = Pet::default();
        pet.life.pointer.as_mut().unwrap().curiosity = 0;
        pet.mouse_event(MouseKind::Motion, 12, 2);
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
