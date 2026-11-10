//! Shared offscreen rendering and buffer blitting helpers.

use std::cell::RefCell;

use crate::render::area::{Area, Size};
use crate::render::buffer::{Buffer, Cell};
use crate::render::chunk::Chunk;
use crate::render::style::Stylized;

// Limits apply to idle allocations per thread, not buffers actively rendering.
const OFFSCREEN_CACHE_MAX_ENTRIES: usize = 8;
const OFFSCREEN_CACHE_MAX_BYTES: usize = 8 * 1024 * 1024;

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
    /// Effect callers must propagate this to the destination chunk so strict
    /// raster presentation can reject flattened raster content.
    pub(crate) unsupported_raster_composition: bool,
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
    let mut buffer = Buffer::new(checked_area_size(area)?);
    {
        let mut chunk = Chunk::new(&mut buffer, area).ok()?;
        render(&mut chunk);
    }
    Some(buffer)
}

/// Render into a reusable offscreen buffer and inspect it before returning the
/// allocation to a thread-local cache. Pooled buffers retain no frame content
/// or raster source snapshots. The pool keeps at most eight idle buffers and
/// 8 MiB of cell-storage capacity per thread, evicting least recently returned
/// allocations first. Larger buffers render normally but are not pooled.
pub(crate) fn with_reused_offscreen<R>(
    area: Area,
    key: Option<u64>,
    render: impl FnOnce(&mut Chunk<'_>),
    read: impl FnOnce(&Buffer) -> R,
) -> Option<R> {
    let size = checked_area_size(area)?;
    let mut cached = OFFSCREEN_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache
            .iter()
            .position(|cached| cached.key == key && cached.buffer.size() == size)
            .map(|index| cache.remove(index))
            .unwrap_or_else(|| {
                let mut buffer = Buffer::new(size);
                buffer.clear_content();
                CachedOffscreen { key, buffer }
            })
    });

    {
        let mut chunk = Chunk::new(&mut cached.buffer, area).ok()?;
        render(&mut chunk);
    }

    let result = read(&cached.buffer);

    recycle_offscreen(cached);

    Some(result)
}

fn recycle_offscreen(mut cached: CachedOffscreen) {
    // Release source snapshots immediately, not only when this key/size is
    // requested again. Each destination is repainted independently, so source
    // history must also be discarded before a subsequent dirty-only blit.
    cached.buffer.clear_content();
    let bytes = cached.buffer.storage_bytes();
    if bytes > OFFSCREEN_CACHE_MAX_BYTES {
        return;
    }
    OFFSCREEN_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        while cache.len() >= OFFSCREEN_CACHE_MAX_ENTRIES
            || cache.iter().fold(bytes, |total, entry| {
                total.saturating_add(entry.buffer.storage_bytes())
            }) > OFFSCREEN_CACHE_MAX_BYTES
        {
            cache.remove(0);
        }
        cache.push(cached);
    });
}

fn checked_area_size(area: Area) -> Option<Size> {
    Some(
        (
            area.x().checked_add(area.width())?,
            area.y().checked_add(area.height())?,
        )
            .into(),
    )
}

/// Copy a clipped region from an offscreen buffer into a target chunk.
pub(crate) fn blit_region(
    target: &mut Chunk<'_>,
    source: &Buffer,
    region: BlitRegion,
    options: BlitOptions,
) {
    let target_size = target.area().size();
    let stats = select_blit_cells(source, region, Some(target_size), options, false, |cell| {
        if let Some(raster) = source.raster_at((cell.source_x, cell.source_y).into()) {
            // Write first: erasing the previous grapheme must not erase the
            // incoming metadata, and copying must not resample cropped cells.
            let _ = target.set_raster_cell_with_fallback(
                cell.dest_x,
                cell.dest_y,
                raster.clone(),
                cell.content,
            );
        } else {
            let _ = target.set_forced(cell.dest_x, cell.dest_y, cell.content);
        }
    });
    if stats.unsupported_raster_composition {
        target.mark_unsupported_raster_composition();
    }
}

/// Visits stored character fallbacks, discarding native raster metadata.
///
/// Effect callers must propagate `unsupported_raster_composition` from the
/// returned stats to their destination via `Chunk::mark_unsupported_raster_composition`.
pub(crate) fn for_each_blit_cell(
    source: &Buffer,
    region: BlitRegion,
    dest_size: Option<Size>,
    options: BlitOptions,
    visit: impl FnMut(BlitCell),
) -> BlitStats {
    select_blit_cells(source, region, dest_size, options, true, visit)
}

