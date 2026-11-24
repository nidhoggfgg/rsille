mod behavior;
mod fish;
mod kitty;
mod menu;
mod motion;
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
use motion::{Motion, Movement};

use rsille::render::area::{Area, Position, Size};
use speech::{Bubble, Speech};
use std::{
    collections::BTreeSet,
    fmt::Write,
    io,
    rc::Rc,
    time::{Duration, Instant},
};

pub(super) const FRAMES_PER_TICK: u64 = 3;
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
    out.extend_from_slice(b"\x1b[0m");
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
    offset: Position,
    w: usize,
    pixels: Rc<[[u8; 4]]>,
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
        let (width, height) = self.cell_extent();
        let width = usize::from(width).min(usize::from(cols - self.col) + 1);
        let mut dirty = vec![false; width];
        let start = out.len();
        for dy in 0..height.min(rows - self.row + 1) {
            let row = self.row + dy;
            for (dx, dirty) in dirty.iter_mut().enumerate() {
                let col = self.col + dx as u16;
                let left = usize::from(col - 1) * cw;
                let top = usize::from(row - 1) * ch;
                *dirty = self.ink_in_cell(col, row, |x, y| {
                    !same_grid.is_some_and(|n| n.opaque_at(left + x, top + y))
                });
            }
            let mut col = usize::from(self.col);
            for run in dirty.chunk_by(|a, b| a == b) {
                if run[0] {
                    if out.len() == start {
                        out.extend_from_slice(b"\x1b[0m");
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

    fn cell_extent(&self) -> (u16, u16) {
        (
            (usize::from(self.offset.x) + self.w).div_ceil(usize::from(self.cell_w)) as u16,
            (usize::from(self.offset.y) + self.w).div_ceil(usize::from(self.cell_h)) as u16,
        )
    }

    fn opaque_at(&self, x: usize, y: usize) -> bool {
        let left =
            usize::from(self.col - 1) * usize::from(self.cell_w) + usize::from(self.offset.x);
        let top = usize::from(self.row - 1) * usize::from(self.cell_h) + usize::from(self.offset.y);
        x >= left
            && y >= top
            && x - left < self.w
            && y - top < self.w
            && self.pixels[(y - top) * self.w + x - left][3] >= 128
    }

    fn near_cell(&self, col: u16, row: u16) -> bool {
        let (width, height) = self.cell_extent();
        // Leave two columns and one row around the displayed sprite for interaction.
        (self.col.saturating_sub(2)..=self.col.saturating_add(width).saturating_add(1))
            .contains(&col)
            && (self.row.saturating_sub(1)..=self.row.saturating_add(height)).contains(&row)
    }

    fn covers_cell(&self, col: u16, row: u16) -> bool {
        self.ink_in_cell(col, row, |_, _| true)
    }

    /// Visit only sprite pixels in this cell, passing their offsets within the cell.
    fn ink_in_cell(&self, col: u16, row: u16, mut visit: impl FnMut(usize, usize) -> bool) -> bool {
        let (Some(dx), Some(dy)) = (col.checked_sub(self.col), row.checked_sub(self.row)) else {
            return false;
        };
        let (cw, ch) = (usize::from(self.cell_w), usize::from(self.cell_h));
        let (left, top) = (usize::from(dx) * cw, usize::from(dy) * ch);
        let (ox, oy) = (usize::from(self.offset.x), usize::from(self.offset.y));
        for y in top.max(oy)..(top + ch).min(oy + self.w) {
            for x in left.max(ox)..(left + cw).min(ox + self.w) {
                if self.pixels[(y - oy) * self.w + x - ox][3] >= 128 && visit(x - left, y - top) {
                    return true;
                }
            }
        }
        false
    }
}

#[derive(PartialEq)]
struct FrameKey {
    offset: Position,
    origin: (u16, u16),
    cell_pixels: (u16, u16),
    side: usize,
    frame: usize,
    bite: Option<FishBite>,
    leftward: bool,
}

pub use kitty::KittySprites;

struct WhaleGrab {
    press: (u16, u16),
    offset: (u16, u16),
    moved: bool,
}

pub struct Pet<'a> {
    pub kitty: Option<&'a KittySprites>,
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
    motion: Motion,
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
            life: Life::new(choice, &features),
            motion: Motion::default(),
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
                    self.hint_until
                        .get_or_insert_with(|| Instant::now() + Duration::from_secs(3));
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
            (MouseKind::Motion | MouseKind::LeftDrag, _) => {
                let distant = self
                    .previous
                    .as_ref()
                    .is_some_and(|image| !image.near_cell(col, row));
                self.life.notice_mouse(cell, distant);
            }
            (MouseKind::RightPress, Some(_)) => self.life.treat = None,
            (MouseKind::RightPress, None) if hit.is_some() => {
                self.grab = None;
                self.menu_open = true;
                self.hint_until = Some(Instant::now());
            }
            _ => {}
        }
    }

    // Dismiss transient UI while the underlying application receives its own click.
    pub fn dismiss_outside(&mut self, kind: MouseKind) {
        if self.menu_open && matches!(kind, MouseKind::LeftPress | MouseKind::RightPress) {
            self.menu_open = false;
        } else if kind == MouseKind::RightPress {
            self.life.treat = None;
        }
    }

    pub fn captures_press(
        &self,
        kind: MouseKind,
        cell: (u16, u16),
        pixel: Option<(u16, u16)>,
    ) -> bool {
        match kind {
            MouseKind::LeftPress | MouseKind::RightPress => {
                let on_menu = self.menu_open
                    && self.painted_menu.is_some_and(|m| {
                        m.area.contains_area(&Area::new(
                            (cell.0 - 1, cell.1 - 1).into(),
                            (1, 1).into(),
                        ))
                    });
                let handles_treat = self.life.treat.is_some_and(|t| {
                    t.state == TreatState::Carried
                        || self.fish_icon.is_some_and(|f| f.contains(cell.0, cell.1))
                });
                on_menu || handles_treat || self.covers(cell, pixel)
            }
            _ => false,
        }
    }

    pub fn covers(&self, cell: (u16, u16), pixel: Option<(u16, u16)>) -> bool {
        let Some(image) = &self.previous else {
            return false;
        };
        if let Some((x, y)) = pixel {
            return x > 0 && y > 0 && image.opaque_at(usize::from(x - 1), usize::from(y - 1));
        }
        image.covers_cell(cell.0, cell.1)
    }

    pub fn cancel_mouse_interaction(&mut self) {
        if let Some(pointer) = &mut self.life.pointer {
            pointer.target = None;
        }
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
            self.erase_panel(old.area, cols, rows, out);
        }
        if let Some(old) = self.fish_icon.take() {
            self.erase_fish(old, cols, rows, out);
        }
        if let Some(old) = self.bubble.take() {
            self.erase_panel(old.area, cols, rows, out);
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

    fn erase_panel(&self, area: Area, cols: u16, rows: u16, out: &mut Vec<u8>) {
        if let Some(kitty) = self.kitty {
            kitty.panel.delete(out).expect("Vec write");
        } else {
            erase_area(area, cols, rows, self.screen.as_ref(), out);
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

    pub fn moving(&self, frame: u64) -> bool {
        self.motion.moving(frame)
    }

    #[cfg(test)]
    fn draw(
        &mut self,
        tick: u64,
        cols: u16,
        rows: u16,
        cell_w: u16,
        cell_h: u16,
        out: &mut Vec<u8>,
    ) -> io::Result<()> {
        self.draw_frame(
            tick * FRAMES_PER_TICK + FRAMES_PER_TICK - 1,
            cols,
            rows,
            cell_w,
            cell_h,
            out,
        )
    }

    pub fn draw_frame(
        &mut self,
        display_frame: u64,
        cols: u16,
        rows: u16,
        cell_w: u16,
        cell_h: u16,
        out: &mut Vec<u8>,
    ) -> io::Result<()> {
        use rsille::render::raster_output::CellPixels;
        use std::io::Write;
        let tick = display_frame / FRAMES_PER_TICK;
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
            self.motion = Motion::default();
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
            write!(out, "\x1b[49m\x1b[{r};{c}H\x1b[X")?;
            if let Some(screen) = &mut self.screen {
                screen.consume(&out[start..]);
            }
            self.dirty = true;
        }
        let frame = self.life.frame();
        let base_row = rise - self.life.altitude;
        let fraction = (display_frame % FRAMES_PER_TICK) as f32 / FRAMES_PER_TICK as f32;
        let lift = self
            .life
            .lift_at(fraction, base_row)
            .map(|rows| (self.life.mood, (rows * f32::from(cell_h)).round() as u32));
        let (x, y) = self.motion.position(
            display_frame,
            (self.life.x, base_row),
            (cols, rows, cell_w, cell_h),
            if self.menu_open {
                Movement::Paused
            } else if self.life.manual_origin.is_some() {
                Movement::Drag
            } else if self.life.slow_swim_step {
                Movement::SlowSwim
            } else {
                Movement::Step
            },
            lift,
        );
        let col = 1 + (x / u32::from(cell_w)) as u16;
        let y = y.min(u32::from(rise) * u32::from(cell_h));
        let row = 1 + (y / u32::from(cell_h)) as u16;
        // Reserve the last partial Sixel band even at an in-cell origin.
        let available = height - usize::from(row - 1) * usize::from(cell_h);
        let offset = Position {
            x: (x % u32::from(cell_w)) as u16,
            y: ((y % u32::from(cell_h)) as usize).min(available / band * band - h) as u16,
        };
        if self.grab.is_none() && self.features.contains(&Feature::Roam) {
            self.life.manual_origin = None;
        }
        let visible_size = metrics
            .size_for_pixels(
                w as u32 + u32::from(offset.x),
                h as u32 + u32::from(offset.y),
            )
            .unwrap();
        let whale = Area::new((col - 1, row - 1).into(), visible_size);
        let menu = self
            .menu_open
            .then(|| Menu::place(whale, cols, rows))
            .flatten();
        self.menu_open = menu.is_some();
        if self.painted_menu != menu {
            if let Some(old) = self.painted_menu.take() {
                self.erase_panel(old.area, cols, rows, out);
            }
            self.dirty = true;
        }
        let bubble_size = (visible_size.width, visible_size.height);
        let bubble = if self.menu_open {
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
        let repaint_panel = self.dirty || self.bubble != bubble;
        if self.bubble != bubble {
            if let Some(old) = self.bubble.take() {
                self.erase_panel(old.area, cols, rows, out);
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
            offset,
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
        let same_sprite = self.last_frame.as_ref().is_some_and(|old| {
            (old.side, old.frame, old.leftward, old.bite) == (w, frame, leftward, bite)
        });
        let repaint_fish = fish_changed || self.dirty || self.kitty.is_none();
        let pixels = if let Some(old) = self.previous.as_ref().filter(|_| same_sprite) {
            Rc::clone(&old.pixels)
        } else {
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
            pixels.into()
        };
        let image = Image {
            col,
            row,
            offset,
            cell_w,
            cell_h,
            w,
            pixels,
        };
        if let Some(kitty) = self.kitty {
            write!(out, "\x1b[{row};{col}H")?;
            if same_sprite && !self.dirty {
                kitty.whale.place_offset(out, kitty::WHALE_Z, offset)?;
            } else {
                kitty.whale.draw_offset(
                    out,
                    (w as u16, h as u16).into(),
                    &image.pixels,
                    kitty::WHALE_Z,
                    offset,
                )?;
            }
        } else {
            if let Some(old) = &self.previous {
                old.clear_uncovered(Some(&image), cols, rows, self.screen.as_ref(), out);
            }
            sixel::draw_offset(col, row, w, &image.pixels, offset, out);
        }
        self.dirty = false;
        self.last_frame = Some(key);
        self.previous = Some(image);
        if let Some(b) = bubble {
            if let Some(kitty) = self.kitty {
                if repaint_panel {
                    kitty.draw_panel(b.area, b, out)?;
                }
            } else {
                b.render(out)?;
            }
        }
        self.bubble = bubble;
        if repaint_fish {
            self.paint_fish(fish, out)?;
        }
        if let Some(menu) = menu {
            if let Some(kitty) = self.kitty {
                if repaint_panel {
                    kitty.draw_panel(menu.area, menu, out)?;
                }
            } else {
                menu.render(out)?;
            }
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
    fn slow_swim_crosses_a_cell_in_six_pixel_steps_on_both_backends() {
        let kitty = KittySprites::new().unwrap();
        let catalog = Default::default();
        for backend in [None, Some(&kitty)] {
            let mut pet = Pet::new(&catalog, [Feature::Roam].into());
            pet.kitty = backend;
            pet.life.mood = Mood::Swim;
            let mut out = Vec::new();
            pet.draw_frame(0, 80, 24, 12, 20, &mut out).unwrap();
            let start = u32::from(pet.previous.as_ref().unwrap().col - 1) * 12;
            for (step, frame) in (6..12).enumerate() {
                out.clear();
                pet.draw_frame(frame, 80, 24, 12, 20, &mut out).unwrap();
                let image = pet.previous.as_ref().unwrap();
                let x = u32::from(image.col - 1) * 12 + u32::from(image.offset.x);
                assert_eq!(x, start - (step as u32 + 1) * 2);
                assert_eq!(pet.life.age, (frame / FRAMES_PER_TICK) as u16);
                if backend.is_some() && frame == 7 {
                    let command = std::str::from_utf8(&out).unwrap();
                    assert!(command.contains(&format!("X=8,Y={}", image.offset.y)));
                    assert!(command.contains("a=p,") && !command.contains("a=t,"));
                }
                out.clear();
                pet.draw_frame(frame, 80, 24, 12, 20, &mut out).unwrap();
                assert!(out.is_empty());
            }
        }
    }

    fn displayed_y(pet: &Pet<'_>) -> u32 {
        let image = pet.previous.as_ref().unwrap();
        u32::from(image.row - 1) * u32::from(image.cell_h) + u32::from(image.offset.y)
    }

    #[test]
    fn swim_bobs_through_a_whole_row_over_the_full_cycle() {
        let kitty = KittySprites::new().unwrap();
        let catalog = Default::default();
        for backend in [None, Some(&kitty)] {
            let mut pet = Pet::new(&catalog, [Feature::Roam].into());
            pet.kitty = backend;
            pet.life.mood = Mood::Swim;
            let mut heights = Vec::new();
            for frame in 0..=12 * FRAMES_PER_TICK {
                pet.draw_frame(frame, 80, 24, 10, 20, &mut Vec::new())
                    .unwrap();
                heights.push(displayed_y(&pet));
                assert!(pet.moving(frame));
            }
            assert_eq!(heights[0], heights[36]);
            assert_eq!(heights[0] - heights[18], 20);
            assert!(heights[..=18].windows(2).all(|w| w[0] >= w[1]));
            assert!(heights[18..].windows(2).all(|w| w[0] <= w[1]));
            assert!(heights.windows(2).all(|w| w[0].abs_diff(w[1]) <= 2));
            assert!(heights.chunk_by(|a, b| a == b).all(|run| run.len() <= 3));
        }
    }

    #[test]
    fn menu_freezes_the_visible_point_on_the_arc() {
        let mut pet = Pet::default();
        pet.life.mood = Mood::Swoop;
        pet.life.choice = 8; // Exercise the tallest swoop.
        for frame in 0..=11 {
            pet.draw_frame(frame, 80, 24, 10, 20, &mut Vec::new())
                .unwrap();
        }
        let y = displayed_y(&pet);
        let (col, row) = pet.last_frame.as_ref().unwrap().origin;
        pet.mouse_event(MouseKind::RightPress, col + 6, row + 3);
        assert!(pet.menu_open);
        for frame in [12, 40, 63] {
            pet.draw_frame(frame, 80, 24, 10, 20, &mut Vec::new())
                .unwrap();
            assert_eq!(displayed_y(&pet), y);
            assert!(!pet.moving(frame));
        }
        pet.mouse_event(MouseKind::RightPress, 1, 1);
        pet.draw_frame(64, 80, 24, 10, 20, &mut Vec::new()).unwrap();
        assert!(displayed_y(&pet).abs_diff(y) <= 3);
        assert!(pet.moving(64));
    }

    #[test]
    fn shifting_ink_across_a_cell_boundary_updates_hits_and_erases_the_old_cell() {
        let mut image = Image {
            col: 3,
            row: 2,
            cell_w: 4,
            cell_h: 6,
            offset: (3, 5).into(),
            w: 2,
            pixels: Rc::from([[0; 4], [0; 4], [0; 4], [255; 4]]),
        };
        assert!(image.covers_cell(4, 3));
        assert!(!image.covers_cell(3, 2));
        assert!(image.opaque_at(12, 12));
        let mut out = Vec::new();
        let old = Image {
            offset: (2, 4).into(),
            pixels: image.pixels.clone(),
            ..image
        };
        old.clear_uncovered(Some(&image), 80, 24, None, &mut out);
        assert!(std::str::from_utf8(&out).unwrap().contains("\x1b[2;3H"));
        image.offset = old.offset;
        out.clear();
        old.clear_uncovered(Some(&image), 80, 24, None, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn mouse_enables_petting_and_dragging_on_both_backends() {
        let catalog = crate::deepseek_pet::dialogue::Catalog::default();
        let kitty = KittySprites::new().unwrap();
        for backend in [None, Some(&kitty)] {
            for enabled in [false, true] {
                let features = if enabled {
                    [Feature::Mouse].into()
                } else {
                    BTreeSet::new()
                };
                let mut pet = Pet::new(&catalog, features);
                pet.kitty = backend;
                pet.draw(0, 80, 24, 10, 20, &mut Vec::new()).unwrap();
                let origin = pet.last_frame.as_ref().unwrap().origin;
                pet.mouse_event(MouseKind::LeftPress, origin.0 + 6, origin.1 + 3);
                pet.mouse_event(MouseKind::LeftDrag, 20, 10);
                pet.mouse_event(MouseKind::LeftRelease, 20, 10);
                for frame in 3..6 {
                    pet.draw_frame(frame, 80, 24, 10, 20, &mut Vec::new())
                        .unwrap();
                }
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
        let kitty = KittySprites::new().unwrap();
        let catalog = crate::deepseek_pet::dialogue::Catalog::default();
        let mut pet = Pet::new(&catalog, [Feature::Mouse].into());
        pet.kitty = Some(&kitty);
        pet.draw(0, 80, 24, 10, 20, &mut Vec::new()).unwrap();
        let (col, row) = pet.last_frame.as_ref().unwrap().origin;
        let hit = (col + 6, row + 3);
        assert!(pet.covers(hit, None));
        assert!(!pet.covers((col, row), None));
        pet.mouse_event(MouseKind::LeftPress, hit.0, hit.1);
        pet.mouse_event(MouseKind::LeftDrag, 16, 8);
        pet.mouse_event(MouseKind::LeftRelease, 16, 8);
        pet.draw_frame(3, 80, 24, 10, 20, &mut Vec::new()).unwrap();
        for tick in [1, 2, 42] {
            pet.draw(tick, 80, 24, 10, 20, &mut Vec::new()).unwrap();
            assert_eq!(pet.last_frame.as_ref().unwrap().origin, (10, 5));
            assert!(!pet.covers(hit, None));
        }
        pet.mouse_event(MouseKind::RightPress, 16, 8);
        assert!(pet.menu_open);
    }

    #[test]
    fn kitty_pet_and_treat_use_independent_images_and_hide_without_erasing_text() {
        let kitty = KittySprites::new().unwrap();
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
    fn kitty_panels_capture_menu_actions_and_leave_the_child_grid_intact() {
        let kitty = KittySprites::new().unwrap();
        let catalog = crate::deepseek_pet::dialogue::Catalog::load(None).unwrap();
        let mut pet = Pet::new(&catalog, [Feature::Mouse, Feature::Speech].into());
        pet.kitty = Some(&kitty);
        let mut out = Vec::new();
        pet.notice_command(b'+', catalog.commands["ls"]);
        pet.draw(0, 80, 24, 10, 20, &mut out).unwrap();
        assert!(pet.bubble.is_some());
        let (x, y) = pet
            .previous
            .as_ref()
            .map(|w| (w.col + 6, w.row + 3))
            .unwrap();
        for (item, mood) in [(1, Mood::Bubbles), (2, Mood::Nap)] {
            assert!(pet.captures_press(MouseKind::RightPress, (x, y), None));
            pet.mouse_event(MouseKind::RightPress, x, y);
            pet.draw(0, 80, 24, 10, 20, &mut out).unwrap();
            let area = pet.painted_menu.unwrap().area;
            let cell = (area.x() + 3, area.y() + 2 + item);
            assert!(pet.captures_press(MouseKind::LeftPress, cell, None));
            assert!(!pet.captures_press(MouseKind::LeftPress, (1, 1), None));
            pet.mouse_event(MouseKind::LeftPress, cell.0, cell.1);
            assert_eq!(pet.life.mood, mood);
            assert!(!pet.captures_press(MouseKind::LeftPress, cell, None));
            pet.draw(0, 80, 24, 10, 20, &mut out).unwrap();
        }
        pet.hide(80, 24, &mut out);
        let commands = std::str::from_utf8(&out).unwrap();
        assert!(commands.contains("z=2147483647,c=14,r=5,"));
        assert!(commands.is_ascii());
        let mut grid = vt100::Parser::new(24.try_into().unwrap(), 80.try_into().unwrap(), 0);
        for row in 1..=24 {
            grid.process(format!("\x1b[{row};1H{}", "v".repeat(79)).as_bytes());
        }
        let before = grid.screen().contents();
        grid.process(&out);
        assert_eq!(grid.screen().contents(), before);
    }

    #[test]
    fn foraging_erases_one_cell_and_the_child_can_redraw_it() {
        let kitty = KittySprites::new().unwrap();
        let catalog = crate::deepseek_pet::dialogue::Catalog::default();
        for backend in [None, Some(&kitty)] {
            let mut pet = Pet::new(&catalog, [Feature::Forage].into());
            pet.kitty = backend;
            pet.draw(0, 80, 24, 10, 20, &mut Vec::new()).unwrap();
            pet.life.foraging.as_mut().unwrap().hunger = 100;
            pet.notice_food(70, 20);
            let mut out = Vec::new();
            pet.draw(1, 80, 24, 10, 20, &mut out).unwrap();
            assert_eq!(pet.life.mood, Mood::Snack);
            pet.draw(
                1 + u64::from(animation::SNACK.duration),
                80,
                24,
                10,
                20,
                &mut Vec::new(),
            )
            .unwrap();
            assert_eq!(pet.life.mood, Mood::Burp);
            let mut grid = vt100::Parser::new(24.try_into().unwrap(), 80.try_into().unwrap(), 0);
            grid.process(b"\x1b[20;69Habc");
            grid.process(&out);
            grid.process(b"\x1b[1;1Hunrelated");
            assert_eq!(grid.screen().contents_between(19, 68, 19, 71), "a c");
            grid.process(b"\x1b[20;70Hb");
            assert_eq!(grid.screen().contents_between(19, 68, 19, 71), "abc");
        }
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
        // Erased cells must also recover their default attributes when SGR is active.
        visible.process(b"\x1b[1;31;44m");
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
        let kitty = KittySprites::new().unwrap();
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
                    assert!(output.contains(&format!("z={},", kitty::FISH_Z)));
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
        let burp = caught + u64::from(animation::SNACK.duration);
        draw(&mut pet, burp);
        assert_eq!(pet.life.mood, Mood::Burp);
        draw(&mut pet, burp + u64::from(animation::BURP.duration));
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
        erase_area(first.area, 80, 24, None, &mut erase);
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
    fn pursuit_tracks_motion_then_keeps_a_stationary_target_until_caught() {
        let mut pet = Pet::default();
        pet.life.pointer.as_mut().unwrap().curiosity = 0;
        let mut out = Vec::new();
        pet.draw(0, 240, 24, 10, 20, &mut out).unwrap();
        pet.mouse_event(MouseKind::Motion, 1, 1);
        for tick in 1..=65 {
            if tick == 40 {
                pet.mouse_event(MouseKind::Motion, 8, 5);
                pet.notice_bell();
                pet.life
                    .notice_command(crate::deepseek_pet::dialogue::Phase::Running);
                pet.notice_food(220, 1);
            }
            out.clear();
            pet.draw(tick, 240, 24, 10, 20, &mut out).unwrap();
        }
        assert_eq!(pet.life.mood, Mood::Chase);
        assert_eq!(pet.life.pointer.as_ref().unwrap().target, Some((8, 5)));
        for tick in 66..=140 {
            out.clear();
            pet.draw(tick, 240, 24, 10, 20, &mut out).unwrap();
            if pet.life.mood == Mood::Grab {
                break;
            }
        }
        assert_eq!(pet.life.mood, Mood::Grab);
        assert_eq!(pet.life.pointer.as_ref().unwrap().target, None);
    }

    #[test]
    fn nearby_motion_only_updates_an_existing_chase() {
        let mut pet = Pet::default();
        pet.life.pointer.as_mut().unwrap().curiosity = 0;
        pet.draw(0, 80, 24, 10, 20, &mut Vec::new()).unwrap();
        let (col, row) = pet.last_frame.as_ref().unwrap().origin;
        for cell in [(col, row), (col - 2, row - 1), (col + 4, row - 1)] {
            pet.mouse_event(MouseKind::Motion, cell.0, cell.1);
            assert_eq!(pet.life.pointer.as_ref().unwrap().target, None);
        }
        pet.mouse_event(MouseKind::Motion, col - 3, row);
        pet.draw(1, 80, 24, 10, 20, &mut Vec::new()).unwrap();
        assert_eq!(pet.life.mood, Mood::Morph);
        pet.mouse_event(MouseKind::Motion, col, row);
        assert_eq!(pet.life.pointer.as_ref().unwrap().target, Some((col, row)));
        pet.mouse_event(MouseKind::Motion, 81, row);
        pet.draw(2, 80, 24, 10, 20, &mut Vec::new()).unwrap();
        assert_eq!(pet.life.mood, Mood::Unmorph);
    }

    #[test]
    fn targets_follow_both_axes_and_stale_food_is_not_eaten() {
        let mut pet = Pet::default();
        pet.life.pointer.as_mut().unwrap().curiosity = 0;
        pet.life.notice_mouse((12, 2), true);
        pet.life.update(0, 20, 10, 1, 1);
        let transformed = 1 + u64::from(animation::MORPH.duration);
        for tick in 1..transformed {
            pet.life.update(tick, 20, 10, 1, 1);
            assert_eq!(
                (pet.life.mood, pet.life.x, pet.life.altitude),
                (Mood::Morph, 20, 0)
            );
        }
        pet.life.update(transformed, 20, 10, 1, 1);
        assert_eq!(
            (pet.life.mood, pet.life.x, pet.life.altitude),
            (Mood::Chase, 18, 1)
        );
        pet.mouse_event(MouseKind::Motion, 20, 2);
        pet.life.update(transformed + 1, 20, 10, 1, 1);
        assert_eq!(
            (pet.life.mood, pet.life.leftward, pet.life.x),
            (Mood::Chase, false, 19)
        );
        pet.life.update(transformed + 8, 20, 10, 1, 1);
        assert_eq!(
            (pet.life.mood, pet.life.x, pet.life.altitude),
            (Mood::Grab, 19, 9)
        );
        pet.life.foraging.as_mut().unwrap().hunger = 100;
        pet.notice_food(18, 8);
        assert_eq!(pet.life.update(transformed + 10, 20, 10, 1, 1), None);
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
