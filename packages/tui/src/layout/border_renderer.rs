//! Border rendering helper functions

use crate::style::BorderStyle;
use render::area::Area;
use render::chunk::Chunk;
use render::style::Style;

/// Render a border around the specified area
///
/// # Arguments
/// * `chunk` - The chunk to render into
/// * `area` - The area to draw the border around
/// * `border` - The border style to use
/// * `style` - The render style for the border
pub fn render_border(
    chunk: &mut Chunk,
    area: Area,
    border: BorderStyle,
    style: Style,
) {
    if area.width() < 2 || area.height() < 2 {
        return; // Not enough space for border
    }

    let chars = border.chars();

    // Top and bottom borders
    for x in 1..area.width().saturating_sub(1) {
        let _ = chunk.set_char(area.x() + x, area.y(), chars.horizontal, style);
        let _ = chunk.set_char(
            area.x() + x,
            area.y() + area.height() - 1,
            chars.horizontal,
            style,
        );
    }

    // Left and right borders
    for y in 1..area.height().saturating_sub(1) {
        let _ = chunk.set_char(area.x(), area.y() + y, chars.vertical, style);
        let _ = chunk.set_char(
            area.x() + area.width() - 1,
            area.y() + y,
            chars.vertical,
            style,
        );
    }

    // Corners
    let _ = chunk.set_char(area.x(), area.y(), chars.top_left, style);
    let _ = chunk.set_char(
        area.x() + area.width() - 1,
        area.y(),
        chars.top_right,
        style,
    );
    let _ = chunk.set_char(
        area.x(),
        area.y() + area.height() - 1,
        chars.bottom_left,
        style,
    );
    let _ = chunk.set_char(
        area.x() + area.width() - 1,
        area.y() + area.height() - 1,
        chars.bottom_right,
        style,
    );
}

/// Render a background fill in the specified area
///
/// # Arguments
/// * `chunk` - The chunk to render into
/// * `area` - The area to fill
/// * `style` - The render style for the background
pub fn render_background(chunk: &mut Chunk, area: Area, style: Style) {
    for y in 0..area.height() {
        for x in 0..area.width() {
            let _ = chunk.set_char(area.x() + x, area.y() + y, ' ', style);
        }
    }
}