fn select_blit_cells(
    source: &Buffer,
    region: BlitRegion,
    dest_size: Option<Size>,
    options: BlitOptions,
    flatten_raster: bool,
    mut visit: impl FnMut(BlitCell),
) -> BlitStats {
    let mut stats = BlitStats {
        unsupported_raster_composition: source.has_unsupported_raster_composition(),
        ..BlitStats::default()
    };
    let source_size = source.size();
    let source_width = source_size.width as usize;
    let source_height = source_size.height as usize;

    if region.source_x >= source_width
        || region.source_y >= source_height
        || region.width == 0
        || region.height == 0
    {
        return stats;
    }

    let previous = if options.dirty_only {
        source.previous()
    } else {
        None
    };

    for row in 0..region.height.min(source_height - region.source_y) {
        let source_y = region.source_y + row;
        let dest_y = region.dest_y as i64 + row as i64;
        if !is_inside_axis(dest_y, dest_size.map(|size| size.height)) {
            continue;
        }

        for col in 0..region.width.min(source_width - region.source_x) {
            let source_x = region.source_x + col;
            let dest_x = region.dest_x as i64 + col as i64;
            if !is_inside_axis(dest_x, dest_size.map(|size| size.width)) {
                continue;
            }

            let index = source_y * source_width + source_x;
            let Some(cell) = source.content().get(index) else {
                continue;
            };
            let pos = (source_x as u16, source_y as u16).into();
            let raster = source.raster_at(pos);
            if flatten_raster && raster.is_some() {
                stats.unsupported_raster_composition = true;
            }
            if previous
                .and_then(|previous| previous.get(index))
                .map(|previous| previous == cell)
                .unwrap_or(false)
                && raster == source.previous_raster_at(pos)
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
                    .unwrap_or(owner_dest + owner_width as i64 <= u16::MAX as i64 + 1);
            let content = if !fully_visible {
                Stylized::new(' ', source.content()[owner].content.style)
            } else {
                if cell.is_occupied || (raster.is_none() && should_skip_cell(cell, options)) {
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

fn is_inside_axis(value: i64, max: Option<u16>) -> bool {
    if value < 0 || value > u16::MAX as i64 {
        return false;
    }

    max.map(|max| value < max as i64).unwrap_or(true)
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
        use crate::render::style::Style;
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

    fn raster_source(size: Size) -> Buffer {
        let canvas = crate::render::raster::RasterCanvas::new(2, 2, Default::default()).unwrap();
        render_to_offscreen(size.into(), |chunk| {
            chunk.raster(&canvas, Default::default()).unwrap();
        })
        .unwrap()
    }

    #[test]
    fn raster_blit_preserves_scroll_crop_and_nested_clip_mapping() {
        let source = raster_source((5, 4).into());
        let mut target = Buffer::new((5, 4).into());
        let mut chunk = Chunk::new(&mut target, Area::new((1, 1).into(), (3, 2).into())).unwrap();
        blit_region(
            &mut chunk,
            &source,
            BlitRegion::new(1, 1, -1, -1, 5, 4),
            BlitOptions::default(),
        );
        assert_eq!(target.raster_cells().count(), 6);
        for (x, y, cell) in target.raster_cells() {
            assert_eq!(cell.target, (5, 4).into());
            assert_eq!((cell.x, cell.y), (x + 1, y + 1));
            assert_eq!(Some(cell), source.raster_at((x + 1, y + 1).into()));
            assert_eq!(target.get((x, y).into()), source.get((x + 1, y + 1).into()));
        }
        assert!(!target.has_unsupported_raster_composition());
        let mut second = Buffer::new((1, 1).into());
        blit_region(
            &mut Chunk::new(&mut second, Area::new((0, 0).into(), (1, 1).into())).unwrap(),
            &target,
            BlitRegion::new(2, 2, 0, 0, 1, 1),
            BlitOptions::default(),
        );
        let cell = second.raster_at((0, 0).into()).unwrap();
        assert_eq!(cell.target, (5, 4).into());
        assert_eq!((cell.x, cell.y), (3, 3));
    }

    #[test]
    fn raster_blits_follow_text_and_wide_grapheme_draw_order() {
        let source = raster_source((3, 1).into());
        let mut target = Buffer::new((3, 1).into());
        target.set((0, 0).into(), Stylized::plain('界')).unwrap();
        {
            let mut chunk =
                Chunk::new(&mut target, Area::new((0, 0).into(), (3, 1).into())).unwrap();
            blit_region(
                &mut chunk,
                &source,
                BlitRegion::new(0, 0, 0, 0, 3, 1),
                BlitOptions::default().skip_blank(),
            );
            assert_eq!(chunk.buffer().raster_cells().count(), 3);
            chunk.set_forced(1, 0, Stylized::plain('界')).unwrap();
            assert_eq!(chunk.buffer().raster_cells().count(), 1);
            blit_region(
                &mut chunk,
                &source,
                BlitRegion::new(2, 0, 2, 0, 1, 1),
                BlitOptions::default(),
            );
        }
        assert_eq!(cell_char(&target, 1, 0), Some(' '));
        assert!(target.raster_at((0, 0).into()).is_some());
        assert!(target.raster_at((1, 0).into()).is_none());
        assert_eq!(
            target.raster_at((2, 0).into()),
            source.raster_at((2, 0).into())
        );
    }

    #[test]
    fn dirty_only_blits_compare_raster_ownership_even_with_identical_fallbacks() {
        let mut source = raster_source((2, 1).into());
        let mut changed = source.raster_at((0, 0).into()).unwrap().clone();
        let unchanged = source.raster_at((1, 0).into()).unwrap().clone();
        changed.x = 1;
        source.clear();
        source.set_raster((0, 0).into(), changed.clone()).unwrap();
        source.set_raster((1, 0).into(), unchanged).unwrap();
        let region = BlitRegion::new(0, 0, 0, 0, 2, 1);
        let options = BlitOptions::default().dirty_only();
        let mut visited = Vec::new();
        let stats = for_each_blit_cell(&source, region, Some((2, 1).into()), options, |cell| {
            visited.push(cell.dest_x)
        });
        assert_eq!(visited, vec![0]);
        assert_eq!(stats.skipped_clean, 1);
        assert!(stats.unsupported_raster_composition);
        let mut target = Buffer::new((2, 1).into());
        blit_region(
            &mut Chunk::new(&mut target, Area::new((0, 0).into(), (2, 1).into())).unwrap(),
            &source,
            region,
            options,
        );
        assert_eq!(target.raster_at((0, 0).into()), Some(&changed));
        assert!(!target.has_unsupported_raster_composition());
        source.clear();
        let fallback = source.previous().unwrap()[0].content.clone();
        source.set((0, 0).into(), fallback).unwrap();
        blit_region(
            &mut Chunk::new(&mut target, Area::new((0, 0).into(), (2, 1).into())).unwrap(),
            &source,
            region,
            options,
        );
        assert!(!target.has_raster());
    }

    #[test]
    fn effects_use_stored_fallback_and_report_unsupported_composition() {
        let source = raster_source((2, 1).into());
        let mut target = Buffer::new((2, 1).into());
        let mut chunk = Chunk::new(&mut target, Area::new((0, 0).into(), (2, 1).into())).unwrap();
        let stats = for_each_blit_cell(
            &source,
            BlitRegion::new(0, 0, 0, 0, 2, 1),
            Some((2, 1).into()),
            BlitOptions::default(),
            |cell| {
                assert_eq!(
                    Some(&cell.content),
                    source.get((cell.source_x, cell.source_y).into())
                );
                chunk
                    .set_forced(cell.dest_x, cell.dest_y, cell.content)
                    .unwrap();
            },
        );
        assert!(stats.unsupported_raster_composition);
        chunk.mark_unsupported_raster_composition();
        assert!(!target.has_raster());
        assert!(target.has_unsupported_raster_composition());
        let mut outer = Buffer::new((2, 1).into());
        blit_region(
            &mut Chunk::new(&mut outer, Area::new((0, 0).into(), (2, 1).into())).unwrap(),
            &target,
            BlitRegion::new(0, 0, 0, 0, 2, 1),
            BlitOptions::default(),
        );
        assert!(outer.has_unsupported_raster_composition());
        let clipped = for_each_blit_cell(
            &source,
            BlitRegion::new(0, 0, -3, 0, 2, 1),
            Some((2, 1).into()),
            BlitOptions::default(),
            |_| panic!("clipped cell visited"),
        );
        assert!(!clipped.unsupported_raster_composition);
    }

    #[test]
    fn headless_and_reused_offscreen_buffers_keep_raster_results_without_stale_owners() {
        let canvas = crate::render::raster::RasterCanvas::new(2, 2, Default::default()).unwrap();
        let area = Area::new((1, 1).into(), (2, 1).into());
        let snapshot = with_reused_offscreen(
            area,
            Some(987_123),
            |chunk| {
                chunk.raster(&canvas, Default::default()).unwrap();
                chunk.mark_unsupported_raster_composition();
            },
            Clone::clone,
        )
        .unwrap();
        assert_eq!(snapshot.raster_cells().count(), 2);
        assert_eq!(snapshot.raster_at((2, 1).into()).unwrap().x, 1);
        assert!(snapshot.get((2, 1).into()).is_some());
        OFFSCREEN_CACHE.with(|cache| {
            let cache = cache.borrow();
            let cached = cache
                .iter()
                .find(|entry| entry.key == Some(987_123))
                .unwrap();
            assert!(!cached.buffer.has_raster());
            assert_eq!(cached.buffer.raster_cells().count(), 0);
            assert!(cached.buffer.previous().is_none());
            assert!(cached.buffer.previous_raster_at((2, 1).into()).is_none());
            assert!(!cached.buffer.has_unsupported_raster_composition());
            assert!(cached
                .buffer
                .content()
                .iter()
                .all(|cell| *cell == Cell::space()));
        });
        with_reused_offscreen(
            area,
            Some(987_123),
            |_| {},
            |buffer| {
                assert!(!buffer.has_raster());
                assert!(buffer.previous().is_none());
                assert!(buffer.previous_raster_at((2, 1).into()).is_none());
                assert!(!buffer.has_unsupported_raster_composition());
            },
        )
        .unwrap();
        assert_eq!(snapshot.raster_cells().count(), 2);
    }

    #[test]
    fn recycling_releases_current_and_previous_raster_snapshots() {
        OFFSCREEN_CACHE.with(|cache| cache.borrow_mut().clear());
        let mut buffer = raster_source((2, 1).into());
        let owner = buffer.raster_at((0, 0).into()).unwrap().clone();
        buffer.mark_unsupported_raster_composition();
        buffer.clear();
        buffer.set_raster((1, 0).into(), owner).unwrap();
        buffer.mark_unsupported_raster_composition();
        assert!(buffer.has_raster());
        assert!(buffer.previous_raster_at((0, 0).into()).is_some());
        recycle_offscreen(CachedOffscreen { key: None, buffer });
        OFFSCREEN_CACHE.with(|cache| {
            let cache = cache.borrow();
            assert_eq!(cache.len(), 1);
            let buffer = &cache[0].buffer;
            assert!(!buffer.has_raster());
            assert!(buffer.previous().is_none());
            for x in 0..2 {
                assert!(buffer.raster_at((x, 0).into()).is_none());
                assert!(buffer.previous_raster_at((x, 0).into()).is_none());
            }
            assert!(!buffer.has_unsupported_raster_composition());
            assert!(!buffer.previous_has_unsupported_raster_composition());
            assert!(buffer.content().iter().all(|cell| *cell == Cell::space()));
        });
    }

    #[test]
    fn offscreen_pool_bounds_changing_keys_and_sizes_and_reuses_text_storage() {
        OFFSCREEN_CACHE.with(|cache| cache.borrow_mut().clear());
        for i in 0..24u16 {
            let area = Area::new((0, 0).into(), (i + 1, 1).into());
            let key = Some((i % 3) as u64);
            let ptr = with_reused_offscreen(
                area,
                key,
                |chunk| {
                    chunk.set_forced(0, 0, Stylized::plain('x')).unwrap();
                },
                |buffer| {
                    assert_eq!(buffer.get((0, 0).into()).unwrap().as_char(), Some('x'));
                    buffer.content().as_ptr()
                },
            )
            .unwrap();
            let reused = with_reused_offscreen(
                area,
                key,
                |_| {},
                |buffer| {
                    assert_eq!(buffer.get((0, 0).into()).unwrap().as_char(), Some(' '));
                    assert!(buffer.previous().is_none());
                    buffer.content().as_ptr()
                },
            )
            .unwrap();
            assert_eq!(reused, ptr);
            OFFSCREEN_CACHE.with(|cache| {
                let cache = cache.borrow();
                assert_eq!(
                    cache.len(),
                    (i as usize + 1).min(OFFSCREEN_CACHE_MAX_ENTRIES)
                );
                assert_eq!(cache.last().unwrap().buffer.size(), area.size());
                let oldest_width = (i as usize + 1).saturating_sub(OFFSCREEN_CACHE_MAX_ENTRIES) + 1;
                assert_eq!(cache[0].buffer.size().width as usize, oldest_width);
                assert!(
                    cache
                        .iter()
                        .map(|entry| entry.buffer.storage_bytes())
                        .sum::<usize>()
                        <= OFFSCREEN_CACHE_MAX_BYTES
                );
            });
        }
    }

    #[test]
    fn offscreen_pool_budget_counts_retained_raster_metadata_capacity() {
        use crate::render::raster::{RasterCanvas, RasterCell};
        OFFSCREEN_CACHE.with(|cache| cache.borrow_mut().clear());
        let area = Area::new((0, 0).into(), (256, 128).into());
        let owner = RasterCell {
            canvas: RasterCanvas::new(1, 1, Default::default()).unwrap(),
            target: (1, 1).into(),
            x: 0,
            y: 0,
            options: Default::default(),
        };
        for key in 0..4 {
            let bytes = with_reused_offscreen(
                area,
                Some(key),
                |chunk| {
                    chunk.set_raster_cell(0, 0, owner.clone()).unwrap();
                },
                Buffer::storage_bytes,
            )
            .unwrap();
            assert!(bytes > area.area() * std::mem::size_of::<Cell>());
            OFFSCREEN_CACHE.with(|cache| {
                let cache = cache.borrow();
                let capacity = (OFFSCREEN_CACHE_MAX_BYTES / bytes).min(OFFSCREEN_CACHE_MAX_ENTRIES);
                assert_eq!(cache.len(), (key as usize + 1).min(capacity));
                assert!(
                    cache
                        .iter()
                        .map(|entry| entry.buffer.storage_bytes())
                        .sum::<usize>()
                        <= OFFSCREEN_CACHE_MAX_BYTES
                );
                assert!(cache.iter().all(|entry| !entry.buffer.has_raster()));
                assert!(cache.iter().all(|entry| entry.buffer.previous().is_none()));
                assert_eq!(cache.last().unwrap().key, Some(key));
            });
        }
    }

    #[test]
    fn oversized_offscreen_is_rendered_but_not_pooled() {
        OFFSCREEN_CACHE.with(|cache| cache.borrow_mut().clear());
        let small = Area::new((0, 0).into(), (1, 1).into());
        with_reused_offscreen(small, Some(0), |_| {}, |_| {}).unwrap();
        let height = (OFFSCREEN_CACHE_MAX_BYTES / (512 * std::mem::size_of::<Cell>()) + 1) as u16;
        let area = Area::new((0, 0).into(), (512, height).into());
        let result = with_reused_offscreen(
            area,
            Some(1),
            |chunk| {
                chunk.set_forced(0, 0, Stylized::plain('x')).unwrap();
            },
            |buffer| {
                assert!(buffer.storage_bytes() > OFFSCREEN_CACHE_MAX_BYTES);
                buffer.get((0, 0).into()).unwrap().as_char()
            },
        )
        .unwrap();
        assert_eq!(result, Some('x'));
        OFFSCREEN_CACHE.with(|cache| {
            let cache = cache.borrow();
            assert_eq!(cache.len(), 1);
            assert_eq!(cache[0].key, Some(0));
        });
    }

    #[test]
    fn blit_extreme_bounds_are_safe() {
        let source = raster_source((2, 1).into());
        for region in [
            BlitRegion::new(usize::MAX, 0, 0, 0, usize::MAX, 1),
            BlitRegion::new(0, usize::MAX, 0, 0, 2, usize::MAX),
            BlitRegion::new(0, 0, i32::MAX, i32::MAX, usize::MAX, usize::MAX),
            BlitRegion::new(0, 0, i32::MIN, i32::MIN, usize::MAX, usize::MAX),
        ] {
            let stats = for_each_blit_cell(&source, region, None, BlitOptions::default(), |_| {
                panic!("out of bounds")
            });
            assert_eq!(stats.visited, 0);
        }
        assert!(
            render_to_offscreen(Area::new((u16::MAX, 0).into(), (2, 1).into()), |_| panic!(
                "invalid area"
            ))
            .is_none()
        );
    }

    fn render_widget_into(
        widget: &impl crate::tui::widget::Widget<()>,
        buffer: &mut Buffer,
        area: Area,
        store: &crate::tui::widget::WidgetStore,
    ) {
        use crate::tui::{animation::AnimationStore, style::Theme, widget::RenderCtx};
        let animations = AnimationStore::new();
        let theme = Theme::dark();
        let geometry = RefCell::new(std::collections::HashMap::new());
        let ctx = RenderCtx::new(store, &animations, &theme, None, &geometry);
        widget.render(&mut Chunk::new(buffer, area).unwrap(), &ctx);
    }

    #[test]
    fn raster_scroll_view_preserves_full_target_and_clamps_scroll_offsets() {
        use crate::render::raster::{RasterCanvas, Rgb};
        use crate::tui::{
            layout::{scroll_view, ScrollState, ScrollbarVisibility},
            widget::{WidgetId, WidgetStore},
            widgets::raster_canvas,
        };
        let canvas = RasterCanvas::new(6, 8, Rgb(50, 100, 150)).unwrap();
        let view = scroll_view(raster_canvas::<()>(canvas.clone()))
            .both()
            .content_width(6)
            .content_height(4)
            .scrollbars(ScrollbarVisibility::Hidden);
        let area = Area::new((1, 1).into(), (3, 2).into());
        let mut buffer = Buffer::new((5, 4).into());
        let mut store = WidgetStore::new();
        for (offset_x, offset_y) in [(0, 0), (2, 1), (usize::MAX, usize::MAX)] {
            *store.get_or_default::<ScrollState>(WidgetId::root()) = ScrollState {
                offset_x,
                offset_y,
                ..ScrollState::default()
            };
            buffer.clear();
            render_widget_into(&view, &mut buffer, area, &store);
            assert_eq!(buffer.raster_cells().count(), 6);
            for (x, y, cell) in buffer.raster_cells() {
                assert!((1..4).contains(&x) && (1..3).contains(&y));
                assert_eq!(cell.target, (6, 4).into());
                assert_eq!(
                    (cell.x, cell.y),
                    (
                        x - 1 + offset_x.min(3) as u16,
                        y - 1 + offset_y.min(2) as u16
                    )
                );
                assert_eq!(cell.canvas, canvas);
                assert_eq!(buffer.get((x, y).into()), Some(&cell.fallback()));
            }
            assert!(!buffer.has_unsupported_raster_composition());
        }
    }

    #[test]
    fn raster_overlay_popup_moves_and_closes_without_stale_ownership() {
        use crate::render::raster::{RasterCanvas, Rgb};
        use crate::tui::{
            layout::{overlay, OverlayAnchor, OverlayLayer},
            style::Color,
            widget::WidgetStore,
            widgets::{label, raster_canvas},
        };
        let canvas = RasterCanvas::new(5, 4, Rgb(50, 100, 150)).unwrap();
        let area = Area::new((0, 0).into(), (5, 2).into());
        let mut buffer = Buffer::new(area.size());
        let store = WidgetStore::new();
        for popup_x in [Some(1), Some(2), None] {
            let mut view = overlay(raster_canvas::<()>(canvas.clone()));
            if let Some(x) = popup_x {
                view = view.layer(
                    OverlayLayer::new(label("XX").bg(Color::Blue))
                        .floating(OverlayAnchor::TopLeft)
                        .size(2, 1)
                        .offset(x, 0),
                );
            }
            buffer.clear();
            render_widget_into(&view, &mut buffer, area, &store);
            for y in 0..2 {
                for x in 0..5 {
                    let covered = popup_x
                        .is_some_and(|left| y == 0 && x >= left as u16 && x < left as u16 + 2);
                    if covered {
                        assert!(buffer.raster_at((x, y).into()).is_none());
                        assert_eq!(cell_char(&buffer, x, y), Some('X'));
                    } else {
                        let cell = buffer.raster_at((x, y).into()).unwrap();
                        assert_eq!(cell.canvas, canvas);
                        assert_eq!((cell.x, cell.y), (x, y));
                        assert_eq!(cell.target, area.size());
                    }
                }
            }
            assert_eq!(
                buffer.raster_cells().count(),
                if popup_x.is_some() { 8 } else { 10 }
            );
            assert!(!buffer.has_unsupported_raster_composition());
        }
    }

    fn cell_char(buffer: &Buffer, x: u16, y: u16) -> Option<char> {
        let index = (y * buffer.size().width + x) as usize;
        buffer.content().get(index)?.content.as_char()
    }
}
