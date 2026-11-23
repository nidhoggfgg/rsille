mod behavior;
mod fish;
mod menu;
mod sixel;
mod speech;

use crate::deepseek_pet::{
    animation::{self, SIDE},
    food::{Treat, TreatState},
    mouse::MouseKind,
    Feature,
};
use behavior::{Life, Mood};
use fish::{FishBite, FishIcon};
use menu::{Action, Menu};
use rsille::render::area::{Area, Size};
use speech::{Bubble, Speech};
use std::{
    collections::BTreeSet,
    fmt::Write,
    io,
    time::{Duration, Instant},
};

const MAX_SIDE: usize = 192;
const FISH_FRAME_INTERVAL: Duration = Duration::from_nanos(1_000_000_000u64.div_ceil(30));

fn erase_area(
    area: Area,
    cols: u16,
    rows: u16,
    screen: Option<&crate::deepseek_pet::screen::Screen>,
    out: &mut Vec<u8>,
) {
    let viewport = Area::from(Size::from((cols, rows)));
    let Some(visible) = area.clamp_to(&viewport) else {
        return;
    };
    let col = visible.x() + 1;
    let width = visible.width();
    out.extend_from_slice(b"\x1b[49m");
    for y in visible.y()..visible.y() + visible.height() {
        crate::deepseek_pet::screen::clear_span(screen, col, y + 1, width, out);
    }
}

/// Return (column, row, SGR-colored text) runs for a cleared viewport.
/// Transparent cells and the rightmost column are never painted.
#[allow(dead_code)] // Text preview; the interactive session uses native graphics.
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
    pixels: Vec<[u8; 4]>,
}

impl Image {
    /// Erase old ink that the new sprite no longer covers, in horizontal cell runs.
    fn clear_uncovered(
        &self,
        next: Option<&Image>,
        cols: u16,
        rows: u16,
        screen: Option<&crate::deepseek_pet::screen::Screen>,
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
                    if color[3] < 128 || dirty[dx] {
                        continue;
                    }
                    let col = self.col + dx as u16;
                    let covered = same_grid.is_some_and(|n| {
                        if col < n.col || row < n.row {
                            return false;
                        }
                        let x = usize::from(col - n.col) * cw + px % cw;
                        let y = usize::from(row - n.row) * ch + py;
                        x < n.w && y < n.w && n.pixels[y * n.w + x][3] >= 128
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
                    crate::deepseek_pet::screen::clear_span(
                        screen,
                        col as u16,
                        row,
                        run.len() as u16,
                        out,
                    );
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
                        .any(|c| c[3] >= 128)
                })
    }
}

#[derive(PartialEq)]
struct FrameKey {
    origin: (u16, u16),
    cell_pixels: (u16, u16),
    side: usize,
    frame: usize,
    bite: Option<FishBite>,
    leftward: bool,
}

pub struct KittySprites {
    pub whale: rsille::render::raster_output::KittyImage,
    pub fish: rsille::render::raster_output::KittyImage,
}

struct WhaleGrab {
    press: (u16, u16),
    offset: (u16, u16),
    moved: bool,
}

pub struct Pet<'a> {
    pub kitty: Option<&'a KittySprites>,
    pub image_only: bool,
    pub(crate) screen: Option<crate::deepseek_pet::screen::Screen>,
    catalog: &'a crate::deepseek_pet::dialogue::Catalog,
    features: BTreeSet<Feature>,
    speech: Option<Speech<'a>>,
    bubble: Option<Bubble<'a>>,
    fish_icon: Option<FishIcon>,
    // Keep the displayed cell across shell repaints. The whale chases the live treat.
    fish_cell: Option<(u16, u16)>,
    next_fish_move: Instant,
    grab: Option<WhaleGrab>,
    menu_open: bool,
    painted_menu: Option<Menu>,
    hint_until: Option<Instant>,
    life: Life,
    previous: Option<Image>,
    last_frame: Option<FrameKey>,
    dirty: bool,
}

#[cfg(test)]
impl Default for Pet<'static> {
    fn default() -> Self {
        static CATALOG: std::sync::LazyLock<crate::deepseek_pet::dialogue::Catalog> =
            std::sync::LazyLock::new(|| {
                crate::deepseek_pet::dialogue::Catalog::load(None).unwrap()
            });
        Self::new(
            &CATALOG,
            crate::deepseek_pet::Options::default().enabled_features(),
        )
    }
}

