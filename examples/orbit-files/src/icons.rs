//! Lightweight terminal line icons, shared by the board and drag preview.
use super::{ui, Color, Entry};
use rsille::render::chunk::Chunk;

pub const WIDTH: i32 = 8;
pub const HEIGHT: i32 = 4;

pub fn color(entry: &Entry) -> Color {
    match entry.kind() {
        "FOLDER" => ui::GOLD,
        "CODE" => ui::ACCENT,
        "IMAGE" => ui::VIOLET,
        "AUDIO" => Color::Rgb(235, 159, 186),
        "ARCHIVE" => Color::Rgb(222, 174, 125),
        "LINK" => Color::Rgb(147, 199, 223),
        _ => Color::Rgb(147, 184, 226),
    }
}

pub fn symbol(entry: &Entry) -> &'static str {
    match entry.kind() {
        "FOLDER" => "▰",
        "CODE" => "λ",
        "TEXT" => "≡",
        "IMAGE" => "▧",
        "AUDIO" => "♪",
        "ARCHIVE" => "▤",
        "LINK" => "↗",
        _ => "◇",
    }
}

pub fn draw(chunk: &mut Chunk, x: i32, y: i32, entry: &Entry, bg: Color, dim: bool) {
    let kind = entry.kind();
    let accent = if dim { ui::MUTED } else { color(entry) };
    let stroke = if dim {
        ui::MUTED.interpolate(bg, 0.48)
    } else if kind == "FOLDER" {
        accent.interpolate(bg, 0.18)
    } else {
        ui::TEXT.interpolate(bg, 0.40)
    };
    // Open space and a single stroke weight keep these legible at 8 × 4 cells.
    // The folder tab, landscape and zipper have their own silhouettes.
    let art = match kind {
        "FOLDER" => ["╭──╮    ", "│  ╰───╮", "│      │", "╰──────╯"],
        "IMAGE" => ["        ", "╭──────╮", "│ · ╱╲ │", "╰──────╯"],
        "ARCHIVE" => [" ╭──┬─╮ ", " │  ┆ │ ", " │  ┆ │ ", " ╰──┴─╯ "],
        _ => [" ╭───╮  ", " │   └╮ ", " │    │ ", " ╰────╯ "],
    };
    for row in 0..HEIGHT {
        ui::put(chunk, x, y + row, art[row as usize], stroke, bg);
    }
    let mark = match kind {
        "CODE" => "‹›",
        "TEXT" => "≡",
        "AUDIO" => "♫",
        "LINK" => "↗",
        "FILE" => "·",
        _ => return,
    };
    let width = unicode_width::UnicodeWidthStr::width(mark) as i32;
    ui::put(
        chunk,
        x + (WIDTH - width) / 2,
        y + 2,
        mark,
        if dim { stroke } else { accent },
        bg,
    );
}
