//! Shared offscreen rendering and buffer blitting helpers.

use std::cell::RefCell;

use render::area::{Area, Size};
use render::buffer::{Buffer, Cell};
use render::chunk::Chunk;
use render::style::Stylized;

thread_local! {
    static OFFSCREEN_CACHE: RefCell<Vec<CachedOffscreen>> = const { RefCell::new(Vec::new()) };
}

#[derive(Debug)]
struct CachedOffscreen {
    key: Option<u64>,
    buffer: Buffer,
}

/// Options controlling how cells are copied from an offscreen buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct BlitOptions {
    skip_blank: bool,
    dirty_only: bool,
}

impl BlitOptions {
    /// Skip cells that do not carry visible content or styling.
    pub(crate) fn skip_blank(mut self) -> Self {
        self.skip_blank = true;
        self
    }

    /// Visit only cells whose source content changed since the previous clear.
    pub(crate) fn dirty_only(mut self) -> Self {
        self.dirty_only = true;
        self
    }
}

/// Coarse accounting collected while selecting blit cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct BlitStats {
    pub(crate) visited: u64,
    pub(crate) skipped_blank: u64,
    pub(crate) skipped_clean: u64,
}

/// Source and destination rectangle for a buffer copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BlitRegion {
    pub(crate) source_x: usize,
    pub(crate) source_y: usize,
    pub(crate) dest_x: i32,
    pub(crate) dest_y: i32,
    pub(crate) width: usize,
    pub(crate) height: usize,
}

impl BlitRegion {
    pub(crate) fn new(
        source_x: usize,
        source_y: usize,
        dest_x: i32,
        dest_y: i32,
        width: usize,
        height: usize,
    ) -> Self {
        Self {
            source_x,
            source_y,
            dest_x,
            dest_y,
            width,
            height,
        }
    }
}

/// A cell selected for blitting after source/destination clipping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BlitCell {
    pub(crate) source_x: u16,
    pub(crate) source_y: u16,
    pub(crate) dest_x: u16,
    pub(crate) dest_y: u16,
    pub(crate) content: Stylized,
}

/// Render arbitrary content into a fresh offscreen buffer.
pub(crate) fn render_to_offscreen(
    area: Area,
    render: impl FnOnce(&mut Chunk<'_>),
) -> Option<Buffer> {
    let mut buffer = Buffer::new(area.real_size());
    {
        let mut chunk = Chunk::new(&mut buffer, area).ok()?;
        render(&mut chunk);
    }
    Some(buffer)
}

/// Render into a reusable offscreen buffer and inspect it before returning the
/// allocation to a thread-local cache.
pub(crate) fn with_reused_offscreen<R>(
    area: Area,
    key: Option<u64>,
    render: impl FnOnce(&mut Chunk<'_>),
    read: impl FnOnce(&Buffer) -> R,
) -> Option<R> {
    let size = area.real_size();
    let mut cached = OFFSCREEN_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache
            .iter()
            .position(|cached| cached.key == key && cached.buffer.size() == size)
            .map(|index| cache.swap_remove(index))
            .unwrap_or_else(|| CachedOffscreen {
                key,
                buffer: Buffer::new(size),
            })
    });

    if cached.buffer.size() != size {
        cached.buffer.resize(size);
    } else if key.is_some() {
        cached.buffer.clear();
    } else {
        cached.buffer.clear_content();
    }

    {
        let mut chunk = Chunk::new(&mut cached.buffer, area).ok()?;
        render(&mut chunk);
    }

    let result = read(&cached.buffer);

    OFFSCREEN_CACHE.with(|cache| {
        cache.borrow_mut().push(cached);
    });

    Some(result)
}

/// Copy a clipped region from an offscreen buffer into a target chunk.
pub(crate) fn blit_region(
    target: &mut Chunk<'_>,
    source: &Buffer,
    region: BlitRegion,
    options: BlitOptions,
) {
    let target_size = target.area().size();
    for_each_blit_cell(source, region, Some(target_size), options, |cell| {
        let _ = target.set_forced(cell.dest_x, cell.dest_y, cell.content);
    });
}

