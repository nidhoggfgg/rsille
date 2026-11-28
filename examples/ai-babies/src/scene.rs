use crossterm::style::Color;

pub const WIDTH: usize = 128;
pub const HEIGHT: usize = 64;
pub const FRAME_BYTES: usize = WIDTH * HEIGHT * 4;
pub const SPRITES: &[u8; FRAME_BYTES * 24] =
    include_bytes!("../assets/chatgpt-actions-v2-128x64.rgba");
pub const SMALL_WIDTH: usize = 96;
pub const SMALL_HEIGHT: usize = 48;
pub const SMALL_SPRITES: &[u8; SMALL_WIDTH * SMALL_HEIGHT * 4 * 24] =
    include_bytes!("../assets/chatgpt-actions-v2-96x48.rgba");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Character {
    #[default]
    ChatGpt,
    DeepSeek,
    Claude,
}

impl Character {
    pub const ALL: [Self; 3] = [Self::ChatGpt, Self::DeepSeek, Self::Claude];

    pub fn name(self) -> &'static str {
        match self {
            Self::ChatGpt => "chatgpt",
            Self::DeepSeek => "deepseek",
            Self::Claude => "claude",
        }
    }

    pub fn parse(name: &str) -> Result<Self, &'static str> {
        Self::ALL
            .into_iter()
            .find(|c| c.name() == name)
            .ok_or("Unknown character; use chatgpt/deepseek/claude")
    }

    pub fn next(self) -> Self {
        match self {
            Self::ChatGpt => Self::DeepSeek,
            Self::DeepSeek => Self::Claude,
            Self::Claude => Self::ChatGpt,
        }
    }

    pub fn atlas(self, display_height: usize) -> (&'static [u8], usize) {
        const DEEPSEEK: &[u8; FRAME_BYTES * 24] =
            include_bytes!("../assets/deepseek-actions-128x64.rgba");
        const DEEPSEEK_SMALL: &[u8; SMALL_WIDTH * SMALL_HEIGHT * 4 * 24] =
            include_bytes!("../assets/deepseek-actions-96x48.rgba");
        const CLAUDE: &[u8; FRAME_BYTES * 24] =
            include_bytes!("../assets/claude-actions-128x64.rgba");
        const CLAUDE_SMALL: &[u8; SMALL_WIDTH * SMALL_HEIGHT * 4 * 24] =
            include_bytes!("../assets/claude-actions-96x48.rgba");
        if display_height <= SMALL_HEIGHT {
            (
                match self {
                    Self::ChatGpt => SMALL_SPRITES,
                    Self::DeepSeek => DEEPSEEK_SMALL,
                    Self::Claude => CLAUDE_SMALL,
                },
                SMALL_HEIGHT,
            )
        } else {
            (
                match self {
                    Self::ChatGpt => SPRITES,
                    Self::DeepSeek => DEEPSEEK,
                    Self::Claude => CLAUDE,
                },
                HEIGHT,
            )
        }
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

pub fn render(
    w: usize,
    h: usize,
    time: f64,
    action: Option<Action>,
    character: Character,
) -> Vec<Option<[u8; 3]>> {
    let mut pixels = vec![None; w * h];
    if w == 0 || h == 0 {
        return pixels;
    }
    // Quadrant samples are half as wide as they are tall in 1:2 terminal cells.
    // Rectangular atlases preserve the artwork's physical aspect and finer detail.
    let dh = h.min(w / 2).min(HEIGHT);
    let dw = dh * 2;
    let (sprites, source_size) = character.atlas(dh);
    let left = (w - dw) / 2;
    let top = (h - dh) / 2;
    let source_width = source_size * 2;
    let frame = current_frame(time, action) * source_width * source_size * 4;
    for dy in 0..dh {
        for dx in 0..dw {
            let sx = (2 * dx + 1) * source_width / (2 * dw);
            let sy = (2 * dy + 1) * source_size / (2 * dh);
            let offset = frame + (sy * source_width + sx) * 4;
            let rgba = &sprites[offset..offset + 4];
            // Packed alpha is binary. Leave transparent samples absent so the
            // terminal supplies its own background, including at quadrant edges.
            if rgba[3] != 0 {
                pixels[(top + dy) * w + left + dx] = Some([rgba[0], rgba[1], rgba[2]]);
            }
        }
    }
    pixels
}
