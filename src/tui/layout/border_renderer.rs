//! Border rendering helper functions

use crate::render::chunk::Chunk;
use crate::render::style::Style;
use crate::tui::style::{BorderChars, BorderStyle};

/// Render a border in the chunk's area
///
/// This function renders borders using the chunk's area information.
/// All rendering is done in relative coordinates within the chunk.
///
/// # Arguments
/// * `chunk` - The chunk to render into (contains area information)
/// * `border` - The border style to use
/// * `style` - The render style for the border
pub fn render_border(chunk: &mut Chunk, border: BorderStyle, style: Style) {
    let area = chunk.area();
    let width = area.width();
    let height = area.height();

    if width < 2 || height < 2 {
        return; // Not enough space for border
    }

    let chars = border_chars(chunk, border);

    // Top and bottom borders
    for x in 1..width.saturating_sub(1) {
        let _ = chunk.set_char(x, 0, chars.horizontal, style);
        let _ = chunk.set_char(x, height - 1, chars.horizontal, style);
    }

    // Left and right borders
    for y in 1..height.saturating_sub(1) {
        let _ = chunk.set_char(0, y, chars.vertical, style);
        let _ = chunk.set_char(width - 1, y, chars.vertical, style);
    }

    // Corners
    let _ = chunk.set_char(0, 0, chars.top_left, style);
    let _ = chunk.set_char(width - 1, 0, chars.top_right, style);
    let _ = chunk.set_char(0, height - 1, chars.bottom_left, style);
    let _ = chunk.set_char(width - 1, height - 1, chars.bottom_right, style);
}

/// Render only the bottom border in the chunk's area
///
/// This function renders only the bottom border using the chunk's area information.
/// All rendering is done in relative coordinates within the chunk.
///
/// # Arguments
/// * `chunk` - The chunk to render into (contains area information)
/// * `border` - The border style to use
/// * `style` - The render style for the border
pub fn render_border_bottom(chunk: &mut Chunk, border: BorderStyle, style: Style) {
    let area = chunk.area();
    let width = area.width();
    let height = area.height();

    if width == 0 || height == 0 {
        return;
    }

    let chars = border_chars(chunk, border);

    // Only draw horizontal line at bottom
    for x in 0..width {
        let _ = chunk.set_char(x, height - 1, chars.horizontal, style);
    }
}

/// Render a background fill in the chunk's area
///
/// # Arguments
/// * `chunk` - The chunk to render into (contains area information)
/// * `style` - The render style for the background
pub fn render_background(chunk: &mut Chunk, style: Style) {
    let area = chunk.area();
    let width = area.width();
    let height = area.height();

    for y in 0..height {
        for x in 0..width {
            let _ = chunk.set_char(x, y, ' ', style);
        }
    }
}

// Box-drawing characters are ambiguous in some width policies. Fall back to
// one-column ASCII so borders retain their geometry on those terminals.
fn border_chars(chunk: &Chunk<'_>, border: BorderStyle) -> BorderChars {
    let chars = border.chars();
    let mut bytes = [0; 4];
    if chunk
        .text_metrics()
        .grapheme_width(chars.horizontal.encode_utf8(&mut bytes), 0)
        == 1
    {
        chars
    } else {
        BorderChars {
            top_left: '+',
            top_right: '+',
            bottom_left: '+',
            bottom_right: '+',
            horizontal: '-',
            vertical: '|',
            top_junction: '+',
            bottom_junction: '+',
            left_junction: '+',
            right_junction: '+',
            cross: '+',
        }
    }
}
