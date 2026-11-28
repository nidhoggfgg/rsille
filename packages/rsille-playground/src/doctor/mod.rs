//! Terminal graphics capability diagnostics and live multi-backend comparison lab.
//!
//! Probes Kitty, Sixel, cell pixel metrics, TrueColor, shell, and multiplexer
//! environment once at startup, then renders the same animated calibration
//! pattern side-by-side across Braille, HalfBlock, Quadrant, and (when
//! supported) native Kitty/Sixel pixel graphics.

use crossterm::{
    event::{Event, KeyCode, KeyEventKind, KeyModifiers},
    style::{Attribute, Attributes, Color, Colors},
};
use rsille::{
    canvas::{Canvas, Encoding},
    raster::{RasterCanvas, RasterOptions, Rgb},
    render::{
        area::{Area, Size},
        buffer::Buffer,
        chunk::Chunk,
        raster_output::{CellPixels, InlineRasterSession, RasterBackend},
        style::Style,
        Builder, Draw, DrawErr, Update,
    },
};
use std::{
    env,
    error::Error,
    f32::consts::TAU,
    io::{self, IsTerminal},
    time::{Duration, Instant},
};

const INLINE_MAX_HEIGHT: u16 = 28;
const TARGET_INLINE_HEIGHT: u16 = 24;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RasterSupport {
    #[default]
    Unknown,
    None,
    Sixel,
    Kitty,
    Both,
}

impl RasterSupport {
    pub fn has_any(self) -> bool {
        matches!(self, Self::Sixel | Self::Kitty | Self::Both)
    }

    pub fn has_kitty(self) -> bool {
        matches!(self, Self::Kitty | Self::Both)
    }

    pub fn has_sixel(self) -> bool {
        matches!(self, Self::Sixel | Self::Both)
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::Both => "Kitty + Sixel",
            Self::Kitty => "Kitty",
            Self::Sixel => "Sixel",
            Self::None => "Unicode Only",
            Self::Unknown => "Unprobed",
        }
    }
}

/// Snapshot of terminal capabilities collected cleanly before managed input starts.
#[derive(Clone, Debug)]
pub struct TerminalCaps {
    pub grid: (u16, u16),
    pub raster: RasterSupport,
    pub cell_pixels: Option<CellPixels>,
    pub inline_session: Option<InlineRasterSession>,
    pub truecolor: bool,
    pub term: String,
    pub terminal_name: String,
    pub multiplexer: Option<String>,
    pub shell: String,
}

impl Default for TerminalCaps {
    fn default() -> Self {
        Self {
            grid: (100, 28),
            raster: RasterSupport::Unknown,
            cell_pixels: None,
            inline_session: None,
            truecolor: true,
            term: "xterm-256color".to_owned(),
            terminal_name: "Unknown".to_owned(),
            multiplexer: None,
            shell: "shell".to_owned(),
        }
    }
}

fn detect_env_caps() -> TerminalCaps {
    let grid = crossterm::terminal::size().unwrap_or((100, 28));
    let term = env::var("TERM").unwrap_or_else(|_| "unknown".to_owned());
    let colorterm = env::var("COLORTERM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let term_program = env::var("TERM_PROGRAM").ok();
    let terminal_name = if env::var_os("KITTY_WINDOW_ID").is_some() {
        "Kitty".to_owned()
    } else if env::var_os("GHOSTTY_RESOURCES_DIR").is_some() {
        "Ghostty".to_owned()
    } else if env::var_os("WEZTERM_EXECUTABLE").is_some() {
        "WezTerm".to_owned()
    } else if env::var_os("WT_SESSION").is_some() {
        "Windows Terminal".to_owned()
    } else if let Some(prog) = term_program {
        prog
    } else if env::var_os("ALACRITTY_SOCKET").is_some() {
        "Alacritty".to_owned()
    } else {
        term.clone()
    };
    let truecolor = colorterm.contains("truecolor")
        || colorterm.contains("24bit")
        || matches!(
            terminal_name.as_str(),
            "Kitty" | "Ghostty" | "WezTerm" | "Windows Terminal" | "Alacritty" | "iTerm.app"
        );
    let multiplexer = if env::var_os("TMUX").is_some() {
        Some("tmux".to_owned())
    } else if env::var_os("ZELLIJ").is_some() {
        Some("zellij".to_owned())
    } else if env::var_os("STY").is_some() {
        Some("GNU screen".to_owned())
    } else {
        None
    };
    #[cfg(windows)]
    let shell = env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_owned());
    #[cfg(not(windows))]
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_owned());

    TerminalCaps {
        grid,
        raster: RasterSupport::Unknown,
        cell_pixels: CellPixels::from_terminal(),
        inline_session: None,
        truecolor,
        term,
        terminal_name,
        multiplexer,
        shell,
    }
}

