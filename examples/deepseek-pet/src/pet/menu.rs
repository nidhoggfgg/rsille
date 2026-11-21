use rsille::render::{
    area::{Area, Size},
    chunk::Chunk,
    text::TextMetrics,
    Builder, Draw, DrawErr,
};
use rsille::tui::{
    layout::border_renderer::render_border,
    style::{BorderStyle, Color, Style},
};
use std::io;

#[derive(Clone, Copy)]
pub(super) enum Action {
    Feed,
    Bubbles,
    Nap,
}

const ITEMS: &[(Action, &str)] = &[
    (Action::Feed, "喂小鱼干"),
    (Action::Bubbles, "吹泡泡"),
    (Action::Nap, "让它打个盹"),
];

#[derive(Clone, Copy, PartialEq)]
pub(super) struct Menu {
    pub(super) area: Area,
    tail: Option<(u16, u16, char)>,
}

impl Menu {
    pub(super) fn place(whale: Area, cols: u16, rows: u16) -> Option<Self> {
        let metrics = TextMetrics::default();
        let width = ITEMS.iter().map(|(_, s)| metrics.width(s)).max()? as u16 + 4;
        let height = ITEMS.len() as u16 + 2;
        let max_x = cols.checked_sub(1 + width)?; // Reserve the last column for autowrap.
        let max_y = rows.checked_sub(height)?;
        let right = whale.real_size().width;
        let (x, y) = if right <= max_x {
            (right, whale.y().min(max_y))
        } else if whale.x() >= width {
            (whale.x() - width, whale.y().min(max_y))
        } else {
            let y = whale
                .y()
                .checked_sub(height)
                .unwrap_or(whale.real_size().height);
            (whale.x().min(max_x), y.min(max_y))
        };
        let area = Area::new((x, y).into(), (width, height).into());
        let whale_end = whale.real_size();
        let tail = if x >= whale_end.width || x + width <= whale.x() {
            let row = (whale.y() + whale.height() / 2)
                .saturating_sub(y)
                .clamp(1, height - 2);
            Some(if x >= whale_end.width {
                (0, row, '◂')
            } else {
                (width - 1, row, '▸')
            })
        } else if y >= whale_end.height || y + height <= whale.y() {
            let col = (whale.x() + whale.width() / 2)
                .saturating_sub(x)
                .clamp(1, width - 2);
            Some(if y >= whale_end.height {
                (col, 0, '▴')
            } else {
                (col, height - 1, '▾')
            })
        } else {
            None
        };
        Some(Self { area, tail })
    }

    pub(super) fn action_at(self, col: u16, row: u16) -> Option<Action> {
        // Mouse coordinates are one-based. The border is not an action.
        let x = col.checked_sub(self.area.x() + 1)?;
        if x == 0 || x >= self.area.width() - 1 {
            return None;
        }
        let item = row.checked_sub(self.area.y() + 2)?;
        ITEMS.get(usize::from(item)).map(|&(action, _)| action)
    }

    pub(super) fn render(self, out: &mut Vec<u8>) -> io::Result<()> {
        Builder::new()
            .size(self.area.size().into())
            .pos(self.area.pos().into())
            .clear(false)
            .build_render(self, out)
            .render()
    }
}

impl Draw for Menu {
    fn draw(&mut self, mut chunk: Chunk<'_>) -> Result<Size, DrawErr> {
        let size = chunk.area().size();
        let text = Style::default().to_render_style();
        let outline = Style::default().fg(Color::Cyan).to_render_style();
        chunk.fill(0, 0, size.width, size.height, ' ', text)?;
        render_border(&mut chunk, BorderStyle::Rounded, outline);
        if let Some((x, y, tip)) = self.tail {
            chunk.set_char(x, y, tip, outline)?;
        }
        for (i, &(_, label)) in ITEMS.iter().enumerate() {
            chunk.set_string(2, i as u16 + 1, label, text)?;
        }
        Ok(size)
    }
}