/// Visit all cells in a blit region after source/destination clipping.
pub(crate) fn for_each_blit_cell(
    source: &Buffer,
    region: BlitRegion,
    dest_size: Option<Size>,
    options: BlitOptions,
    mut visit: impl FnMut(BlitCell),
) -> BlitStats {
    let mut stats = BlitStats::default();
    let source_size = source.size();
    let source_width = source_size.width as usize;
    let source_height = source_size.height as usize;

    if source_width == 0 || source_height == 0 || region.width == 0 || region.height == 0 {
        return stats;
    }

    let previous = if options.dirty_only {
        source.previous()
    } else {
        None
    };

    for row in 0..region.height {
        let source_y = region.source_y + row;
        if source_y >= source_height {
            break;
        }

        let dest_y = region.dest_y + row as i32;
        if !is_inside_axis(dest_y, dest_size.map(|size| size.height)) {
            continue;
        }

        for col in 0..region.width {
            let source_x = region.source_x + col;
            if source_x >= source_width {
                break;
            }

            let dest_x = region.dest_x + col as i32;
            if !is_inside_axis(dest_x, dest_size.map(|size| size.width)) {
                continue;
            }

            let index = source_y * source_width + source_x;
            let Some(cell) = source.content().get(index) else {
                continue;
            };
            if previous
                .and_then(|previous| previous.get(index))
                .map(|previous| previous == cell)
                .unwrap_or(false)
            {
                stats.skipped_clean += 1;
                continue;
            }
            let owner = cell.owner.unwrap_or(index);
            let owner_x = owner % source_width;
            let owner_width = source.content()[owner].width();
            let owner_dest = region.dest_x as i64 + owner_x as i64 - region.source_x as i64;
            let fully_visible = owner_x >= region.source_x
                && owner_x + owner_width
                    <= region
                        .source_x
                        .saturating_add(region.width)
                        .min(source_width)
                && owner_dest >= 0
                && dest_size
                    .map(|size| owner_dest + owner_width as i64 <= size.width as i64)
                    .unwrap_or(true);
            let content = if !fully_visible {
                Stylized::new(' ', source.content()[owner].content.style)
            } else {
                if should_skip_cell(cell, options) {
                    stats.skipped_blank += 1;
                    continue;
                }
                cell.content.clone()
            };

            stats.visited += 1;
            visit(BlitCell {
                source_x: source_x as u16,
                source_y: source_y as u16,
                dest_x: dest_x as u16,
                dest_y: dest_y as u16,
                content,
            });
        }
    }

    stats
}

fn should_skip_cell(cell: &Cell, options: BlitOptions) -> bool {
    cell.is_occupied || (options.skip_blank && is_blank_cell(cell))
}

fn is_blank_cell(cell: &Cell) -> bool {
    match cell.content.as_char() {
        _ if cell.content.is_empty() => true,
        Some(' ') => !cell.content.has_color() && !cell.content.has_attr(),
        _ => false,
    }
}