fn can_wire_probe(caps: &TerminalCaps) -> bool {
    io::stdin().is_terminal()
        && io::stdout().is_terminal()
        && !caps.term.eq_ignore_ascii_case("dumb")
}

/// Fast startup capability probe used by the inline launcher (single wire round-trip).
pub(crate) fn probe_fast(timeout: Duration) -> TerminalCaps {
    let mut caps = detect_env_caps();
    if !can_wire_probe(&caps) {
        return caps;
    }
    match RasterBackend::Auto.negotiate_inline(None, timeout) {
        Ok(session) => {
            caps.cell_pixels = Some(session.cell_pixels());
            caps.raster = match session {
                InlineRasterSession::Kitty(_) => RasterSupport::Kitty,
                InlineRasterSession::Sixel(_) => RasterSupport::Sixel,
                _ => RasterSupport::None,
            };
            caps.inline_session = Some(session);
        }
        Err(err) if err.kind() == io::ErrorKind::Unsupported => {
            caps.raster = RasterSupport::None;
        }
        Err(_) => {
            caps.raster = RasterSupport::Unknown;
        }
    }
    caps
}

/// Full diagnostic probe testing both Kitty and Sixel independently.
pub fn probe_full(timeout: Duration) -> TerminalCaps {
    let mut caps = detect_env_caps();
    if !can_wire_probe(&caps) {
        return caps;
    }

    let mut has_kitty = false;
    let mut has_sixel = false;
    let mut timed_out = false;

    match RasterBackend::Kitty.negotiate_inline(None, timeout) {
        Ok(session) => {
            has_kitty = true;
            caps.cell_pixels = Some(session.cell_pixels());
            caps.inline_session = Some(session);
        }
        Err(err) if err.kind() == io::ErrorKind::TimedOut => timed_out = true,
        Err(_) => {}
    }

    if !timed_out {
        match RasterBackend::Sixel.negotiate_inline(None, timeout) {
            Ok(session) => {
                has_sixel = true;
                if caps.cell_pixels.is_none() {
                    caps.cell_pixels = Some(session.cell_pixels());
                }
                if caps.inline_session.is_none() {
                    caps.inline_session = Some(session);
                }
            }
            Err(err) if err.kind() == io::ErrorKind::TimedOut => timed_out = true,
            Err(_) => {}
        }
    }

    if !timed_out && caps.cell_pixels.is_none() {
        caps.cell_pixels = CellPixels::probe(timeout).ok();
    }

    caps.raster = match (has_kitty, has_sixel, timed_out) {
        (true, true, _) => RasterSupport::Both,
        (true, false, _) => RasterSupport::Kitty,
        (false, true, _) => RasterSupport::Sixel,
        (false, false, true) => RasterSupport::Unknown,
        (false, false, false) => RasterSupport::None,
    };
    caps
}

