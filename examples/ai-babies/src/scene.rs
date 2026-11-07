use crossterm::style::Color;

pub const WIDTH: usize = 64;
pub const HEIGHT: usize = 64;
pub const FRAME_BYTES: usize = WIDTH * HEIGHT * 4;
pub const SPRITES: &[u8; FRAME_BYTES * 24] = include_bytes!("../assets/dragon-actions-v2-64.rgba");
pub const SMALL_WIDTH: usize = 48;
pub const SMALL_SPRITES: &[u8; SMALL_WIDTH * SMALL_WIDTH * 4 * 24] =
    include_bytes!("../assets/dragon-actions-v2-48.rgba");

pub fn atlas(display_height: usize) -> (&'static [u8], usize) {
    if display_height <= SMALL_WIDTH {
        (SMALL_SPRITES, SMALL_WIDTH)
    } else {
        (SPRITES, WIDTH)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Idle,
    Wave,
    Walk,
    Hop,
    Think,
    Sleep,
}

// (atlas column, duration in milliseconds). Drawings and holds are authored;
// runtime never reconstructs eyes, bends limbs or translates a still image.
const IDLE: &[(usize, u32)] = &[(0, 900), (1, 600), (2, 150), (3, 750)];
const WAVE: &[(usize, u32)] = &[
    (0, 300),
    (1, 200),
    (2, 200),
    (3, 200),
    (2, 200),
    (3, 200),
    (1, 200),
    (0, 500),
];
const WALK: &[(usize, u32)] = &[
    (0, 180),
    (1, 180),
    (2, 180),
    (3, 180),
    (0, 180),
    (1, 180),
    (2, 180),
    (3, 180),
];
const HOP: &[(usize, u32)] = &[(0, 500), (1, 180), (2, 320), (3, 180), (0, 500)];
const THINK: &[(usize, u32)] = &[
    (0, 400),
    (1, 300),
    (2, 700),
    (3, 700),
    (2, 500),
    (1, 300),
    (0, 400),
];
const SLEEP: &[(usize, u32)] = &[
    (0, 700),
    (1, 600),
    (2, 900),
    (3, 900),
    (2, 900),
    (3, 900),
    (1, 600),
    (0, 700),
];

impl Action {
    pub const ALL: [Self; 6] = [
        Self::Idle,
        Self::Wave,
        Self::Walk,
        Self::Hop,
        Self::Think,
        Self::Sleep,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Wave => "wave",
            Self::Walk => "walk",
            Self::Hop => "hop",
            Self::Think => "think",
            Self::Sleep => "sleep",
        }
    }

    const fn clip(self) -> &'static [(usize, u32)] {
        match self {
            Self::Idle => IDLE,
            Self::Wave => WAVE,
            Self::Walk => WALK,
            Self::Hop => HOP,
            Self::Think => THINK,
            Self::Sleep => SLEEP,
        }
    }

    pub const fn duration(self) -> f64 {
        let clip = self.clip();
        let mut ms = 0;
        let mut i = 0;
        while i < clip.len() {
            ms += clip[i].1;
            i += 1;
        }
        ms as f64 / 1000.0
    }

    pub fn frame(self, time: f64) -> usize {
        let mut ms = (time.rem_euclid(self.duration()) * 1000.0 + 1e-7).floor() as u32;
        for &(column, hold) in self.clip() {
            if ms < hold {
                return self as usize * 4 + column;
            }
            ms -= hold;
        }
        self as usize * 4
    }
}

pub const PERIOD: f64 = Action::Idle.duration()
    + Action::Wave.duration()
    + Action::Walk.duration()
    + Action::Hop.duration()
    + Action::Think.duration()
    + Action::Sleep.duration();

pub fn current_frame(time: f64, action: Option<Action>) -> usize {
    if let Some(action) = action {
        return action.frame(time);
    }
    let mut time = time.rem_euclid(PERIOD);
    for action in Action::ALL {
        if time + 1e-9 < action.duration() {
            return action.frame(time);
        }
        time -= action.duration();
    }
    Action::Idle.frame(0.0)
}

pub fn color([r, g, b]: [u8; 3]) -> Color {
    Color::Rgb { r, g, b }
}

pub fn render(w: usize, h: usize, time: f64, action: Option<Action>) -> Vec<Option<[u8; 3]>> {
    let mut pixels = vec![None; w * h];
    if w == 0 || h == 0 {
        return pixels;
    }
    // Half-block samples are physically square with typical 1:2 terminal cells.
    // 24- and 32-row displays use atlases baked directly from the original art.
    let dh = h.min(w).min(HEIGHT);
    let dw = dh;
    let (sprites, source_size) = atlas(dh);
    let left = (w - dw) / 2;
    let top = (h - dh) / 2;
    let frame = current_frame(time, action) * source_size * source_size * 4;
    for dy in 0..dh {
        for dx in 0..dw {
            let sx = (2 * dx + 1) * source_size / (2 * dw);
            let sy = (2 * dy + 1) * source_size / (2 * dh);
            let offset = frame + (sy * source_size + sx) * 4;
            let rgba = &sprites[offset..offset + 4];
            // Packed alpha is binary. Leave transparent samples absent so the
            // terminal supplies its own background, including at half-cell edges.
            if rgba[3] != 0 {
                pixels[(top + dy) * w + left + dx] = Some([rgba[0], rgba[1], rgba[2]]);
            }
        }
    }
    pixels
}
