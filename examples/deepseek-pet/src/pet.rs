use rsille::render::{
    area::{Area, Size},
    chunk::Chunk,
    text::TextMetrics,
    Builder, Draw, DrawErr,
};
use rsille::tui::{
    layout::border_renderer::{border_chars, render_border},
    style::{BorderStyle, Color, Style},
};
use std::{
    fmt::Write,
    io::{self, Cursor},
    sync::OnceLock,
};

const TILE: usize = 320;
const ATLAS_WIDTH: usize = 4 * TILE;
const MAX_SIDE: usize = 192;

// The 4-by-6 sheet is decoded once. Rows within a tile retain the atlas stride.
fn atlas_tile(action: usize, pose: usize) -> &'static [u8] {
    static PIXELS: OnceLock<Vec<u8>> = OnceLock::new();
    let pixels = PIXELS.get_or_init(|| {
        let png = include_bytes!("../../ai-babies/assets/deepseek-actions-320x320.png");
        let mut reader = png::Decoder::new(Cursor::new(png))
            .read_info()
            .expect("embedded DeepSeek atlas");
        let mut rgba = vec![0; ATLAS_WIDTH * 6 * TILE * 4];
        let info = reader
            .next_frame(&mut rgba)
            .expect("embedded DeepSeek pixels");
        assert_eq!(
            (info.width, info.height),
            (ATLAS_WIDTH as u32, (6 * TILE) as u32)
        );
        assert_eq!(
            (info.color_type, info.bit_depth),
            (png::ColorType::Rgba, png::BitDepth::Eight)
        );
        rgba
    });
    &pixels[(action * TILE * ATLAS_WIDTH + pose * TILE) * 4..]
}

