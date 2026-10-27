//! Shared hit testing and scrolling for row-based widgets.

use crate::tui::event::{Event, MouseEventKind};
use crate::tui::layout::ensure_item_visible;
use crate::tui::widget::EventCtx;

#[derive(Debug, Clone, Default)]
pub(crate) struct EditorPointer {
    pub anchor: Option<usize>,
    pub scrolled: bool,
}

pub(crate) fn wheel_delta(event: &Event) -> Option<isize> {
    match event {
        Event::Mouse(mouse) => match mouse.kind {
            MouseEventKind::ScrollUp => Some(-3),
            MouseEventKind::ScrollDown => Some(3),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn visible_rows<M>(ctx: &EventCtx<M>, border: bool, height: u16, header: u16) -> usize {
    ctx.bounds()
        .map(|area| area.height())
        .unwrap_or(height)
        .saturating_sub(u16::from(border) * 2)
        .saturating_sub(header) as usize
}

/// Coordinates inside the content, excluding borders and any header rows.
pub(crate) fn content_position<M>(
    ctx: &EventCtx<M>,
    event: &Event,
    border: bool,
    header: u16,
) -> Option<(usize, usize)> {
    let (x, y) = ctx.local_mouse_position(event)?;
    let area = ctx.bounds()?;
    let inset = u16::from(border);
    if x < inset
        || x >= area.width().saturating_sub(inset)
        || y < inset + header
        || y >= area.height().saturating_sub(inset)
    {
        return None;
    }
    Some(((x - inset) as usize, (y - inset - header) as usize))
}

/// Scroll the viewport and keep its active item visible without selecting it.
pub(crate) fn scroll_cursor(
    offset: usize,
    active: usize,
    total: usize,
    rows: usize,
    delta: isize,
    enabled: impl Fn(usize) -> bool,
) -> Option<(usize, usize)> {
    if total == 0 || rows == 0 {
        return None;
    }
    let offset = ensure_item_visible(offset.min(total - 1), active, rows);
    let next = offset
        .saturating_add_signed(delta)
        .min(total.saturating_sub(rows));
    if next == offset {
        return None;
    }
    let end = next.saturating_add(rows).min(total);
    let target = active.clamp(next, end - 1);
    let cursor = (next..end)
        .filter(|&i| enabled(i))
        .min_by_key(|&i| i.abs_diff(target))?;
    Some((next, cursor))
}
