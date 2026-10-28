//! A paint-only drag image: it follows the captured pointer without taking hits.
use super::*;
use rsille::render::chunk::Chunk;
use ui::{paint, put, ACCENT, BG, MUTED, TEXT};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Folder(PathBuf, String),
    Blocked(&'static str),
}
impl Target {
    pub fn path(&self) -> Option<&PathBuf> {
        if let Self::Folder(path, _) = self {
            Some(path)
        } else {
            None
        }
    }
}
#[derive(Clone, Debug)]
pub struct Preview {
    pub pointer: CellPoint,
    pub entries: Vec<Entry>,
    pub total: usize,
    pub copy: bool,
    pub target: Target,
}

pub struct Layer {
    pub child: [Box<dyn Widget<Msg>>; 1],
    pub preview: Option<Preview>,
}
impl Widget<Msg> for Layer {
    fn constraints(&self) -> Constraints {
        Constraints::fill()
    }
    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.child
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        ctx.render_children(chunk, self);
        if let Some(preview) = &self.preview {
            preview.render(chunk);
        }
    }
}

pub fn outline(chunk: &mut Chunk, rect: (i32, i32, i32, i32), fg: Color, bg: Color) {
    let (x, y, w, h) = rect;
    if w < 2 || h < 2 {
        return;
    }
    for xx in x + 1..x + w - 1 {
        put(chunk, xx, y, "─", fg, bg);
        put(chunk, xx, y + h - 1, "─", fg, bg);
    }
    for yy in y + 1..y + h - 1 {
        put(chunk, x, yy, "│", fg, bg);
        put(chunk, x + w - 1, yy, "│", fg, bg);
    }
    for (xx, yy, c) in [
        (x, y, "╭"),
        (x + w - 1, y, "╮"),
        (x, y + h - 1, "╰"),
        (x + w - 1, y + h - 1, "╯"),
    ] {
        put(chunk, xx, yy, c, fg, bg);
    }
}
impl Preview {
    fn render(&self, chunk: &mut Chunk) {
        let (vw, vh) = (
            i32::from(chunk.area().width()),
            i32::from(chunk.area().height()),
        );
        if vw < 6 || vh < 3 {
            return;
        }
        let depth = self.total.saturating_sub(1).min(2) as i32;
        let w = 34.min(vw - depth - 1);
        let h = 7.min(vh - depth);
        // Flip to the other side near an edge so the pointer and target stay visible.
        let x = if self.pointer.x + 2 + w + depth < vw {
            self.pointer.x + 2
        } else {
            self.pointer.x - w - 2
        }
        .clamp(0, vw - w - depth);
        let y = if self.pointer.y + 1 + h + depth < vh {
            self.pointer.y + 1
        } else {
            self.pointer.y - h - 1
        }
        .clamp(0, vh - h - depth);
        let bg = Color::Rgb(34, 51, 69);
        let border = Color::Rgb(111, 148, 168);
        for d in (1..=depth).rev() {
            let back = Color::Rgb(24 + d as u8 * 3, 35 + d as u8 * 3, 49 + d as u8 * 3);
            paint(chunk, x + d, y + d, w, h, back);
            outline(chunk, (x + d, y + d, w, h), Color::Rgb(61, 83, 105), back);
        }
        paint(chunk, x, y, w, h, bg);
        outline(chunk, (x, y, w, h), border, bg);
        let show_icon = w >= 18 && h >= icons::HEIGHT + 3;
        if show_icon {
            if let Some(entry) = self.entries.first() {
                icons::draw(chunk, x + 2, y + 1, entry, bg, false);
            }
        }
        let inset = if show_icon { icons::WIDTH + 4 } else { 2 };
        for (row, entry) in self
            .entries
            .iter()
            .take((h - 3).max(0) as usize)
            .enumerate()
        {
            put(
                chunk,
                x + inset,
                y + 1 + row as i32,
                &ui::truncate(&entry.name, (w - inset - 2).max(0) as usize),
                TEXT,
                bg,
            );
        }
        if self.total > 1 {
            let badge = format!(" {} ", self.total);
            let bw = badge.len().min((w - 2) as usize) as i32;
            put(
                chunk,
                x + w - bw - 1,
                y,
                &ui::truncate(&badge, bw as usize),
                BG,
                ACCENT,
            );
        }
        let (status, color) = match &self.target {
            Target::Folder(_, name) => (
                format!(
                    "{} {} to {}",
                    if self.copy { "+" } else { "↳" },
                    if self.copy { "Copy" } else { "Move" },
                    name
                ),
                ACCENT,
            ),
            Target::Blocked(reason) => (
                format!("× {} · {}", if self.copy { "Copy" } else { "Move" }, reason),
                MUTED,
            ),
        };
        if h >= 5 {
            put(
                chunk,
                x + 2,
                y + h - 2,
                &ui::truncate(&status, (w - 4) as usize),
                color,
                bg,
            );
        }
    }
}