/// Return (column, row, SGR-colored text) runs for a cleared viewport.
/// Transparent cells and the rightmost column are never painted.
pub fn frame(tick: u64, cols: u16, rows: u16) -> Vec<(u16, u16, String)> {
    let usable = usize::from(cols.saturating_sub(1));
    let rows = usize::from(rows);
    let h = 64
        .min(rows.saturating_sub(rows / 6))
        .min(usable.div_ceil(2));
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
    let tile = atlas_tile(action, pose);
    for y in 0..h {
        let mut run = String::new();
        let mut start = 0;
        let mut color = None;
        for x in 0..w {
            let sx = (2 * x + 1) * TILE / (2 * w);
            let sx = if returning { TILE - 1 - sx } else { sx };
            let sy = (2 * y + 1) * TILE / (2 * h);
            let i = (sy * ATLAS_WIDTH + sx) * 4;
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

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
enum Mood {
    Swim,
    Watch,
    Hop,
    Swoop,
    Bubbles,
    Wave,
    Startled,
    Snack,
    Nap,
    #[default]
    Idle,
}

#[derive(Default)]
struct Life {
    mood: Mood,
    age: u16,
    fatigue: u16,
    hunger: u16,
    cooldown: u16,
    bell_cooldown: u16,
    choice: u32,
    leftward: bool,
    x: u16,
    altitude: u16,
    curiosity: u32,
    pursuit: u16,
    mouse: Option<(u16, u16)>,
    food: Option<(u16, u16)>,
    full: u16,
    last: Option<u64>,
}

impl Life {
    fn react(&mut self, mood: Mood) {
        self.altitude = self.altitude.saturating_add(self.pose().2);
        self.mood = mood;
        self.age = 0;
        self.mouse = None;
        self.pursuit = 0;
    }

    fn update(
        &mut self,
        tick: u64,
        travel: u16,
        rise: u16,
        width: u16,
        height: u16,
    ) -> Option<(u16, u16)> {
        let mut eaten = None;
        self.altitude = self.altitude.min(rise);
        let visible =
            |&(c, r): &(u16, u16)| c > 0 && r > 0 && c <= travel + width && r <= rise + height;
        self.food = self.food.filter(visible);
        self.mouse = self.mouse.filter(visible);
        if self.last.is_none() {
            self.x = travel;
        }
        self.x = self.x.min(travel);
        // A suspended session resumes near its last pose, without unbounded catch-up.
        let elapsed = tick
            .saturating_sub(self.last.replace(tick).unwrap_or(tick))
            .min(128);
        for _ in 0..elapsed {
            self.age += 1;
            self.cooldown = self.cooldown.saturating_sub(1);
            self.bell_cooldown = self.bell_cooldown.saturating_sub(1);
            self.hunger = self.hunger.saturating_add(1);
            self.full = self.full.saturating_sub(1);
            if self.pursuit == 0 {
                self.mouse = None;
            }
            self.pursuit = self.pursuit.saturating_sub(1);
            let meal = self.food.filter(|_| self.full == 0 && self.hunger >= 30);
            let reacting = matches!(self.mood, Mood::Wave | Mood::Startled);
            if let Some((c, r)) = meal.or(self.mouse).filter(|_| !reacting) {
                let x = (c - 1).saturating_sub(width / 2).min(travel);
                let altitude = rise - (r - 1).saturating_sub(height / 2).min(rise);
                if self.x != x {
                    self.leftward = x < self.x;
                }
                self.x = approach(self.x, x);
                self.altitude = approach(self.altitude, altitude);
                self.mood = Mood::Swim;
                self.age %= 16;
                if (self.x, self.altitude) == (x, altitude) {
                    self.age = 0;
                    self.mouse = None;
                    if meal.is_some() {
                        eaten = self.food.take();
                        self.full = 120;
                        self.hunger = 0;
                        self.mood = Mood::Snack;
                    } else {
                        self.mood = Mood::Watch;
                    }
                }
                continue;
            }
            if !reacting && !matches!(self.mood, Mood::Snack | Mood::Idle) {
                self.altitude = self.altitude.saturating_sub(1);
            }
            if matches!(self.mood, Mood::Swim | Mood::Swoop) {
                self.fatigue = self.fatigue.saturating_add(1);
                if self.mood == Mood::Swoop || self.age % 2 == 0 {
                    if self.x == 0 || self.x == travel {
                        self.leftward = self.x != 0;
                    }
                    self.x = if self.leftward {
                        self.x.saturating_sub(1)
                    } else {
                        self.x.saturating_add(1).min(travel)
                    };
                }
            }
            let duration = match self.mood {
                Mood::Swim => 48 + (self.choice % 49) as u16,
                Mood::Nap => 48,
                Mood::Watch | Mood::Hop => 16,
                Mood::Wave | Mood::Bubbles => 20,
                Mood::Startled => 12,
                Mood::Swoop => 40,
                Mood::Idle => 70 + (self.choice % 81) as u16,
                Mood::Snack => 24,
            };
            if self.age == duration {
                if self.mood == Mood::Nap {
                    self.fatigue = 0;
                }
                if self.mood == Mood::Snack {
                    self.hunger = 0;
                }
                self.choice = self.choice.wrapping_mul(1664525).wrapping_add(1013904223);
                self.mood = if self.fatigue >= 160 {
                    Mood::Nap
                } else if self.mood == Mood::Idle {
                    let actions = [
                        Mood::Swim,
                        Mood::Watch,
                        Mood::Swoop,
                        Mood::Hop,
                        Mood::Bubbles,
                    ];
                    actions[(self.choice >> 16) as usize % actions.len()]
                } else {
                    Mood::Idle
                };
                self.age = 0;
            }
        }
        eaten
    }

    fn pose(&self) -> (usize, usize, u16) {
        let t = usize::from(self.age);
        match self.mood {
            Mood::Swim => (2, t / 4 % 4, (t / 6 % 2) as u16),
            Mood::Bubbles => (0, usize::from(t < 3) * 2, 0),
            Mood::Wave => (1, [0, 1, 2, 3, 2, 3, 2, 1, 0, 0][t / 2], 0),
            Mood::Startled => (3, [1, 2, 2, 2, 3, 0][t / 2], [0, 3, 5, 4, 2, 0][t / 2]),
            Mood::Swoop => {
                let phase = t as f32 / 39.0;
                let amplitude = 5 + self.choice % 9;
                (
                    2,
                    t / 4 % 4,
                    ((phase * std::f32::consts::PI).sin() * amplitude as f32).round() as u16,
                )
            }
            Mood::Hop => (
                3,
                [0, 1, 2, 2, 2, 2, 3, 0][t / 2],
                [0, 0, 1, 3, 3, 1, 0, 0][t / 2],
            ),
            Mood::Watch => (4, [0, 1, 2, 3, 2, 1, 0, 0][t / 2], 0),
            Mood::Snack => (1, [0, 1, 3, 2, 3, 0][t / 4], 0),
            Mood::Nap => (5, [0, 1, 2, 3, 2, 1][t / 8], 0),
            Mood::Idle => match (t + (self.choice % 21) as usize) % 79 {
                32 | 33 => (0, 2, 0),
                60..=62 if self.choice & 1 != 0 => (1, 1, 0),
                _ => (0, 0, 0),
            },
        }
    }

    // Effects are authored on a 96-by-48 logical grid, independent of tile resolution.
    fn effect_color(&self, x: usize, y: usize) -> Option<u8> {
        let t = usize::from(self.age);
        match self.mood {
            Mood::Wave => {
                let top = 13 - t / 2;
                let heart = [
                    0b0110110u8,
                    0b1111111,
                    0b1111111,
                    0b0111110,
                    0b0011100,
                    0b0001000,
                ];
                let bit = x.checked_sub(70)? / 2;
                let row = heart.get(y.checked_sub(top)?)?;
                (bit < 7 && row & (1 << bit) != 0).then_some(0b111_011_10)
            }
            Mood::Startled
                if t < 9 && (74..=76).contains(&x) && ((6..=10).contains(&y) || y == 13) =>
            {
                Some(0b111_110_01)
            }
            Mood::Bubbles => {
                for delay in [0, 4, 8] {
                    if let Some(age @ 0..=13) = t.checked_sub(delay) {
                        let cx = 78 + age / 2 + delay / 2;
                        let cy = 25 - age * 3 / 2;
                        let d = x.abs_diff(cx).pow(2) + (2 * y.abs_diff(cy)).pow(2);
                        if (7..=17).contains(&d) {
                            return Some(0b100_111_11);
                        }
                    }
                }
                None
            }
            Mood::Snack if t >= 16 => {
                let d = x.abs_diff(80).pow(2) + y.abs_diff(24 - (t - 16) * 2).pow(2);
                (9..=16).contains(&d).then_some(255)
            }
            _ => None,
        }
    }
}

fn approach(value: u16, goal: u16) -> u16 {
    if value < goal {
        value + 1
    } else if value > goal {
        value - 1
    } else {
        value
    }
}

#[derive(PartialEq)]
struct FrameKey {
    origin: (u16, u16),
    cell_pixels: (u16, u16),
    side: usize,
    action: usize,
    pose: usize,
    leftward: bool,
    effect: Option<(Mood, u16)>,
}

#[derive(Clone, Copy, PartialEq)]
struct Bubble {
    text: &'static str,
    area: Area,
    above: bool,
    tail: u16,
}

impl Bubble {
    fn place(
        text: &'static str,
        col: u16,
        row: u16,
        size: (u16, u16),
        cols: u16,
        rows: u16,
    ) -> Option<Self> {
        let width = u16::try_from(TextMetrics::default().width(text))
            .ok()?
            .checked_add(4)?;
        if width >= cols {
            return None;
        }
        let room_above = row - 1;
        let room_below = rows - (room_above + size.1);
        let height = if room_above.max(room_below) >= 3 {
            3
        } else {
            1
        };
        let above = room_above >= height;
        let row = if above {
            row - height
        } else if room_below >= height {
            row + size.1
        } else {
            return None;
        };
        let center = col + size.0 / 2;
        let col = center.saturating_sub(width / 2).clamp(1, cols - width);
        Some(Self {
            text,
            area: Area::new((col - 1, row - 1).into(), (width, height).into()),
            above,
            tail: (center - col).clamp(1, width - 2),
        })
    }

    fn erase(self, cols: u16, rows: u16, out: &mut Vec<u8>) {
        use std::io::Write;
        let viewport = Area::from(Size::from((cols, rows)));
        let Some(visible) = self.area.clamp_to(&viewport) else {
            return;
        };
        let col = visible.x() + 1;
        let width = visible.width();
        out.extend_from_slice(b"\x1b[49m");
        for y in visible.y()..visible.y() + visible.height() {
            write!(out, "\x1b[{};{col}H\x1b[{width}X", y + 1).unwrap();
        }
    }

    fn render(self, out: &mut Vec<u8>) -> io::Result<()> {
        Builder::new()
            .size(self.area.size().into())
            .pos(self.area.pos().into())
            .clear(false)
            .build_render(self, out)
            .render()
    }
}

impl Draw for Bubble {
    fn draw(&mut self, mut chunk: Chunk<'_>) -> Result<Size, DrawErr> {
        let size = chunk.area().size();
        let border = BorderStyle::Rounded;
        let outline = Style::default().fg(Color::Indexed(116)).to_render_style();
        let lettering = Style::default().fg(Color::Indexed(195)).to_render_style();
        if size.height == 1 {
            let side = border_chars(&chunk, border).vertical;
            chunk.set_char(0, 0, side, outline)?;
            chunk.set_char(size.width - 1, 0, side, outline)?;
        } else {
            render_border(&mut chunk, border, outline);
            let (row, tip) = if self.above {
                (size.height - 1, '▾')
            } else {
                (0, '▴')
            };
            chunk.set_char(self.tail, row, tip, outline)?;
        }
        chunk.set_string(2, size.height / 2, self.text, lettering)?;
        Ok(size)
    }
}

pub struct Pet {
    dialogue: crate::dialogue::Dialogue,
    speech: Option<(&'static str, std::time::Instant)>,
    bubble: Option<Bubble>,
    life: Life,
    previous: Option<Image>,
    last_frame: Option<FrameKey>,
    dirty: bool,
}

impl Default for Pet {
    fn default() -> Self {
        use std::hash::BuildHasher;
        let choice = std::collections::hash_map::RandomState::new().hash_one(()) as u32;
        Self {
            life: Life {
                choice,
                curiosity: choice,
                ..Life::default()
            },
            dialogue: Default::default(),
            speech: None,
            bubble: None,
            previous: None,
            last_frame: None,
            dirty: false,
        }
    }
}

impl Pet {
    pub fn notice_command(&mut self, phase: u8, command: u8) {
        if (phase, command) == (b'e', b'0') {
            self.speech = None;
            return;
        }
        let mood = match phase {
            b'e' => Mood::Watch,
            b'r' => Mood::Swim,
            b'+' => Mood::Wave,
            b'-' => Mood::Startled,
            b'i' => Mood::Idle,
            _ => return,
        };
        let Some(text) = self.dialogue.choose(phase, command) else {
            return;
        };
        self.life.react(mood);
        self.speech = Some((
            text,
            std::time::Instant::now() + std::time::Duration::from_secs(3),
        ));
    }

    pub fn notice_touch(&mut self, col: u16, row: u16) {
        if self.life.mood != Mood::Wave
            && self
                .previous
                .as_ref()
                .is_some_and(|image| image.covers_cell(col, row))
        {
            self.life.react(Mood::Wave);
        }
    }

    pub fn notice_bell(&mut self) {
        if self.life.bell_cooldown == 0 && self.life.mood != Mood::Wave {
            self.life.react(Mood::Startled);
            self.life.bell_cooldown = 60;
        }
    }

    /// Notice a mouse position in one-based terminal cells.
    pub fn notice_mouse(&mut self, col: u16, row: u16) {
        if self.life.mouse.is_some() {
            self.life.mouse = Some((col, row));
        } else if self.life.cooldown == 0
            && matches!(self.life.mood, Mood::Swim | Mood::Idle | Mood::Watch)
        {
            self.life.curiosity = self
                .life
                .curiosity
                .wrapping_mul(1664525)
                .wrapping_add(1013904223);
            self.life.cooldown = 5;
            if self.life.curiosity >> 31 == 0 {
                self.life.pursuit = 32;
                self.life.cooldown = 48;
                self.life.mouse = Some((col, row));
            }
        }
    }

    /// Replace the latest edible output cell (one-based terminal coordinates).
    pub fn notice_food(&mut self, col: u16, row: u16) {
        self.life.food = Some((col, row));
    }

    pub fn forget_food(&mut self) {
        self.life.food = None;
    }
    /// Occasionally look up. Key repeats cannot restart the gesture.
    pub fn notice_input(&mut self) {
        if self.life.cooldown == 0 && matches!(self.life.mood, Mood::Idle | Mood::Swim) {
            self.life.mood = Mood::Watch;
            self.life.age = 0;
            self.life.cooldown = 120;
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
        let (action, pose, lift) = self.life.pose();
        let col = 1 + self.life.x;
        let row = 1 + (rise - self.life.altitude).saturating_sub(lift);
        self.speech = self
            .speech
            .filter(|&(_, until)| std::time::Instant::now() < until);
        let bubble = self.speech.and_then(|(text, _)| {
            Bubble::place(text, col, row, (size.width, size.height), cols, rows)
        });
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
            action,
            pose,
            leftward,
            effect: match self.life.mood {
                Mood::Snack | Mood::Wave | Mood::Startled | Mood::Bubbles => {
                    Some((self.life.mood, self.life.age))
                }
                _ => None,
            },
        };
        if !self.dirty && self.last_frame.as_ref() == Some(&key) {
            return Ok(());
        }
        self.last_frame = Some(key);
        let mut pixels = Vec::with_capacity(w * h);
        let mut palette = [false; 256];
        let tile = atlas_tile(action, pose);
        for y in 0..h {
            let sy = (2 * y + 1) * TILE / (2 * h);
            let effect_y = (2 * y + 1) * 48 / (2 * h);
            for x in 0..w {
                let x = if leftward { w - 1 - x } else { x };
                let sx = (2 * x + 1) * TILE / (2 * w);
                let i = (sy * ATLAS_WIDTH + sx) * 4;
                let c = if let Some(color) =
                    self.life.effect_color((2 * x + 1) * 96 / (2 * w), effect_y)
                {
                    palette[usize::from(color)] = true;
                    u16::from(color)
                } else if tile[i + 3] < 128 {
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

    #[test]
    fn bubble_points_at_the_whale_and_clears_only_visible_rows() {
        for (row, top, tail) in [(1, 3, '▴'), (7, 4, '▾')] {
            let bubble = Bubble::place("你好", 17, row, (3, 2), 20, 8).unwrap();
            assert_eq!(bubble.area, Area::new((11, top - 1).into(), (8, 3).into()));
            let mut out = Vec::new();
            bubble.render(&mut out).unwrap();
            let drawing = String::from_utf8(out).unwrap();
            assert!(drawing.contains(tail));
            assert_eq!(drawing.matches('─').count(), 11); // two six-cell edges, one pointer
            let mut erase = Vec::new();
            bubble.erase(15, top + 1, &mut erase);
            assert_eq!(
                erase,
                format!("\x1b[49m\x1b[{top};12H\x1b[4X\x1b[{};12H\x1b[4X", top + 1).as_bytes()
            );
        }
        assert_eq!(
            Bubble::place("你好", 1, 2, (3, 2), 20, 4)
                .unwrap()
                .area
                .height(),
            1
        );
    }

    #[test]
    fn bubble_uses_framework_text_metrics_and_renders_only_its_rectangle() {
        // A CJK glyph, an ASCII letter with a combining accent, and an emoji cluster.
        let message = "鲸e\u{301}👩‍💻";
        let bubble = Bubble::place(message, 5, 7, (3, 2), 20, 10).unwrap();
        assert_eq!(bubble.area.width(), 9);
        let mut bytes = Vec::new();
        bubble.render(&mut bytes).unwrap();
        let output = String::from_utf8(bytes).unwrap();
        assert!(output.contains(message));
        assert!(!output.contains("\x1b[2J") && !output.contains('\n'));
    }

    #[test]
    fn command_bubble_tracks_edits_results_and_available_space() {
        let mut pet = Pet::default();
        let mut out = Vec::new();
        pet.notice_command(b'e', b't');
        let edit = pet.speech.unwrap().0;
        pet.draw(0, 80, 24, 10, 20, &mut out).unwrap();
        let first = pet.bubble.unwrap();
        assert_eq!(first.text, edit);
        assert!(first.area.real_size().width < 80);
        assert_eq!(
            first.area.real_size().height + 1,
            pet.previous.as_ref().unwrap().row
        );
        pet.notice_command(b'e', b'0');
        out.clear();
        pet.draw(0, 80, 24, 10, 20, &mut out).unwrap();
        let mut erase = Vec::new();
        first.erase(80, 24, &mut erase);
        assert!(out.starts_with(&erase));
        assert!(pet.bubble.is_none());
        pet.notice_command(b'r', b't');
        pet.notice_command(b'+', b't');
        assert_eq!(pet.life.mood, Mood::Wave);
        pet.draw(1, 5, 2, 10, 20, &mut out).unwrap();
        assert!(pet.bubble.is_none());
        pet.draw(1, 80, 24, 10, 20, &mut out).unwrap();
        assert_eq!(pet.bubble.unwrap().text, pet.speech.unwrap().0);
        assert_ne!(pet.bubble.unwrap().text, edit);
        pet.speech.as_mut().unwrap().1 = std::time::Instant::now();
        pet.draw(2, 80, 24, 10, 20, &mut out).unwrap();
        assert!(pet.bubble.is_none());
    }

    #[test]
    fn idle_gestures_leave_the_whale_in_place() {
        for (choice, duration) in [(0, 70), (1, 71), (80, 150)] {
            let mut life = Life {
                choice,
                altitude: 5,
                ..Life::default()
            };
            let mut still = 0;
            let mut blinked = false;
            let mut moved_fin = false;
            for tick in 0..duration {
                life.update(tick, 60, 10, 1, 1);
                assert_eq!((life.mood, life.x, life.altitude), (Mood::Idle, 60, 5));
                match life.pose() {
                    (0, 0, 0) => still += 1,
                    (0, 2, 0) => blinked = true,
                    (1, 1, 0) => moved_fin = true,
                    other => panic!("unexpected idle pose {other:?}"),
                }
            }
            assert!(still > duration * 9 / 10 && blinked);
            assert_eq!(moved_fin, choice & 1 != 0);
            life.update(duration, 60, 10, 1, 1);
            assert_ne!(life.mood, Mood::Idle);
        }
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
        pet.life.hunger = 100;
        pet.notice_food(1, 1);
        for tick in 1..20 {
            pet.notice_touch(col + 6, row + 3);
            pet.notice_bell();
            pet.life.update(tick, 66, 17, 13, 7);
            assert_eq!((pet.life.mood, pet.life.age), (Mood::Wave, tick as u16));
        }
        pet.life.update(20, 66, 17, 13, 7);
        assert_eq!(pet.life.mood, Mood::Idle);
    }

    #[test]
    fn bell_wakes_a_sleeping_pet_and_repeated_bells_allow_it_to_land() {
        let mut pet = Pet::default();
        pet.life.mood = Mood::Nap;
        for tick in 0..=12 {
            pet.notice_bell();
            pet.life.update(tick, 5, 5, 1, 1);
            if tick == 4 {
                assert_eq!(pet.life.pose().2, 5);
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
    fn undisturbed_whale_blows_rising_bubbles() {
        let mut life = Life::default();
        for tick in 0..2000 {
            life.update(tick, 60, 10, 1, 1);
            if life.mood == Mood::Bubbles {
                assert!(life.effect_color(82, 25).is_some());
                life.update(tick + 10, 60, 10, 1, 1);
                assert!(life.effect_color(87, 10).is_some());
                life.update(tick + 20, 60, 10, 1, 1);
                assert_ne!(life.mood, Mood::Bubbles);
                return;
            }
        }
        panic!("the whale never blew bubbles");
    }

    #[test]
    fn fed_pet_uses_every_action_and_stays_in_bounds() {
        let mut life = Life::default();
        let mut seen = [false; 6];
        for tick in 0..12000 {
            let travel = (tick / 31 % 8) as u16;
            if tick % 600 == 0 {
                life.food = Some((1, 1));
            }
            life.update(tick, travel, 0, 1, 1);
            let (action, pose, _) = life.pose();
            seen[action] = true;
            assert!(life.x <= travel && pose < 4);
        }
        assert_eq!(seen, [true; 6]);
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
        pet.life.curiosity = 0;
        for tick in 0..=33 {
            pet.notice_mouse(1 + tick as u16, 1);
            pet.life.update(tick, 200, 0, 1, 1);
        }
        assert!(pet.life.mouse.is_none());
        assert!(pet.life.cooldown > 0);
    }

    #[test]
    fn swoop_travels_while_rising_and_falling() {
        let mut life = Life {
            mood: Mood::Swoop,
            ..Life::default()
        };
        life.update(0, 100, 20, 1, 1);
        life.update(20, 100, 20, 1, 1);
        assert_eq!(life.x, 80);
        assert!(life.pose().2 >= 5);
        life.update(39, 100, 20, 1, 1);
        assert_eq!((life.x, life.pose().2), (61, 0));
    }

    #[test]
    fn targets_follow_both_axes_and_stale_food_is_not_eaten() {
        let mut pet = Pet::default();
        pet.life.curiosity = 0;
        pet.notice_mouse(12, 2);
        pet.life.update(0, 20, 10, 1, 1);
        pet.life.update(9, 20, 10, 1, 1);
        assert_eq!((pet.life.x, pet.life.altitude), (11, 9));
        pet.life.hunger = 100;
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