fn is_inside_axis(value: i32, max: Option<u16>) -> bool {
    if value < 0 || value > u16::MAX as i32 {
        return false;
    }

    max.map(|max| value < max as i32).unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_to_offscreen_preserves_area_coordinates() {
        let area = Area::new((2, 1).into(), (3, 2).into());
        let buffer = render_to_offscreen(area, |chunk| {
            let _ = chunk.set_forced(0, 0, Stylized::plain('x'));
        })
        .unwrap();

        assert_eq!(buffer.size(), (5, 3).into());
        assert_eq!(cell_char(&buffer, 2, 1), Some('x'));
    }

    #[test]
    fn blit_region_clips_source_and_destination_bounds() {
        let mut source = Buffer::new((4, 1).into());
        let _ = source.overwrite((1, 0).into(), Stylized::plain('b'));
        let _ = source.overwrite((2, 0).into(), Stylized::plain('c'));
        let _ = source.overwrite((3, 0).into(), Stylized::plain('d'));

        let mut target = Buffer::new((3, 1).into());
        {
            let mut chunk =
                Chunk::new(&mut target, Area::new((0, 0).into(), (3, 1).into())).unwrap();
            blit_region(
                &mut chunk,
                &source,
                BlitRegion::new(1, 0, 1, 0, 4, 1),
                BlitOptions::default(),
            );
        }

        assert_eq!(cell_char(&target, 0, 0), Some(' '));
        assert_eq!(cell_char(&target, 1, 0), Some('b'));
        assert_eq!(cell_char(&target, 2, 0), Some('c'));
    }

    #[test]
    fn blit_region_can_skip_blank_cells() {
        let source = Buffer::new((1, 1).into());
        let mut target = Buffer::new((1, 1).into());
        let _ = target.overwrite((0, 0).into(), Stylized::plain('z'));

        {
            let mut chunk =
                Chunk::new(&mut target, Area::new((0, 0).into(), (1, 1).into())).unwrap();
            blit_region(
                &mut chunk,
                &source,
                BlitRegion::new(0, 0, 0, 0, 1, 1),
                BlitOptions::default().skip_blank(),
            );
        }

        assert_eq!(cell_char(&target, 0, 0), Some('z'));
    }

    #[test]
    fn blit_iteration_can_skip_clean_cells() {
        let mut source = Buffer::new((3, 1).into());
        let _ = source.overwrite((0, 0).into(), Stylized::plain('a'));
        let _ = source.overwrite((1, 0).into(), Stylized::plain('b'));
        source.clear();
        let _ = source.overwrite((0, 0).into(), Stylized::plain('a'));
        let _ = source.overwrite((1, 0).into(), Stylized::plain('c'));

        let mut visited = Vec::new();
        let stats = for_each_blit_cell(
            &source,
            BlitRegion::new(0, 0, 0, 0, 3, 1),
            Some((3, 1).into()),
            BlitOptions::default().skip_blank().dirty_only(),
            |cell| visited.push((cell.dest_x, cell.dest_y, cell.content.as_char())),
        );

        assert_eq!(visited, vec![(1, 0, Some('c'))]);
        assert_eq!(stats.visited, 1);
        assert_eq!(stats.skipped_clean, 2);
        assert_eq!(stats.skipped_blank, 0);
    }

    #[test]
    fn blit_replaces_partial_graphemes_at_each_edge_with_spaces() {
        use render::style::Style;
        let mut source = Buffer::new((6, 1).into());
        let mut chunk = Chunk::new(&mut source, Area::new((0, 0).into(), (6, 1).into())).unwrap();
        chunk.set_string(0, 0, "👩🏽‍💻x中z", Style::default()).unwrap();
        for (source_x, dest_x, width, expected) in
            [(1, 0, 4, " x中"), (0, 0, 4, "👩🏽‍💻x "), (0, -1, 6, " x中z")]
        {
            let target_width = if dest_x < 0 { 5 } else { 4 };
            let mut target = Buffer::new((target_width, 1).into());
            let mut chunk = Chunk::new(
                &mut target,
                Area::new((0, 0).into(), (target_width, 1).into()),
            )
            .unwrap();
            blit_region(
                &mut chunk,
                &source,
                BlitRegion::new(source_x, 0, dest_x, 0, width, 1),
                BlitOptions::default(),
            );
            let actual: String = target
                .content()
                .iter()
                .filter(|c| !c.is_occupied)
                .map(|c| c.content.symbol().into_owned())
                .collect();
            assert_eq!(actual, expected);
            for (i, cell) in target.content().iter().enumerate() {
                if cell.is_occupied {
                    let owner = cell.owner.unwrap();
                    assert!(owner < i && owner + target.content()[owner].width() > i);
                }
            }
        }
    }

    fn cell_char(buffer: &Buffer, x: u16, y: u16) -> Option<char> {
        let index = (y * buffer.size().width + x) as usize;
        buffer.content().get(index)?.content.as_char()
    }
}
