//! Rendering utilities for select widget

use super::item::SelectItem;
use super::style::{DropdownStyles, StyleConfig};
use ::render::chunk::Chunk;
use ::render::style::Style as RenderStyle;
use unicode_width::UnicodeWidthStr;

/// Truncate text to fit within available width
///
/// Returns the truncated string with an ellipsis if needed
pub fn truncate_text(text: &str, available_width: u16) -> String {
    use unicode_width::UnicodeWidthChar;

    let text_width = text.width() as u16;
    if text_width <= available_width {
        return text.to_string();
    }

    let mut truncated = String::new();
    let mut current_width = 0u16;

    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
        if current_width + ch_width + 1 > available_width {
            truncated.push('…');
            break;
        }
        truncated.push(ch);
        current_width += ch_width;
    }

    truncated
}

/// Render a single dropdown item
///
/// Returns the y-offset for the next item
pub fn render_item<T: Clone>(
    chunk: &mut Chunk,
    item: &SelectItem<T>,
    y: u16,
    width: u16,
    config: &StyleConfig,
    styles: &DropdownStyles,
    is_focused: bool,
    is_selected: bool,
) {
    let item_style = styles.item_style(item.disabled, is_selected, is_focused);

    // Fill background for item
    let _ = chunk.fill(config.border_offset, y, width, 1, ' ', item_style);

    // Render selection indicator
    let indicator = if is_selected { "• " } else { "  " };
    let indicator_x = config.content_padding_x;
    let _ = chunk.set_string(indicator_x, y, indicator, item_style);

    // Calculate label position and available width
    let label_x = indicator_x + 2; // +2 for indicator width
    let available_width = width.saturating_sub(label_x + config.content_padding_x);

    // Render item label (truncated if needed)
    let label = truncate_text(&item.label, available_width);
    let _ = chunk.set_string(label_x, y, &label, item_style);
}

/// Render scrollbar for dropdown
pub fn render_scrollbar(
    chunk: &mut Chunk,
    x: u16,
    height: u16,
    viewport_size: usize,
    total_items: usize,
    scroll_offset: usize,
    track_style: RenderStyle,
    thumb_style: RenderStyle,
) {
    let scrollbar_height = height.saturating_sub(2) as usize;

    if scrollbar_height == 0 {
        return;
    }

    // Calculate thumb size and position
    let thumb_size = ((viewport_size as f64 / total_items as f64) * scrollbar_height as f64)
        .max(1.0)
        .round() as usize;

    let max_scroll = total_items.saturating_sub(viewport_size);
    let scroll_ratio = if max_scroll > 0 {
        scroll_offset as f64 / max_scroll as f64
    } else {
        0.0
    };
    let thumb_position = (scroll_ratio * (scrollbar_height - thumb_size) as f64).round() as usize;

    // Draw scrollbar track
    for y in 1..(height - 1) {
        let _ = chunk.set_char(x, y, '│', track_style);
    }

    // Draw scrollbar thumb
    for offset in 0..thumb_size {
        let y = 1 + (thumb_position + offset).min(scrollbar_height - 1);
        let _ = chunk.set_char(x, y as u16, '█', thumb_style);
    }
}

/// Render dropdown menu with items
pub fn render_dropdown<T: Clone>(
    chunk: &mut Chunk,
    items: &[SelectItem<T>],
    focused_index: Option<usize>,
    selected_index: Option<usize>,
    scroll_offset: usize,
    config: &StyleConfig,
    styles: &DropdownStyles,
    show_scrollbar: bool,
    scrollbar_x: u16,
) {
    let area = chunk.area();
    let width = area.width();
    let height = area.height();

    if width == 0 || height == 0 {
        return;
    }

    // Calculate dimensions
    let content_start_y = config.content_padding_y;
    let content_width = width.saturating_sub(config.border_offset * 2);
    let visible_height = height.saturating_sub(config.dropdown_border_offset()) as usize;

    // Fill background
    if config.border_offset > 0 {
        let _ = chunk.fill(1, 1, width.saturating_sub(2), height.saturating_sub(2), ' ', styles.background);
        // Render border
        use crate::style::BorderStyle;
        crate::layout::border_renderer::render_border(chunk, BorderStyle::Single, styles.border);
    } else {
        let _ = chunk.fill(0, 0, width, height, ' ', styles.background);
    }

    // Render items
    let visible_start = scroll_offset;
    let visible_end = (visible_start + visible_height).min(items.len());

    let mut y = content_start_y;
    for (idx, item) in items
        .iter()
        .enumerate()
        .skip(visible_start)
        .take(visible_end - visible_start)
    {
        if y >= height.saturating_sub(config.border_offset) {
            break;
        }

        let is_focused = Some(idx) == focused_index && !item.disabled;
        let is_selected = Some(idx) == selected_index;

        render_item(chunk, item, y, content_width, config, styles, is_focused, is_selected);

        y += 1;
    }

    // Render scrollbar if needed
    if show_scrollbar {
        let (track_style, thumb_style) = super::style::get_scrollbar_styles();
        render_scrollbar(
            chunk,
            scrollbar_x,
            height,
            visible_height,
            items.len(),
            scroll_offset,
            track_style,
            thumb_style,
        );
    }
}

/// Render trigger button
pub fn render_trigger(
    chunk: &mut Chunk,
    trigger_text: &str,
    is_opened: bool,
    config: &StyleConfig,
    style: RenderStyle,
) {
    let area = chunk.area();
    let width = area.width();
    let height = area.height();

    if width == 0 || height == 0 {
        return;
    }

    // Fill background
    if config.border_offset > 0 {
        let _ = chunk.fill(1, 1, width.saturating_sub(2), height.saturating_sub(2), ' ', style);
        // Render border
        use crate::style::BorderStyle;
        crate::layout::border_renderer::render_border(chunk, BorderStyle::Single, style);
    } else {
        let _ = chunk.fill(0, 0, width, height, ' ', style);
    }

    // Calculate text position
    let text_y = config.content_padding_y;
    let text_x = config.content_padding_x;
    let available_width = width.saturating_sub(text_x + config.content_padding_x + 2); // -2 for indicator

    // Render text (truncated if needed)
    let text = truncate_text(trigger_text, available_width);
    let _ = chunk.set_string(text_x, text_y, &text, style);

    // Render dropdown indicator
    let indicator = if is_opened { "▲" } else { "▼" };
    let indicator_x = width.saturating_sub(config.content_padding_x + 1);
    let _ = chunk.set_string(indicator_x, text_y, indicator, style);
}