impl<'a> Pet<'a> {
    pub fn new(
        catalog: &'a crate::deepseek_pet::dialogue::Catalog,
        features: BTreeSet<Feature>,
    ) -> Self {
        use std::hash::BuildHasher;
        let choice = std::collections::hash_map::RandomState::new().hash_one(()) as u32;
        Self {
            screen: None,
            kitty: None,
            image_only: false,
            life: Life::new(choice, &features),
            catalog,
            speech: features.contains(&Feature::Speech).then(Speech::default),
            features,
            bubble: None,
            fish_icon: None,
            fish_cell: None,
            next_fish_move: Instant::now(),
            grab: None,
            menu_open: false,
            painted_menu: None,
            hint_until: None,
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
        if !self.menu_open && self.features.contains(&Feature::CommandReactions) {
            self.life.notice_command(event.phase);
        }
    }

    pub fn notice_input(&mut self) {
        if !self.menu_open && self.features.contains(&Feature::Typing) {
            self.life.notice_input();
        }
    }

    pub fn notice_bell(&mut self) {
        if !self.menu_open && self.features.contains(&Feature::Bell) {
            self.life.notice_bell();
        }
    }

    pub fn mouse_event(&mut self, kind: MouseKind, col: u16, row: u16) {
        if !self.features.contains(&Feature::Mouse) {
            return;
        }
        let cell = (col, row);
        if self.menu_open {
            match kind {
                MouseKind::LeftPress => {
                    self.menu_open = false;
                    match self.painted_menu.and_then(|menu| menu.action_at(col, row)) {
                        Some(Action::Feed) => {
                            self.life.treat = Some(Treat {
                                cell,
                                state: TreatState::Carried,
                            })
                        }
                        Some(Action::Bubbles) => self.life.play(Mood::Bubbles),
                        Some(Action::Nap) => self.life.play(Mood::Nap),
                        None => {}
                    }
                }
                MouseKind::RightPress => self.menu_open = false,
                _ => {}
            }
            return;
        }
        if let Some(grab) = &mut self.grab {
            match kind {
                MouseKind::LeftDrag | MouseKind::LeftRelease => {
                    grab.moved |= cell != grab.press;
                    if grab.moved {
                        let (dx, dy) = grab.offset;
                        self.life.manual_origin =
                            Some((col.saturating_sub(dx).max(1), row.saturating_sub(dy).max(1)));
                    }
                    if kind == MouseKind::LeftRelease {
                        self.grab = None;
                    }
                    return;
                }
                _ => {}
            }
        }
        let hit = self
            .previous
            .as_ref()
            .filter(|image| image.covers_cell(col, row));
        match (kind, self.life.treat.as_mut()) {
            (MouseKind::LeftPress, Some(treat))
                if treat.state == TreatState::Carried
                    || self.fish_icon.is_some_and(|fish| fish.contains(col, row)) =>
            {
                treat.cell = cell;
                treat.state = TreatState::Dragging;
            }
            (MouseKind::LeftPress, _) => {
                if let Some(image) = hit {
                    self.grab = Some(WhaleGrab {
                        press: cell,
                        offset: (col - image.col, row - image.row),
                        moved: false,
                    });
                    if self.life.mood != Mood::Wave {
                        self.life.react(Mood::Wave);
                    }
                    if !self.image_only {
                        self.hint_until
                            .get_or_insert_with(|| Instant::now() + Duration::from_secs(3));
                    }
                }
            }
            (MouseKind::LeftDrag, Some(treat)) if treat.state != TreatState::Placed => {
                treat.cell = cell;
                treat.state = TreatState::Dragging;
            }
            (MouseKind::LeftRelease, Some(treat)) if treat.state == TreatState::Dragging => {
                treat.cell = cell;
                treat.state = TreatState::Placed;
            }
            (MouseKind::Motion, Some(treat)) if treat.state == TreatState::Carried => {
                treat.cell = cell;
            }
            (MouseKind::Motion | MouseKind::LeftDrag, _) if !self.image_only => {
                self.life.notice_mouse(col, row)
            }
            (MouseKind::RightPress, Some(_)) => self.life.treat = None,
            (MouseKind::RightPress, None) if hit.is_some() && self.image_only => {
                self.life.play(Mood::Bubbles)
            }
            (MouseKind::RightPress, None) if hit.is_some() => {
                self.grab = None;
                self.menu_open = true;
                self.hint_until = Some(Instant::now());
            }
            _ => {}
        }
    }

    pub fn covers(&self, cell: (u16, u16), pixel: Option<(u16, u16)>) -> bool {
        let Some(image) = &self.previous else {
            return false;
        };
        if let Some((x, y)) = pixel {
            let left = usize::from(image.col - 1) * usize::from(image.cell_w);
            let top = usize::from(image.row - 1) * usize::from(image.cell_h);
            let x = usize::from(x - 1).checked_sub(left);
            let y = usize::from(y - 1).checked_sub(top);
            return x.zip(y).is_some_and(|(x, y)| {
                x < image.w && y < image.w && image.pixels[y * image.w + x][3] >= 128
            });
        }
        image.covers_cell(cell.0, cell.1)
    }

    pub fn cancel_mouse_interaction(&mut self) {
        self.grab = None;
        self.life.treat = None;
        self.menu_open = false;
    }

    pub fn fish_refresh_at(&self, now: Instant) -> Option<Instant> {
        let target = self.life.treat.map(|t| t.cell);
        if target == self.fish_cell {
            None
        } else if target.is_some() && self.fish_cell.is_some() {
            Some(self.next_fish_move)
        } else {
            Some(now)
        }
    }

    fn update_fish_cell(&mut self, now: Instant) {
        if self.fish_refresh_at(now).is_some_and(|at| at <= now) {
            self.fish_cell = self.life.treat.map(|t| t.cell);
            self.next_fish_move = now + FISH_FRAME_INTERVAL;
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
        if let Some(old) = self.painted_menu.take() {
            erase_area(old.area, cols, rows, self.screen.as_ref(), out);
        }
        if let Some(old) = self.fish_icon.take() {
            self.erase_fish(old, cols, rows, out);
        }
        if let Some(old) = self.bubble.take() {
            old.erase(cols, rows, self.screen.as_ref(), out);
        }
        let Some(old) = self.previous.take() else {
            return;
        };
        self.last_frame = None;
        if let Some(kitty) = self.kitty {
            kitty.whale.delete(out).expect("Vec write");
        } else {
            old.clear_uncovered(None, cols, rows, self.screen.as_ref(), out);
        }
    }

    fn erase_fish(&self, fish: FishIcon, cols: u16, rows: u16, out: &mut Vec<u8>) {
        if let Some(kitty) = self.kitty {
            kitty.fish.delete(out).expect("Vec write");
        } else {
            fish.erase(cols, rows, self.screen.as_ref(), out);
        }
    }

    fn paint_fish(&mut self, fish: Option<FishIcon>, out: &mut Vec<u8>) -> io::Result<()> {
        if let Some(icon) = fish {
            icon.draw(self.kitty.map(|k| &k.fish), out)?;
        }
        self.fish_icon = fish;
        Ok(())
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
        let band = if self.kitty.is_some() { 1 } else { 6 };
        let h = (height / 3)
            .clamp(48, MAX_SIDE)
            .min(width)
            .min(height / band * band);
        if h == 0 {
            self.cancel_mouse_interaction();
            self.fish_cell = None;
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
            .size_for_pixels(w as u32, h.next_multiple_of(band) as u32)
            .unwrap();
        let travel = cols - 1 - size.width;
        let rise = rows - size.height;
        if self.menu_open {
            self.life.pause_at(tick);
        }
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
        if self.grab.is_none() && self.features.contains(&Feature::Roam) {
            self.life.manual_origin = None;
        }
        let whale = Area::new((col - 1, row - 1).into(), (size.width, size.height).into());
        let menu = self
            .menu_open
            .then(|| Menu::place(whale, cols, rows))
            .flatten();
        self.menu_open = menu.is_some();
        if self.painted_menu != menu {
            if let Some(old) = self.painted_menu.take() {
                erase_area(old.area, cols, rows, self.screen.as_ref(), out);
            }
            self.dirty = true;
        }
        let bubble_size = (size.width, size.height);
        let bubble = if self.image_only || self.menu_open {
            None
        } else if self.hint_until.is_some_and(|until| Instant::now() < until) {
            Bubble::place("右键点我，一起玩", col, row, bubble_size, cols, rows)
        } else {
            self.speech
                .as_mut()
                .and_then(|speech| speech.place(col, row, bubble_size, cols, rows))
        };
        self.update_fish_cell(Instant::now());
        let fish = self
            .fish_cell
            .map(|cell| FishIcon::place(cell, cols, rows, metrics, band));
        let fish_changed = self.fish_icon != fish;
        if fish_changed {
            if let Some(old) = self.fish_icon.take() {
                self.erase_fish(old, cols, rows, out);
                if self.kitty.is_none() {
                    let erased = old.erase_area();
                    self.dirty |= erased.intersects(&whale)
                        || bubble.is_some_and(|b| erased.intersects(&b.area))
                        || menu.is_some_and(|m| erased.intersects(&m.area));
                }
            }
        }
        if self.bubble != bubble {
            if let Some(old) = self.bubble.take() {
                old.erase(cols, rows, self.screen.as_ref(), out);
            }
            self.dirty = true;
        }
        let leftward = self.life.leftward;
        let bite = if self.life.mood == Mood::EatFish {
            FishBite::at(self.life.age)
        } else {
            None
        };
        let key = FrameKey {
            origin: (col, row),
            cell_pixels: (cell_w, cell_h),
            side: w,
            frame,
            bite,
            leftward,
        };
        if !self.dirty && self.last_frame.as_ref() == Some(&key) {
            if fish_changed {
                self.paint_fish(fish, out)?;
            }
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
                let c = if let Some(ink) = bite.and_then(|b| b.pixel(sx, sy)) {
                    ink
                } else {
                    tile[i..i + 4].try_into().unwrap()
                };
                pixels.push(c);
            }
        }
        // Kitty placements sit above terminal text. Leave room for the menu
        // when a small window forces it to overlap the whale.
        if let Some(menu) = menu.filter(|m| self.kitty.is_some() && whale.intersects(&m.area)) {
            let left = usize::from(menu.area.x().saturating_sub(whale.x())) * usize::from(cell_w);
            let right = usize::from(menu.area.real_size().width - whale.x()) * usize::from(cell_w);
            let top = usize::from(menu.area.y().saturating_sub(whale.y())) * usize::from(cell_h);
            let bottom =
                usize::from(menu.area.real_size().height - whale.y()) * usize::from(cell_h);
            for line in pixels.chunks_exact_mut(w).take(bottom).skip(top) {
                line[left.min(w)..right.min(w)].fill([0; 4]);
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
        if !self.dirty && !fish_changed && self.previous.as_ref() == Some(&image) {
            return Ok(());
        }
        let repaint_fish = fish_changed || self.dirty || self.kitty.is_none();
        self.dirty = false;
        if let Some(kitty) = self.kitty {
            write!(out, "\x1b[{row};{col}H")?;
            kitty.whale.draw(
                out,
                w as u16,
                h as u16,
                &image.pixels,
                if self.image_only { i32::MAX } else { 0 },
            )?;
        } else {
            if let Some(old) = &self.previous {
                old.clear_uncovered(Some(&image), cols, rows, self.screen.as_ref(), out);
            }
            sixel::draw(col, row, w, &image.pixels, out);
        }
        self.previous = Some(image);
        if let Some(b) = bubble {
            b.render(out)?;
        }
        self.bubble = bubble;
        if repaint_fish {
            self.paint_fish(fish, out)?;
        }
        if let Some(menu) = menu {
            menu.render(out)?;
        }
        self.painted_menu = menu;
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
    fn mouse_enables_petting_and_dragging_on_both_screens() {
        let catalog = crate::deepseek_pet::dialogue::Catalog::default();
        for image_only in [false, true] {
            for enabled in [false, true] {
                let features = if enabled {
                    [Feature::Mouse].into()
                } else {
                    BTreeSet::new()
                };
                let mut pet = Pet::new(&catalog, features);
                pet.image_only = image_only;
                pet.draw(0, 80, 24, 10, 20, &mut Vec::new()).unwrap();
                let origin = pet.last_frame.as_ref().unwrap().origin;
                pet.mouse_event(MouseKind::LeftPress, origin.0 + 6, origin.1 + 3);
                pet.mouse_event(MouseKind::LeftDrag, 20, 10);
                pet.mouse_event(MouseKind::LeftRelease, 20, 10);
                pet.draw(1, 80, 24, 10, 20, &mut Vec::new()).unwrap();
                assert_eq!(
                    pet.last_frame.as_ref().unwrap().origin,
                    if enabled { (14, 7) } else { origin }
                );
                assert_eq!(pet.life.mood == Mood::Wave, enabled);
            }
        }
    }

    #[test]
    fn dragging_a_fixed_overlay_releases_the_covered_text() {
        let kitty = KittySprites {
            whale: rsille::render::raster_output::KittyImage::new().unwrap(),
            fish: rsille::render::raster_output::KittyImage::new().unwrap(),
        };
        let catalog = crate::deepseek_pet::dialogue::Catalog::default();
        let mut pet = Pet::new(&catalog, [Feature::Mouse].into());
        pet.kitty = Some(&kitty);
        pet.image_only = true;
        pet.draw(0, 80, 24, 10, 20, &mut Vec::new()).unwrap();
        let (col, row) = pet.last_frame.as_ref().unwrap().origin;
        let hit = (col + 6, row + 3);
        assert!(pet.covers(hit, None));
        assert!(!pet.covers((col, row), None));
        pet.mouse_event(MouseKind::LeftPress, hit.0, hit.1);
        pet.mouse_event(MouseKind::LeftDrag, 16, 8);
        pet.mouse_event(MouseKind::LeftRelease, 16, 8);
        for tick in [1, 2, 42] {
            pet.draw(tick, 80, 24, 10, 20, &mut Vec::new()).unwrap();
            assert_eq!(pet.last_frame.as_ref().unwrap().origin, (10, 5));
            assert!(!pet.covers(hit, None));
        }
        pet.mouse_event(MouseKind::RightPress, 16, 8);
        assert_eq!(pet.life.mood, Mood::Bubbles);
        assert!(!pet.menu_open);
    }

    #[test]
    fn kitty_pet_and_treat_use_independent_images_and_hide_without_erasing_text() {
        use rsille::render::raster_output::KittyImage;
        let kitty = KittySprites {
            whale: KittyImage::new().unwrap(),
            fish: KittyImage::new().unwrap(),
        };
        let catalog = crate::deepseek_pet::dialogue::Catalog::default();
        let mut pet = Pet::new(&catalog, [Feature::Mouse].into());
        pet.kitty = Some(&kitty);
        let mut bytes = Vec::new();
        pet.draw(0, 80, 24, 10, 20, &mut bytes).unwrap();
        assert!(std::str::from_utf8(&bytes).unwrap().contains("f=32"));
        bytes.clear();
        choose_fish(&mut pet, 1);
        pet.mouse_event(MouseKind::Motion, 2, 2);
        pet.draw(1, 80, 24, 10, 20, &mut bytes).unwrap();
        let output = std::str::from_utf8(&bytes).unwrap();
        assert_eq!(output.matches("\x1b_Ga=p,").count(), 2);
        assert!(!output.contains("\x1bP"));
        bytes.clear();
        pet.hide(80, 24, &mut bytes);
        let mut expected = Vec::new();
        kitty.fish.delete(&mut expected).unwrap();
        kitty.whale.delete(&mut expected).unwrap();
        assert_eq!(bytes, expected);
    }

    #[test]
    fn moving_and_hiding_the_pet_and_bubble_restores_shell_text() {
        let catalog = crate::deepseek_pet::dialogue::Catalog::load(None).unwrap();
        let mut pet = Pet::new(&catalog, [Feature::Speech, Feature::Roam].into());
        let backing = crate::deepseek_pet::screen::Screen::new(24, 80, (1, 1)).unwrap();
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
        let catalog = crate::deepseek_pet::dialogue::Catalog::default();
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
                    let mut screen =
                        crate::deepseek_pet::screen::Screen::new(40, 120, (1, 1)).unwrap();
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
        let catalog = crate::deepseek_pet::dialogue::Catalog::load(None).unwrap();
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

    fn open_menu(pet: &mut Pet<'_>, tick: u64) -> Area {
        let (col, row) = pet.last_frame.as_ref().unwrap().origin;
        pet.mouse_event(MouseKind::RightPress, col + 6, row + 3);
        pet.draw(tick, 80, 24, 10, 20, &mut Vec::new()).unwrap();
        pet.painted_menu.unwrap().area
    }

    fn choose_fish(pet: &mut Pet<'_>, tick: u64) {
        let menu = open_menu(pet, tick);
        let (col, row) = (menu.x() + 3, menu.y() + 2);
        pet.mouse_event(MouseKind::LeftPress, col, row);
        pet.mouse_event(MouseKind::LeftRelease, col, row);
    }

    #[test]
    fn fish_motion_coalesces_until_due_and_stops_when_idle() {
        let mut pet = Pet::default();
        let start = Instant::now();
        pet.draw(0, 80, 24, 10, 20, &mut Vec::new()).unwrap();
        choose_fish(&mut pet, 0);
        pet.mouse_event(MouseKind::Motion, 3, 2);
        assert_eq!(pet.fish_refresh_at(start), Some(start));
        pet.update_fish_cell(start);
        let deadline = start + FISH_FRAME_INTERVAL;
        for col in 4..12 {
            pet.mouse_event(MouseKind::Motion, col, 2);
            assert_eq!(pet.fish_refresh_at(start), Some(deadline));
            pet.update_fish_cell(deadline - Duration::from_nanos(1));
            assert_eq!(pet.fish_cell, Some((3, 2)));
        }
        pet.update_fish_cell(deadline);
        assert_eq!(pet.fish_cell, Some((11, 2)));
        assert_eq!(pet.fish_refresh_at(deadline), None);
        pet.mouse_event(MouseKind::RightPress, 11, 2);
        pet.update_fish_cell(deadline);
        assert_eq!(pet.fish_cell, None);
        assert_eq!(pet.fish_refresh_at(deadline), None);
    }

    #[test]
    fn moving_only_the_fish_repaints_the_whale_only_to_repair_sixel_overlap() {
        use rsille::render::raster_output::KittyImage;
        let kitty = KittySprites {
            whale: KittyImage::new().unwrap(),
            fish: KittyImage::new().unwrap(),
        };
        let catalog = crate::deepseek_pet::dialogue::Catalog::default();
        for backend in [Some(&kitty), None] {
            let mut pet = Pet::new(&catalog, [Feature::Mouse].into());
            pet.kitty = backend;
            pet.draw(0, 80, 24, 10, 20, &mut Vec::new()).unwrap();
            choose_fish(&mut pet, 0);
            pet.mouse_event(MouseKind::Motion, 2, 2);
            pet.draw(0, 80, 24, 10, 20, &mut Vec::new()).unwrap();
            pet.mouse_event(MouseKind::LeftPress, 2, 2);
            for (col, row, erased_whale) in [(5, 3, false), (70, 20, false), (8, 5, true)] {
                pet.mouse_event(MouseKind::LeftDrag, col, row);
                pet.update_fish_cell(pet.next_fish_move);
                let mut bytes = Vec::new();
                pet.draw(0, 80, 24, 10, 20, &mut bytes).unwrap();
                let output = std::str::from_utf8(&bytes).unwrap();
                if backend.is_some() {
                    assert_eq!(output.matches("\x1b_Ga=p,").count(), 1);
                    assert!(output.contains("z=1,"));
                } else {
                    assert!(output.contains("q\"1;1;30;15#"));
                    assert_eq!(output.contains("q\"1;1;160;160#"), erased_whale);
                }
                assert!(pet.fish_icon.unwrap().contains(col, row));
                assert_eq!(pet.life.age, 0);
            }
        }
    }

    #[test]
    fn menu_pauses_roaming_and_dismisses_without_feeding() {
        let mut pet = Pet::default();
        pet.life.mood = Mood::Swim;
        pet.draw(0, 80, 24, 10, 20, &mut Vec::new()).unwrap();
        let origin = pet.last_frame.as_ref().unwrap().origin;
        let area = open_menu(&mut pet, 1);
        pet.mouse_event(MouseKind::Motion, 1, 1);
        pet.notice_command(b'+', pet.catalog.commands["ls"]);
        pet.notice_bell();
        pet.notice_input();
        pet.hide(80, 24, &mut Vec::new());
        pet.draw(200, 80, 24, 10, 20, &mut Vec::new()).unwrap();
        assert_eq!(pet.last_frame.as_ref().unwrap().origin, origin);
        assert_eq!(pet.life.mood, Mood::Swim);
        assert_eq!(pet.painted_menu.unwrap().area, area);
        assert!(pet.bubble.is_none());
        pet.mouse_event(MouseKind::LeftPress, 1, 1);
        pet.mouse_event(MouseKind::LeftRelease, 1, 1);
        pet.draw(201, 80, 24, 10, 20, &mut Vec::new()).unwrap();
        assert!(!pet.menu_open && pet.painted_menu.is_none() && pet.life.treat.is_none());
        assert_eq!(pet.life.age, 1);
    }

    #[test]
    fn menu_actions_play_and_carry_or_cancel_a_treat() {
        let catalog = crate::deepseek_pet::dialogue::Catalog::default();
        let mut pet = Pet::new(&catalog, [Feature::Mouse].into());
        pet.draw(0, 80, 24, 10, 20, &mut Vec::new()).unwrap();
        let (col, row) = pet.last_frame.as_ref().unwrap().origin;
        let body = (col + 6, row + 3);
        pet.mouse_event(MouseKind::LeftPress, body.0, body.1);
        pet.draw(0, 80, 24, 10, 20, &mut Vec::new()).unwrap();
        assert_eq!(pet.bubble.unwrap().text, "右键点我，一起玩");
        pet.hint_until = Some(Instant::now());
        pet.mouse_event(MouseKind::LeftPress, body.0, body.1);
        pet.draw(0, 80, 24, 10, 20, &mut Vec::new()).unwrap();
        assert!(pet.bubble.is_none());

        for (offset, mood) in [(1, Mood::Bubbles), (2, Mood::Nap)] {
            let menu = open_menu(&mut pet, 1);
            pet.mouse_event(MouseKind::LeftPress, menu.x() + 3, menu.y() + 2 + offset);
            pet.draw(1, 80, 24, 10, 20, &mut Vec::new()).unwrap();
            assert_eq!(pet.life.mood, mood);
            assert!(pet.painted_menu.is_none() && pet.life.treat.is_none());
        }

        pet.draw(60, 80, 24, 10, 20, &mut Vec::new()).unwrap();
        for cancel in [true, false] {
            choose_fish(&mut pet, 61);
            pet.mouse_event(MouseKind::Motion, 2, 2);
            pet.draw(61, 80, 24, 10, 20, &mut Vec::new()).unwrap();
            assert!(pet.fish_icon.unwrap().contains(2, 2));
            assert!(pet.life.treat.unwrap().state == TreatState::Carried);
            if cancel {
                pet.mouse_event(MouseKind::RightPress, 2, 2);
                assert!(pet.life.treat.is_none() && !pet.menu_open);
            } else {
                pet.mouse_event(MouseKind::Motion, body.0, body.1);
                pet.draw(62, 80, 24, 10, 20, &mut Vec::new()).unwrap();
                assert_eq!(pet.life.mood, Mood::EatFish);
                assert!(pet.life.treat.is_none() && pet.fish_icon.is_none());
            }
        }
    }

    #[test]
    fn menu_restores_text_and_releases_the_whale_if_resized_too_small() {
        let mut pet = Pet::default();
        pet.draw(0, 80, 24, 10, 20, &mut Vec::new()).unwrap();
        let area = open_menu(&mut pet, 0);
        let (x, y) = (area.x() + 1, area.y() + 2);
        let shell = format!("\x1b[{y};{x}H\x1b[32;45m鲸鱼在这里");
        pet.screen = crate::deepseek_pet::screen::Screen::new(24, 80, (1, 1));
        pet.screen.as_mut().unwrap().feed(shell.as_bytes());
        let mut visible = vt100::Parser::new(24.try_into().unwrap(), 80.try_into().unwrap(), 0);
        visible.process(shell.as_bytes());
        let expected = visible.screen().clone();
        pet.invalidate();
        let mut out = Vec::new();
        pet.draw(1, 80, 24, 10, 20, &mut out).unwrap();
        visible.process(&out);
        assert!(visible.screen().contents().contains("喂小鱼干"));
        for row in area.y()..area.real_size().height {
            for col in area.x()..area.real_size().width {
                let cell = visible.screen().cell(row, col).unwrap();
                assert_eq!(cell.bgcolor(), vt100::Color::Default);
            }
        }
        let label = visible.screen().cell(y - 1, area.x() + 2).unwrap();
        assert_eq!(label.fgcolor(), vt100::Color::Default);
        out.clear();
        pet.hide(80, 24, &mut out);
        visible.process(&out);
        for col in area.x()..area.real_size().width {
            assert_eq!(visible.screen().cell(y - 1, col), expected.cell(y - 1, col));
        }
        pet.draw(2, 8, 3, 10, 20, &mut Vec::new()).unwrap();
        assert!(!pet.menu_open && pet.painted_menu.is_none());
    }

    #[test]
    fn command_gestures_work_without_speech_mouse_or_roaming() {
        let catalog = crate::deepseek_pet::dialogue::Catalog::load(None).unwrap();
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
        let catalog = crate::deepseek_pet::dialogue::Catalog::default();
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
        let mut screen = crate::deepseek_pet::screen::Screen::new(24, 80, (1, 1)).unwrap();
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
        choose_fish(&mut pet, 0);
        pet.mouse_event(MouseKind::LeftPress, 40, 8);
        draw(&mut pet, 1);
        assert!(pet.fish_icon.unwrap().contains(40, 8));
        pet.mouse_event(MouseKind::LeftDrag, 21, 8);
        pet.notice_food(1, 1);
        let caught = (2..66)
            .find(|&tick| {
                draw(&mut pet, tick);
                pet.life.mood == Mood::EatFish
            })
            .expect("the whale should catch the fish while the button is still down");
        assert!(pet.life.treat.is_none() && pet.fish_icon.is_none());
        assert_eq!(pet.life.foraging.as_mut().unwrap().tick(), None);
        let gold_pixels = |pet: &Pet<'_>| {
            pet.previous
                .as_ref()
                .unwrap()
                .pixels
                .iter()
                .filter(|&&c| sixel::index(c) == 0b111_101_01)
                .count()
        };
        let whole_fish = gold_pixels(&pet);
        pet.mouse_event(MouseKind::LeftDrag, 22, 8);
        pet.mouse_event(MouseKind::LeftRelease, 22, 8);
        draw(&mut pet, caught + 13);
        let bitten_fish = gold_pixels(&pet);
        draw(&mut pet, caught + 16);
        assert!(whole_fish > bitten_fish && bitten_fish > gold_pixels(&pet));
        draw(&mut pet, caught + u64::from(animation::SNACK.duration));
        assert_eq!(pet.life.mood, Mood::Idle);
        assert!(pet.life.treat.is_none() && pet.fish_icon.is_none());
        let mut out = Vec::new();
        pet.hide(80, 24, &mut out);
        visible.process(&out);
        for col in 19..23 {
            assert_eq!(visible.screen().cell(7, col), expected.cell(7, col));
        }
    }

    #[test]
    fn fixed_whale_can_be_hand_fed_and_distant_treats_can_be_retrieved() {
        let catalog = crate::deepseek_pet::dialogue::Catalog::default();
        let mut pet = Pet::new(&catalog, [Feature::Mouse].into());
        let mut out = Vec::new();
        pet.draw(0, 80, 24, 10, 20, &mut out).unwrap();
        let origin = pet.last_frame.as_ref().unwrap().origin;
        choose_fish(&mut pet, 0);
        pet.mouse_event(MouseKind::LeftPress, 1, 1);
        pet.mouse_event(MouseKind::LeftRelease, 1, 1);
        pet.draw(40, 80, 24, 10, 20, &mut out).unwrap();
        assert!(pet.fish_icon.unwrap().contains(1, 1));
        assert_eq!(pet.last_frame.as_ref().unwrap().origin, origin);
        pet.mouse_event(MouseKind::RightPress, 10, 2);
        pet.draw(41, 80, 24, 10, 20, &mut out).unwrap();
        assert!(pet.fish_icon.is_none());
        choose_fish(&mut pet, 41);
        pet.mouse_event(MouseKind::LeftPress, 2, 2);
        pet.mouse_event(MouseKind::LeftRelease, origin.0 + 6, origin.1 + 3);
        pet.draw(42, 80, 24, 10, 20, &mut out).unwrap();
        assert_eq!(pet.life.mood, Mood::EatFish);
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
        for (x, y) in [(1, 1), (col, row)] {
            pet.mouse_event(MouseKind::LeftPress, x, y);
            pet.mouse_event(MouseKind::LeftDrag, x, y);
            pet.mouse_event(MouseKind::LeftRelease, x, y);
            assert!(pet.life.treat.is_none());
        }
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
                    sixel::index(image.pixels[py * image.w + px]) == 0b100_111_11
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
    fn whale_catches_the_pointer_at_hand_height_and_holds_the_pose() {
        let mut pet = Pet::default();
        pet.life.pointer.as_mut().unwrap().curiosity = 0;
        let mut out = Vec::new();
        pet.draw(0, 80, 24, 10, 20, &mut out).unwrap();
        pet.mouse_event(MouseKind::Motion, 60, 14);
        pet.draw(12, 80, 24, 10, 20, &mut out).unwrap();
        let image = pet.previous.as_ref().unwrap();
        assert_eq!(pet.life.mood, Mood::Grab);
        assert!(image.covers_cell(60, 14));
        assert_eq!(image.row, 14 - 5); // The pointer sits one row below the center.
        let reach = pet.life.frame();
        pet.draw(18, 80, 24, 10, 20, &mut out).unwrap();
        assert_ne!(pet.life.frame(), reach);
        assert_eq!(pet.life.frame(), animation::GRAB.frame(6));
        assert_eq!(pet.previous.as_ref().unwrap().row, 14 - 5);
        pet.draw(
            12 + u64::from(animation::GRAB.duration),
            80,
            24,
            10,
            20,
            &mut out,
        )
        .unwrap();
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