/// Formats a crisp copy-pasteable plain-text diagnostic report.
pub fn format_plain_report(caps: &TerminalCaps) -> String {
    let cell_px = match caps.cell_pixels {
        Some(px) => format!("{}×{} px", px.width, px.height),
        None => "unavailable".to_owned(),
    };
    let kitty = if caps.raster.has_kitty() {
        "✓ Supported"
    } else if caps.raster == RasterSupport::Unknown {
        "? Unconfirmed (non-TTY or timed out)"
    } else {
        "✗ Not advertised"
    };
    let sixel = if caps.raster.has_sixel() {
        "✓ Supported"
    } else if caps.raster == RasterSupport::Unknown {
        "? Unconfirmed (non-TTY or timed out)"
    } else {
        "✗ Not advertised"
    };
    let mux = caps.multiplexer.as_deref().unwrap_or("none");
    let color = if caps.truecolor {
        "TrueColor (24-bit RGB)"
    } else {
        "256-color / unverified (set COLORTERM=truecolor if supported)"
    };
    let raster_note = if caps.raster.has_kitty() {
        "✓ Ready (Kitty)"
    } else if caps.raster.has_sixel() {
        "✓ Ready (Sixel)"
    } else {
        "✗ Needs Kitty or Sixel terminal (e.g. Ghostty, Kitty, WezTerm, Windows Terminal 1.22+)"
    };

    let pet_mark = if caps.raster.has_any() { "✓" } else { "·" };
    [
        "rsille doctor · Terminal Graphics & Capability Report".to_owned(),
        "─────────────────────────────────────────────────────────────".to_owned(),
        format!(
            "Terminal       : {} ({}) · {}×{} cells",
            caps.terminal_name, caps.term, caps.grid.0, caps.grid.1
        ),
        format!("Cell Pixels    : {cell_px}"),
        format!("Color Depth    : {color}"),
        format!("Multiplexer    : {mux}"),
        format!("Shell          : {}", caps.shell),
        format!("Kitty Graphics : {kitty}"),
        format!("Sixel Graphics : {sixel}"),
        format!("Raster Status  : {raster_note}"),
        "─────────────────────────────────────────────────────────────".to_owned(),
        "Experience Readiness:".to_owned(),
        "  ✓ rsille voxel      (HalfBlock 3D · Universal)".to_owned(),
        "  ✓ rsille clockwork  (HalfBlock/Braille 3D · Universal)".to_owned(),
        "  ✓ rsille desk       (Floating TUI + FX · Universal)".to_owned(),
        "  ✓ rsille hud        (Live Shell Overlay · Universal)".to_owned(),
        "  ✓ rsille jellyfish  (HalfBlock Aquarium · Universal)".to_owned(),
        format!("  {pet_mark} rsille pet        (Live Shell Pet · Kitty/Sixel)"),
        format!("  {pet_mark} rsille draw       (2D/3D Math REPL · Kitty/Sixel)"),
        "  ✓ rsille doctor     (Diagnostics & 4-Backend Lab · Universal)".to_owned(),
    ]
    .join("\n")
        + "\n"
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pattern {
    SpectrumOrb,
    PrecisionGrid,
    PrismTorus,
    ChromaSwatches,
}

impl Pattern {
    pub const ALL: [Self; 4] = [
        Self::SpectrumOrb,
        Self::PrecisionGrid,
        Self::PrismTorus,
        Self::ChromaSwatches,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::SpectrumOrb => "Spectrum Orb",
            Self::PrecisionGrid => "Precision Lines",
            Self::PrismTorus => "Neon Torus",
            Self::ChromaSwatches => "Chroma Swatches",
        }
    }

    fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|&p| p == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|&p| p == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// Evaluates the calibration test pattern at normalized coordinates `(u, v)` in `[-1, 1]`.
pub(crate) fn sample_pattern(pattern: Pattern, u: f32, v: f32, time: f32) -> [u8; 3] {
    match pattern {
        Pattern::SpectrumOrb => {
            let r2 = u * u + v * v;
            let r = r2.sqrt();
            if r < 0.72 {
                let z = (0.72 * 0.72 - r2).max(0.0).sqrt();
                let nx = u / 0.72;
                let ny = v / 0.72;
                let nz = z / 0.72;
                let lx = (time * 1.1).cos() * 0.65;
                let ly = -0.45;
                let lz = (time * 1.1).sin().abs() * 0.5 + 0.55;
                let l_len = (lx * lx + ly * ly + lz * lz).sqrt();
                let diff = ((nx * lx + ny * ly + nz * lz) / l_len).max(0.0);
                let spec = diff.powf(12.0) * 0.75;
                let angle = v.atan2(u) + time * 0.85;
                let hue = (angle / TAU).rem_euclid(1.0);
                let base = hsv_to_rgb(hue, 0.78, 0.28 + 0.72 * diff);
                [
                    ((base[0] as f32 + spec * 255.0).min(255.0)) as u8,
                    ((base[1] as f32 + spec * 255.0).min(255.0)) as u8,
                    ((base[2] as f32 + spec * 255.0).min(255.0)) as u8,
                ]
            } else {
                let ring = ((r - 0.82).abs() * 14.0).clamp(0.0, 1.0);
                let glow = (1.0 - ring) * (0.55 + 0.35 * (time * 2.5 + u * 4.0).sin());
                [
                    (11.0 + glow * 72.0) as u8,
                    (15.0 + glow * 215.0) as u8,
                    (25.0 + glow * 255.0) as u8,
                ]
            }
        }
        Pattern::PrecisionGrid => {
            let c = (time * 0.45).cos();
            let s = (time * 0.45).sin();
            let rx = u * c - v * s;
            let ry = u * s + v * c;
            let gx = ((rx * 6.0).fract().abs() - 0.5).abs();
            let gy = ((ry * 6.0).fract().abs() - 0.5).abs();
            let line = (gx > 0.44) || (gy > 0.44);
            let diag = (rx - ry).abs() < 0.045 || (rx + ry).abs() < 0.045;
            let circle = ((u * u + v * v).sqrt() - 0.68).abs() < 0.035;
            if circle {
                [255, 198, 92]
            } else if diag {
                [88, 232, 255]
            } else if line {
                [168, 128, 255]
            } else {
                [10, 14, 24]
            }
        }
        Pattern::PrismTorus => {
            let r = (u * u + v * v).sqrt();
            let wave = (r * 9.0 - time * 3.0).sin() * 0.5 + 0.5;
            let spokes = (v.atan2(u) * 6.0 + time * 1.4).cos() * 0.5 + 0.5;
            let band = (1.0 - ((r - 0.54).abs() / 0.34).clamp(0.0, 1.0)).powf(1.4);
            let hue = ((v.atan2(u) / TAU) + r * 0.35 + time * 0.12).rem_euclid(1.0);
            let rgb = hsv_to_rgb(hue, 0.85, (0.18 + 0.82 * wave * spokes) * band);
            [rgb[0].max(10), rgb[1].max(14), rgb[2].max(24)]
        }
        Pattern::ChromaSwatches => {
            let hue = ((u * 0.5 + 0.5) + time * 0.08).rem_euclid(1.0);
            let val = (1.0 - (v * 0.5 + 0.5) * 0.72).clamp(0.15, 1.0);
            let sat = if v > 0.62 { 0.0 } else { 0.88 };
            let checker = ((((u + 1.0) * 6.0) as i32 + ((v + 1.0) * 6.0) as i32) & 1) == 0;
            let mut rgb = hsv_to_rgb(hue, sat, val);
            if v > 0.62 && checker {
                rgb = [235, 240, 250];
            }
            rgb
        }
    }
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let h6 = h.rem_euclid(1.0) * 6.0;
    let c = v * s;
    let x = c * (1.0 - ((h6 % 2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h6 as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [
        ((r + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

struct DoctorApp {
    caps: TerminalCaps,
    pattern: Pattern,
    time: f32,
    paused: bool,
    quit: bool,
    last: Option<Instant>,
    braille: Canvas,
    halfblock: Canvas,
    quadrant: Canvas,
}

impl DoctorApp {
    fn new(caps: TerminalCaps) -> Self {
        Self {
            caps,
            pattern: Pattern::SpectrumOrb,
            time: 0.0,
            paused: false,
            quit: false,
            last: None,
            braille: Canvas::with_encoding(Encoding::Braille),
            halfblock: Canvas::with_encoding(Encoding::HalfBlock),
            quadrant: Canvas::with_encoding(Encoding::Quadrant),
        }
    }

    fn draw_canvas_tile(
        canvas: &mut Canvas,
        encoding: Encoding,
        pattern: Pattern,
        time: f32,
        area: Area,
        parent: &mut Chunk<'_>,
    ) -> Result<(), DrawErr> {
        if area.width() < 4 || area.height() < 3 {
            return Ok(());
        }
        let (sx, sy) = encoding.samples_per_cell();
        let inner_w = area.width().saturating_sub(2);
        let inner_h = area.height().saturating_sub(2);
        let pw = usize::from(inner_w) * usize::from(sx);
        let ph = usize::from(inner_h) * usize::from(sy);

        canvas.clear();
        canvas.set_background(Color::Rgb { r: 9, g: 12, b: 20 });
        canvas.set_bound((0, inner_w as i32 - 1), (0, inner_h as i32 - 1));
        for y in 0..ph {
            let v = (y as f32 + 0.5) / ph.max(1) as f32 * 2.0 - 1.0;
            let cy = (ph - 1 - y) as f64;
            for x in 0..pw {
                let u = (x as f32 + 0.5) / pw.max(1) as f32 * 2.0 - 1.0;
                let [r, g, b] = sample_pattern(pattern, u, v, time);
                if encoding == Encoding::Braille {
                    // Raise contrast slightly so Braille dots glow brightly against obsidian bg.
                    if u16::from(r) + u16::from(g) + u16::from(b) > 75 {
                        canvas.set_color(x as f64, cy, Color::Rgb { r, g, b });
                    }
                } else {
                    canvas.set_color(x as f64, cy, Color::Rgb { r, g, b });
                }
            }
        }
        let origin = parent.area().pos();
        let inner = Area::new(
            (origin.x + area.x() + 1, origin.y + area.y() + 1).into(),
            (inner_w, inner_h).into(),
        );
        let sub = parent.from_area(inner)?;
        canvas.draw(sub)?;
        Ok(())
    }

    fn draw_raster_tile(&self, area: Area, parent: &mut Chunk<'_>) -> Result<(), DrawErr> {
        let inner_w = area.width().saturating_sub(2);
        let inner_h = area.height().saturating_sub(2);
        if inner_w < 2 || inner_h < 2 {
            return Ok(());
        }
        let Some(px) = self.caps.cell_pixels else {
            return Ok(());
        };
        if !self.caps.raster.has_any() {
            return Ok(());
        }
        let rw = (u32::from(inner_w) * u32::from(px.width)).clamp(16, 480);
        let rh = (u32::from(inner_h) * u32::from(px.height)).clamp(16, 360);
        let mut pixels = Vec::with_capacity((rw * rh) as usize);
        for y in 0..rh {
            let v = (y as f32 + 0.5) / rh as f32 * 2.0 - 1.0;
            for x in 0..rw {
                let u = (x as f32 + 0.5) / rw as f32 * 2.0 - 1.0;
                let [r, g, b] = sample_pattern(self.pattern, u, v, self.time);
                pixels.push(Rgb(r, g, b));
            }
        }
        if let Ok(rc) = RasterCanvas::from_pixels(rw, rh, Rgb(9, 12, 20), pixels) {
            let origin = parent.area().pos();
            let inner = Area::new(
                (origin.x + area.x() + 1, origin.y + area.y() + 1).into(),
                (inner_w, inner_h).into(),
            );
            let mut sub = parent.from_area(inner)?;
            let _ = sub.raster(&rc, RasterOptions::default());
        }
        Ok(())
    }
}

impl Update for DoctorApp {
    fn on_events(&mut self, events: &[Event]) -> Result<(), DrawErr> {
        for event in events {
            if let Event::Key(key) = event {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => self.quit = true,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.quit = true
                    }
                    KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
                        self.pattern = self.pattern.next();
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        self.pattern = self.pattern.prev();
                    }
                    KeyCode::Char('1') => self.pattern = Pattern::SpectrumOrb,
                    KeyCode::Char('2') => self.pattern = Pattern::PrecisionGrid,
                    KeyCode::Char('3') => self.pattern = Pattern::PrismTorus,
                    KeyCode::Char('4') => self.pattern = Pattern::ChromaSwatches,
                    KeyCode::Char(' ') | KeyCode::Char('p') => self.paused = !self.paused,
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn update(&mut self) -> Result<bool, DrawErr> {
        let now = Instant::now();
        if !self.paused {
            if let Some(last) = self.last {
                self.time += now.duration_since(last).as_secs_f32().min(0.1);
            }
        }
        self.last = Some(now);
        Ok(true)
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn required_size(&self, current: Size) -> Option<Size> {
        Some(Size::from((
            current.width,
            current.height.min(TARGET_INLINE_HEIGHT),
        )))
    }
}

fn style_rgb(fg: [u8; 3], bg: [u8; 3], bold: bool) -> Style {
    let colors = Colors::new(
        Color::Rgb {
            r: fg[0],
            g: fg[1],
            b: fg[2],
        },
        Color::Rgb {
            r: bg[0],
            g: bg[1],
            b: bg[2],
        },
    );
    if bold {
        Style::with_both(colors, Attributes::from(Attribute::Bold))
    } else {
        Style::with_colors(colors)
    }
}

fn draw_box(
    chunk: &mut Chunk<'_>,
    area: Area,
    title: &str,
    accent: [u8; 3],
    bg: [u8; 3],
) -> Result<(), DrawErr> {
    if area.width() < 4 || area.height() < 3 {
        return Ok(());
    }
    let x = area.x();
    let y = area.y();
    let w = area.width();
    let h = area.height();
    let border = style_rgb(accent, bg, false);
    chunk.fill(x, y, w, h, ' ', style_rgb([220, 230, 248], bg, false))?;
    let _ = chunk.set_char(x, y, '╭', border);
    let _ = chunk.set_char(x + w - 1, y, '╮', border);
    let _ = chunk.set_char(x, y + h - 1, '╰', border);
    let _ = chunk.set_char(x + w - 1, y + h - 1, '╯', border);
    for dx in 1..w - 1 {
        let _ = chunk.set_char(x + dx, y, '─', border);
        let _ = chunk.set_char(x + dx, y + h - 1, '─', border);
    }
    for dy in 1..h - 1 {
        let _ = chunk.set_char(x, y + dy, '│', border);
        let _ = chunk.set_char(x + w - 1, y + dy, '│', border);
    }
    let label = format!(" {title} ");
    if w as usize > label.len() + 2 {
        let _ = chunk.set_string(x + 2, y, &label, style_rgb([240, 246, 255], bg, true));
    }
    Ok(())
}

impl Draw for DoctorApp {
    fn draw(&mut self, mut chunk: Chunk) -> Result<Size, DrawErr> {
        let size = chunk.area().size();
        if size.width < 12 || size.height < 6 {
            return Ok(size);
        }
        let bg = [9, 12, 20];
        let hdr_bg = [15, 20, 34];
        chunk.fill(
            0,
            0,
            size.width,
            size.height,
            ' ',
            style_rgb([230, 238, 252], bg, false),
        )?;

        // Header + capability summary bar (3 rows)
        chunk.fill(
            0,
            0,
            size.width,
            3,
            ' ',
            style_rgb([230, 238, 252], hdr_bg, false),
        )?;
        let _ = chunk.set_string(
            1,
            0,
            "RSILLE DOCTOR · Terminal Graphics & Backend Lab",
            style_rgb([108, 232, 255], hdr_bg, true),
        );
        let cell_str = match self.caps.cell_pixels {
            Some(px) => format!("{}×{}px", px.width, px.height),
            None => "n/a".to_owned(),
        };
        let kitty_tag = if self.caps.raster.has_kitty() {
            "Kitty ✓"
        } else {
            "Kitty ✗"
        };
        let sixel_tag = if self.caps.raster.has_sixel() {
            "Sixel ✓"
        } else {
            "Sixel ✗"
        };
        let summary = format!(
            "{} · {}×{} · Cell {} · {} · {} · TrueColor {}",
            self.caps.terminal_name,
            size.width,
            size.height,
            cell_str,
            kitty_tag,
            sixel_tag,
            if self.caps.truecolor { "✓" } else { "?" }
        );
        let max_len = size.width.saturating_sub(2) as usize;
        let clipped: String = summary.chars().take(max_len).collect();
        let _ = chunk.set_string(1, 1, &clipped, style_rgb([182, 198, 224], hdr_bg, false));

        let pat_line = format!(
            "Pattern: {}   [1-4 / ←→ switch · Space {} · q quit]",
            self.pattern.name(),
            if self.paused { "resume" } else { "pause" }
        );
        let pat_clipped: String = pat_line.chars().take(max_len).collect();
        let _ = chunk.set_string(
            1,
            2,
            &pat_clipped,
            style_rgb([138, 156, 188], hdr_bg, false),
        );

        // 4 side-by-side (or 2x2) pipeline tiles
        let body_y = 4u16;
        let body_h = size.height.saturating_sub(body_y + 1);
        if body_h < 4 {
            return Ok(size);
        }

        let tiles = if size.width >= 84 {
            // 4 columns across
            let gap = 1u16;
            let usable = size.width.saturating_sub(2 + gap * 3);
            let tw = usable / 4;
            [
                Area::new((1, body_y).into(), (tw, body_h).into()),
                Area::new((1 + tw + gap, body_y).into(), (tw, body_h).into()),
                Area::new((1 + (tw + gap) * 2, body_y).into(), (tw, body_h).into()),
                Area::new((1 + (tw + gap) * 3, body_y).into(), (tw, body_h).into()),
            ]
        } else {
            // 2x2 grid
            let usable_w = size.width.saturating_sub(3);
            let tw = usable_w / 2;
            let th = body_h / 2;
            [
                Area::new((1, body_y).into(), (tw, th).into()),
                Area::new((2 + tw, body_y).into(), (tw, th).into()),
                Area::new((1, body_y + th).into(), (tw, body_h - th).into()),
                Area::new((2 + tw, body_y + th).into(), (tw, body_h - th).into()),
            ]
        };

        draw_box(&mut chunk, tiles[0], "Braille 2×4", [168, 128, 255], bg)?;
        Self::draw_canvas_tile(
            &mut self.braille,
            Encoding::Braille,
            self.pattern,
            self.time,
            tiles[0],
            &mut chunk,
        )?;

        draw_box(&mut chunk, tiles[1], "HalfBlock 1×2", [72, 224, 255], bg)?;
        Self::draw_canvas_tile(
            &mut self.halfblock,
            Encoding::HalfBlock,
            self.pattern,
            self.time,
            tiles[1],
            &mut chunk,
        )?;

        draw_box(&mut chunk, tiles[2], "Quadrant 2×2", [106, 232, 198], bg)?;
        Self::draw_canvas_tile(
            &mut self.quadrant,
            Encoding::Quadrant,
            self.pattern,
            self.time,
            tiles[2],
            &mut chunk,
        )?;

        let raster_title = if self.caps.raster.has_kitty() {
            "Kitty Pixel"
        } else if self.caps.raster.has_sixel() {
            "Sixel Pixel"
        } else {
            "Native Raster"
        };
        draw_box(&mut chunk, tiles[3], raster_title, [255, 184, 96], bg)?;
        if self.caps.raster.has_any() && self.caps.inline_session.is_some() {
            self.draw_raster_tile(tiles[3], &mut chunk)?;
        } else {
            let x = tiles[3].x() + 2;
            let mut y = tiles[3].y() + 2;
            let max_w = tiles[3].width().saturating_sub(4) as usize;
            let lines = [
                "Not detected here.",
                "",
                "Unicode Canvas works",
                "in ANY terminal!",
                "",
                "For Kitty / Sixel:",
                "Ghostty, Kitty,",
                "WezTerm, WT 1.22+",
            ];
            for line in lines {
                if y + 1 >= tiles[3].y() + tiles[3].height() {
                    break;
                }
                let s: String = line.chars().take(max_w).collect();
                let col = if line.starts_with("Unicode") {
                    [110, 232, 165]
                } else {
                    [156, 170, 198]
                };
                let _ = chunk.set_string(x, y, &s, style_rgb(col, bg, false));
                y += 1;
            }
        }

        // Bottom readiness bar
        let foot_y = size.height - 1;
        let ready = if self.caps.raster.has_any() {
            format!(
                " All 8 rsille experiences ready ({}) · try: rsille voxel | clockwork | desk | hud | pet | draw",
                self.caps.raster.short_label()
            )
        } else {
            " Ready everywhere: voxel · clockwork · desk · hud · jellyfish · doctor (pet/draw need Kitty/Sixel)".to_owned()
        };
        let foot_clipped: String = ready.chars().take(size.width as usize).collect();
        let _ = chunk.set_string(
            0,
            foot_y,
            &foot_clipped,
            style_rgb([150, 168, 198], bg, false),
        );

        Ok(size)
    }
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "RSILLE DOCTOR — Terminal graphics probe & multi-backend lab\n\n\
             Usage: rsille doctor [--plain | --headless]\n\n\
             --plain     Print a plain-text capability report (ideal for bug reports)\n\
             --headless  Verify rendering across all patterns without a TTY\n\n\
             Interactive keys: 1-4 / ←→ switch pattern · Space pause · q / Esc quit"
        );
        return Ok(());
    }
    if args.iter().any(|a| a == "--headless") {
        return verify_headless();
    }
    if args.iter().any(|a| a == "--plain")
        || !io::stdin().is_terminal()
        || !io::stdout().is_terminal()
    {
        let caps = probe_full(Duration::from_millis(250));
        print!("{}", format_plain_report(&caps));
        return Ok(());
    }

    let caps = probe_full(Duration::from_millis(250));
    let session = caps.inline_session;
    let (width, height) = crossterm::terminal::size()?;
    let buf_h = height.clamp(1, INLINE_MAX_HEIGHT);
    let used_h = height.clamp(1, TARGET_INLINE_HEIGHT);

    let mut builder = Builder::new();
    builder
        .size((width, buf_h))
        .inline_mode(true)
        .inline_max_height(INLINE_MAX_HEIGHT)
        .enable_raw_mode()
        .enable_hide_cursor()
        .frame_limit(30);
    if let Some(session) = session {
        builder.inline_raster_session(session);
    }
    let mut event_loop = builder.build_event_loop(DoctorApp::new(caps));
    event_loop.set_initial_used_height(used_h);
    event_loop.run()?;
    println!();
    Ok(())
}

fn verify_headless() -> Result<(), Box<dyn Error>> {
    let caps = TerminalCaps::default();
    let report = format_plain_report(&caps);
    assert!(report.contains("rsille doctor"));
    assert!(report.contains("rsille voxel"));

    let mut app = DoctorApp::new(caps);
    for pattern in Pattern::ALL {
        app.pattern = pattern;
        app.time = 1.25;
        for (w, h) in [(110, 24), (72, 20), (40, 12)] {
            let size = Size::from((w, h));
            let mut buffer = Buffer::new(size);
            let chunk = Chunk::new(&mut buffer, size.into())?;
            app.draw(chunk)?;
        }
    }
    println!("Doctor headless verified: report formatting and 4-backend calibration lab across all patterns.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headless_doctor_verifies_all_patterns_and_layouts() {
        verify_headless().unwrap();
    }

    #[test]
    fn plain_report_reflects_protocol_availability() {
        let caps = TerminalCaps {
            raster: RasterSupport::Kitty,
            cell_pixels: Some(CellPixels {
                width: 10,
                height: 20,
            }),
            ..TerminalCaps::default()
        };
        let report = format_plain_report(&caps);
        assert!(report.contains("10×20 px"));
        assert!(report.contains("Kitty Graphics : ✓ Supported"));
        assert!(report.contains("Ready (Kitty)"));
    }
}
