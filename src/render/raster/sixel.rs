//! Position-independent Sixel tiles for already-visible raster cells.
//!
//! The parent gates this module with `cfg(feature = "sixel")` and owns cursor
//! placement, clipping, terminal capabilities, and graphics cleanup. Arbitrary
//! cell heights use a partial final band: P2=1 leaves unset padding transparent,
//! raster attributes keep the actual dimensions, and no trailing band advance
//! is emitted. Transparency alone does not prevent bottom scrolling: terminals
//! such as Windows Terminal may check the full band's height. Inline reserves
//! ceil(((6 - cell_height % 6) % 6) / cell_height) guard rows in the parent.
//! The host must support safe positioned graphics and padding that does not
//! erase adjacent cells. Fullscreen can prepend transparent rows and anchor a
//! bottom-edge tile higher so its final six-pixel band fits the viewport.

use std::collections::VecDeque;
use std::io::{self, Write};
use std::sync::Arc;

use super::{RasterCell, RasterOptions, Rgb};

const MAX_CELL_DIMENSION: u16 = 1024;
const MAX_TILE_PIXELS: usize = 64 * 1024;
const MAX_TILE_BYTES: usize = 1024 * 1024;
const MAX_CACHE_BYTES: usize = 4 * 1024 * 1024;
const MAX_CACHE_ENTRIES: usize = 1024;
// Reserve space for keys, queue storage, and Arc headers even for tiny tiles.
const ENTRY_OVERHEAD: usize = std::mem::size_of::<Entry>() + 2 * std::mem::size_of::<usize>();
const MAX_CACHE_PAYLOAD: usize = MAX_CACHE_BYTES - MAX_CACHE_ENTRIES * ENTRY_OVERHEAD;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Key {
    canvas_id: u64,
    version: u64,
    x: u16,
    y: u16,
    target_width: u16,
    target_height: u16,
    options: RasterOptions,
    cell_width: u16,
    cell_height: u16,
    top_padding: u16,
}

impl Key {
    fn new(cell: &RasterCell, cell_width: u16, cell_height: u16) -> Self {
        Self {
            canvas_id: cell.canvas.id(),
            version: cell.canvas.version(),
            x: cell.x,
            y: cell.y,
            target_width: cell.target.width,
            target_height: cell.target.height,
            options: cell.options,
            cell_width,
            cell_height,
            top_padding: 0,
        }
    }
}

#[derive(Debug, Clone)]
struct Entry {
    key: Key,
    encoded: Arc<[u8]>,
}

/// Bounded LRU of encoded cell tiles, without retained source canvas snapshots.
///
/// At most 1,024 entries and approximately 4 MiB including entry overhead are
/// retained. Returned Arcs can outlive eviction; their external owners are not
/// charged to this cache. Clones share encoded bytes but have independent LRU
/// order and counters. No terminal display state is cached here.
#[derive(Debug, Clone, Default)]
pub(super) struct SixelCache {
    entries: VecDeque<Entry>,
    bytes: usize,
    encodes: u64,
    hits: u64,
}

impl SixelCache {
    /// Encodes the original cell crop at the supplied physical pixel metrics.
    ///
    /// Every actual pixel is opaque after RGB332 quantization, including black
    /// and letterboxing. Dimensions must be 1..=1024, their product at most
    /// 65,536, and encoded output at most 1 MiB. Limit violations return
    /// `InvalidInput`, without changing cache entries or counters.
    #[cfg(test)]
    pub(super) fn encode(
        &mut self,
        cell: &RasterCell,
        cell_width: u16,
        cell_height: u16,
    ) -> io::Result<Arc<[u8]>> {
        self.encode_padded(cell, cell_width, cell_height, 0)
    }

    pub(super) fn encode_padded(
        &mut self,
        cell: &RasterCell,
        cell_width: u16,
        cell_height: u16,
        top_padding: u16,
    ) -> io::Result<Arc<[u8]>> {
        validate_dimensions(cell_width, cell_height)?;
        if u32::from(top_padding) > 5 * u32::from(cell_height) {
            return Err(invalid_input("Sixel top padding exceeds five cell rows"));
        }
        let mut key = Key::new(cell, cell_width, cell_height);
        key.top_padding = top_padding;
        if let Some(index) = self.entries.iter().position(|entry| entry.key == key) {
            let entry = self.entries.remove(index).expect("existing cache entry");
            let encoded = Arc::clone(&entry.encoded);
            self.entries.push_back(entry);
            self.hits = self.hits.saturating_add(1);
            return Ok(encoded);
        }

        let encoded: Arc<[u8]> =
            encode_tile_padded(cell, cell_width, cell_height, top_padding)?.into();
        self.insert(key, Arc::clone(&encoded))?;
        self.encodes = self.encodes.saturating_add(1);
        Ok(encoded)
    }

    fn insert(&mut self, key: Key, encoded: Arc<[u8]>) -> io::Result<()> {
        // Reserve before eviction so allocation failure leaves the cache intact.
        if self.entries.len() == self.entries.capacity() && self.entries.len() < MAX_CACHE_ENTRIES {
            let capacity = (self.entries.capacity().max(2) * 2).min(MAX_CACHE_ENTRIES);
            self.entries
                .try_reserve_exact(capacity - self.entries.len())
                .map_err(allocation_error)?;
        }
        while self.entries.len() >= MAX_CACHE_ENTRIES
            || self.bytes + encoded.len() > MAX_CACHE_PAYLOAD
        {
            let evicted = self.entries.pop_front().expect("bounded tile fits cache");
            self.bytes -= evicted.encoded.len();
        }
        self.bytes += encoded.len();
        self.entries.push_back(Entry { key, encoded });
        Ok(())
    }

    /// Successful encodes (misses), including entries subsequently evicted.
    pub(super) fn encodes(&self) -> u64 {
        self.encodes
    }

    /// Successful lookups served without sampling or encoding.
    pub(super) fn hits(&self) -> u64 {
        self.hits
    }

    /// Encoded payload bytes currently retained, excluding entry overhead.
    pub(super) fn bytes(&self) -> usize {
        self.bytes
    }

    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }
}

fn invalid_input(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn allocation_error(error: std::collections::TryReserveError) -> io::Error {
    io::Error::new(io::ErrorKind::OutOfMemory, error)
}

fn validate_dimensions(width: u16, height: u16) -> io::Result<()> {
    if width == 0 || height == 0 || width > MAX_CELL_DIMENSION || height > MAX_CELL_DIMENSION {
        return Err(invalid_input("Sixel cell dimensions must be in 1..=1024"));
    }
    if usize::from(width)
        .checked_mul(usize::from(height))
        .is_none_or(|pixels| pixels > MAX_TILE_PIXELS)
    {
        return Err(invalid_input("Sixel cell exceeds 65536 pixels"));
    }
    Ok(())
}

fn zeroed(len: usize) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(len).map_err(allocation_error)?;
    bytes.resize(len, 0);
    Ok(bytes)
}

fn palette_index(Rgb(red, green, blue): Rgb) -> u8 {
    (red & 0xe0) | ((green >> 3) & 0x1c) | (blue >> 6)
}

/// Sixel RGB components are integer percentages, not eight-bit channel values.
fn palette_color(index: u8) -> [u8; 3] {
    [
        ((u16::from(index >> 5) * 100 + 3) / 7) as u8,
        ((u16::from((index >> 2) & 7) * 100 + 3) / 7) as u8,
        ((u16::from(index & 3) * 100 + 1) / 3) as u8,
    ]
}

/// Enforces the output limit during encoding, not after a large allocation.
#[derive(Default)]
struct TileBytes(Vec<u8>);

impl Write for TileBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let required = self
            .0
            .len()
            .checked_add(bytes.len())
            .filter(|&len| len <= MAX_TILE_BYTES)
            .ok_or_else(|| invalid_input("Sixel encoded tile exceeds 1 MiB"))?;
        if required > self.0.capacity() {
            let capacity = required
                .max(self.0.capacity().max(256) * 2)
                .min(MAX_TILE_BYTES);
            self.0
                .try_reserve_exact(capacity - self.0.len())
                .map_err(allocation_error)?;
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn encode_tile_padded(
    cell: &RasterCell,
    cell_width: u16,
    cell_height: u16,
    top_padding: u16,
) -> io::Result<Vec<u8>> {
    let width = usize::from(cell_width);
    let height = usize::from(cell_height);
    let mut pixels = zeroed(width * height)?;
    let mut used = [false; 256];
    for (offset, pixel) in pixels.iter_mut().enumerate() {
        *pixel = palette_index(cell.sample(
            (offset % width) as u32,
            (offset / width) as u32,
            cell_width,
            cell_height,
        ));
        used[usize::from(*pixel)] = true;
    }

    let mut output = TileBytes::default();
    // Transparent *unset bits* avoid filling the final band's padding. All real
    // pixels, including the presentation background, are explicitly set below.
    // This does not prevent full-band scroll checks; inline needs a bottom margin.
    let padding = usize::from(top_padding);
    let encoded_height = padding + height;
    // P1=7 starts at 1:1, unlike P1=0's initial 2:1. Windows Terminal retains
    // that initial 12-pixel segment for its first scroll check even after DECGRA.
    write!(output, "\x1bP7;1;0q\"1;1;{width};{encoded_height}")?;
    for (index, &present) in used.iter().enumerate() {
        if present {
            let [red, green, blue] = palette_color(index as u8);
            write!(output, "#{index};2;{red};{green};{blue}")?;
        }
    }

    // Only one band's color planes are held at a time (at most 256 KiB).
    let mut planes = zeroed(256 * width)?;
    for top in (0..encoded_height).step_by(6) {
        planes.fill(0);
        for y in top.max(padding)..(top + 6).min(encoded_height) {
            for x in 0..width {
                let index = usize::from(pixels[(y - padding) * width + x]);
                planes[index * width + x] |= 1 << (y - top);
            }
        }
        let mut first_color = true;
        for (index, plane) in planes.chunks_exact(width).enumerate() {
            let Some(last) = plane.iter().rposition(|&mask| mask != 0) else {
                continue;
            };
            if !first_color {
                output.write_all(b"$")?;
            }
            first_color = false;
            write!(output, "#{index}")?;
            write_plane(&mut output, &plane[..=last])?;
        }
        // Avoid an extra advance past the final band. The band itself may still
        // trigger bottom scrolling without the parent's inline guard rows.
        if top + 6 < encoded_height {
            output.write_all(b"-")?;
        }
    }
    output.write_all(b"\x1b\\")?;
    Ok(output.0)
}

fn write_plane(output: &mut TileBytes, plane: &[u8]) -> io::Result<()> {
    let mut start = 0;
    while start < plane.len() {
        let mask = plane[start];
        let mut end = start + 1;
        while end < plane.len() && plane[end] == mask {
            end += 1;
        }
        let count = end - start;
        let character = mask + b'?';
        if count >= 4 {
            write!(output, "!{count}")?;
            output.write_all(&[character])?;
        } else {
            output.write_all(&[character; 3][..count])?;
        }
        start = end;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::area::Size;
    use crate::render::raster::{RasterAlign, RasterCanvas, RasterFallback, RasterScale};

    fn cell(width: u32, height: u32) -> RasterCell {
        RasterCell {
            canvas: RasterCanvas::new(width, height, Rgb(0, 0, 0)).unwrap(),
            target: Size::from((1, 1)),
            x: 0,
            y: 0,
            options: RasterOptions {
                scale: RasterScale::Stretch,
                ..RasterOptions::default()
            },
        }
    }

    struct Decoded {
        width: usize,
        height: usize,
        pixels: Vec<Option<[u8; 3]>>,
        definitions: usize,
        bands: usize,
    }

    fn number(input: &[u8], position: &mut usize) -> usize {
        let start = *position;
        let mut value = 0;
        while *position < input.len() && input[*position].is_ascii_digit() {
            value = value * 10 + usize::from(input[*position] - b'0');
            *position += 1;
        }
        assert!(*position > start, "missing numeric parameter");
        value
    }

    fn parameter(input: &[u8], position: &mut usize) -> usize {
        assert_eq!(input[*position], b';');
        *position += 1;
        number(input, position)
    }

    // Independent protocol parser: unset pixels stay None, so transparent holes,
    // repeated writes, and any painted padding are errors, not hidden by black.
    fn decode(input: &[u8]) -> Decoded {
        let prefix = b"\x1bP7;1;0q\"";
        assert!(input.starts_with(prefix));
        assert!(input.ends_with(b"\x1b\\"));
        assert!(!input.ends_with(b"-\x1b\\"));
        let mut position = prefix.len();
        assert_eq!(number(input, &mut position), 1);
        assert_eq!(parameter(input, &mut position), 1);
        let width = parameter(input, &mut position);
        let height = parameter(input, &mut position);
        let mut result = Decoded {
            width,
            height,
            pixels: vec![None; width * height],
            definitions: 0,
            bands: 1,
        };
        let mut palette = [None; 256];
        let (mut x, mut y, mut selected) = (0, 0, None);
        while position < input.len() - 2 {
            let command = input[position];
            position += 1;
            match command {
                b'#' => {
                    let index = number(input, &mut position);
                    assert!(index < 256);
                    selected = Some(index);
                    if input[position] == b';' {
                        assert_eq!(parameter(input, &mut position), 2);
                        let rgb = std::array::from_fn(|_| {
                            let channel = parameter(input, &mut position);
                            assert!(channel <= 100);
                            channel as u8
                        });
                        assert!(palette[index].replace(rgb).is_none());
                        result.definitions += 1;
                    }
                }
                b'$' => x = 0,
                b'-' => {
                    x = 0;
                    y += 6;
                    assert!(y < height);
                    result.bands += 1;
                }
                b'!' | b'?'..=b'~' => {
                    let (count, mask) = if command == b'!' {
                        let count = number(input, &mut position);
                        let character = input[position];
                        position += 1;
                        assert!((b'?'..=b'~').contains(&character));
                        assert!(count > 0);
                        (count, character - b'?')
                    } else {
                        (1, command - b'?')
                    };
                    assert!(x + count <= width);
                    let color = palette[selected.unwrap()].unwrap();
                    for column in x..x + count {
                        for bit in 0..6 {
                            if mask & (1 << bit) != 0 {
                                assert!(y + bit < height, "painted band padding");
                                let pixel = &mut result.pixels[(y + bit) * width + column];
                                assert!(pixel.replace(color).is_none(), "painted pixel twice");
                            }
                        }
                    }
                    x += count;
                }
                _ => panic!("unexpected Sixel command {command}"),
            }
        }
        result
    }

    fn expected(Rgb(red, green, blue): Rgb) -> [u8; 3] {
        // Deliberately independent of the encoder's palette helpers.
        [
            (((red as usize / 32) * 100 + 3) / 7) as u8,
            (((green as usize / 32) * 100 + 3) / 7) as u8,
            (((blue as usize / 64) * 100 + 1) / 3) as u8,
        ]
    }

    fn assert_pixels(cell: &RasterCell, width: u16, height: u16, bytes: &[u8]) -> Decoded {
        let decoded = decode(bytes);
        assert!(
            decoded.pixels.iter().all(Option::is_some),
            "transparent holes"
        );
        assert_eq!(
            (decoded.width, decoded.height),
            (width as usize, height as usize)
        );
        assert_eq!(decoded.bands, usize::from(height).div_ceil(6));
        for y in 0..height {
            for x in 0..width {
                assert_eq!(
                    decoded.pixels[usize::from(y) * usize::from(width) + usize::from(x)],
                    Some(expected(cell.sample(
                        u32::from(x),
                        u32::from(y),
                        width,
                        height
                    )))
                );
            }
        }
        decoded
    }

    #[test]
    fn round_trips_all_colors_and_every_partial_band_height() {
        for height in 1..=25 {
            let mut cell = cell(256, height);
            for y in 0..height {
                for x in 0..256 {
                    let index = ((x + y * 37) % 256) as u8;
                    cell.canvas.set_pixel(
                        x as i32,
                        y as i32,
                        Rgb(index & 0xe0, (index & 0x1c) << 3, (index & 3) << 6),
                    );
                }
            }
            let encoded = SixelCache::default()
                .encode(&cell, 256, height as u16)
                .unwrap();
            assert_eq!(
                assert_pixels(&cell, 256, height as u16, &encoded).definitions,
                256
            );
            let again = SixelCache::default()
                .encode(&cell, 256, height as u16)
                .unwrap();
            assert_eq!(encoded, again);
        }
    }

    #[test]
    fn ten_by_twenty_tiles_preserve_exact_sampling_and_transparent_padding() {
        let mut cell = cell(20, 40);
        for y in 0..40 {
            for x in 0..20 {
                cell.canvas
                    .set_pixel(x, y, Rgb((x * 13) as u8, (y * 6) as u8, 145));
            }
        }
        cell.target = (2, 2).into();
        let mut cache = SixelCache::default();
        for y in 0..2 {
            for x in 0..2 {
                cell.x = x;
                cell.y = y;
                let bytes = cache.encode(&cell, 10, 20).unwrap();
                // The decoder rejects painted padding rather than clipping it.
                let decoded = assert_pixels(&cell, 10, 20, &bytes);
                assert_eq!(decoded.bands, 4);
                for py in 0..20 {
                    for px in 0..10 {
                        let source_x = usize::from(x) * 10 + px;
                        let source_y = usize::from(y) * 20 + py;
                        assert_eq!(
                            decoded.pixels[py * 10 + px],
                            Some(expected(Rgb(
                                (source_x * 13) as u8,
                                (source_y * 6) as u8,
                                145
                            )))
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn transparent_prefix_preserves_sampling_and_is_part_of_cache_key() {
        let mut cell = cell(10, 20);
        for y in 0..20 {
            for x in 0..10 {
                cell.canvas
                    .set_pixel(x, y, Rgb((x * 25) as u8, (y * 12) as u8, 200));
            }
        }
        let mut cache = SixelCache::default();
        let plain = cache.encode(&cell, 10, 20).unwrap();
        let reference = decode(&plain);
        for padding in [20, 40, 100] {
            let bytes = cache.encode_padded(&cell, 10, 20, padding).unwrap();
            let decoded = decode(&bytes);
            let prefix = usize::from(padding) * 10;
            assert!(decoded.pixels[..prefix].iter().all(Option::is_none));
            assert_eq!(&decoded.pixels[prefix..], &reference.pixels);
            assert!(Arc::ptr_eq(
                &bytes,
                &cache.encode_padded(&cell, 10, 20, padding).unwrap()
            ));
        }
        assert!(Arc::ptr_eq(&plain, &cache.encode(&cell, 10, 20).unwrap()));
        assert_eq!(cache.encodes(), 4);
        assert!(cache.encode_padded(&cell, 10, 20, 101).is_err());
        assert_eq!(cache.encodes(), 4);
    }

    #[test]
    fn black_is_opaque_and_only_used_palette_entries_are_defined() {
        let cell = cell(1, 1);
        let bytes = SixelCache::default().encode(&cell, 100, 7).unwrap();
        assert_eq!(assert_pixels(&cell, 100, 7, &bytes).definitions, 1);
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(text.contains("!100~"));
        assert!(text.contains("!100@"));
        assert!(!text.contains("#1;2;"));
    }

    #[test]
    fn cell_crops_and_letterboxing_match_sampler() {
        let mut cell = cell(9, 5);
        for y in 0..5 {
            for x in 0..9 {
                cell.canvas
                    .set_pixel(x, y, Rgb((x * 29) as u8, (y * 59) as u8, 145));
            }
        }
        cell.target = Size::from((3, 2));
        cell.options.background = Rgb(255, 0, 255);
        for scale in [
            RasterScale::Fit,
            RasterScale::Stretch,
            RasterScale::Original,
        ] {
            for align in [RasterAlign::Start, RasterAlign::Center, RasterAlign::End] {
                cell.options.scale = scale;
                cell.options.align_x = align;
                cell.options.align_y = align;
                for y in 0..2 {
                    for x in 0..3 {
                        cell.x = x;
                        cell.y = y;
                        for (width, height) in [(7, 13), (1, 1)] {
                            let bytes = SixelCache::default().encode(&cell, width, height).unwrap();
                            assert_pixels(&cell, width, height, &bytes);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn hits_reuse_arcs_and_revisions_and_identities_miss() {
        let cell = cell(2, 2);
        let mut cache = SixelCache::default();
        let first = cache.encode(&cell, 8, 12).unwrap();
        assert!(Arc::ptr_eq(
            &first,
            &cache.encode(&cell.clone(), 8, 12).unwrap()
        ));
        let mut changed = cell.clone();
        changed.canvas.set_pixel(0, 0, Rgb(255, 0, 0));
        let second = cache.encode(&changed, 8, 12).unwrap();
        assert_ne!(first, second);
        let mut branch = cell.clone();
        branch.canvas.set_pixel(0, 0, Rgb(0, 255, 0));
        assert_ne!(second, cache.encode(&branch, 8, 12).unwrap());
        let mut independent = cell.clone();
        independent.canvas = RasterCanvas::new(2, 2, Rgb(0, 0, 0)).unwrap();
        let identical = cache.encode(&independent, 8, 12).unwrap();
        assert_eq!(first, identical);
        assert!(!Arc::ptr_eq(&first, &identical));
        changed.canvas.set_pixel(0, 0, Rgb(255, 0, 0));
        assert!(Arc::ptr_eq(
            &second,
            &cache.encode(&changed, 8, 12).unwrap()
        ));
        assert_eq!((cache.encodes(), cache.hits(), cache.len()), (4, 2, 4));
    }

    #[test]
    fn every_presentation_field_and_physical_metric_is_in_the_key() {
        let mut original = cell(3, 2);
        original.target = Size::from((3, 3));
        let mut cache = SixelCache::default();
        cache.encode(&original, 8, 12).unwrap();
        let mut variants = vec![original.clone(); 9];
        variants[0].x = 1;
        variants[1].y = 1;
        variants[2].target.width = 4;
        variants[3].target.height = 4;
        variants[4].options.background = Rgb(255, 255, 255);
        variants[5].options.scale = RasterScale::Fit;
        variants[6].options.align_x = RasterAlign::End;
        variants[7].options.align_y = RasterAlign::Start;
        variants[8].options.fallback = RasterFallback::Braille;
        for variant in variants {
            cache.encode(&variant, 8, 12).unwrap();
        }
        cache.encode(&original, 9, 12).unwrap();
        cache.encode(&original, 8, 13).unwrap();
        assert_eq!((cache.encodes(), cache.hits(), cache.len()), (12, 0, 12));
    }

    #[test]
    fn destination_position_is_not_encoded_or_cached() {
        let cell = cell(2, 2);
        // Moving an intact RasterCell in a buffer changes only its destination.
        let mut placed = ((1_u16, 2_u16), cell);
        let mut cache = SixelCache::default();
        let first = cache.encode(&placed.1, 8, 12).unwrap();
        placed.0 = (40, 20);
        let second = cache.encode(&placed.1, 8, 12).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!((cache.encodes(), cache.hits()), (1, 1));
        assert_pixels(&placed.1, 8, 12, &second);
    }

    #[test]
    fn rejects_dimensions_and_output_overflow_without_cache_changes() {
        let cell = cell(1, 1);
        let mut cache = SixelCache::default();
        for (width, height) in [
            (0, 6),
            (8, 0),
            (1025, 1),
            (1, 1025),
            (257, 256),
            (u16::MAX, u16::MAX),
        ] {
            assert_eq!(
                cache.encode(&cell, width, height).unwrap_err().kind(),
                io::ErrorKind::InvalidInput
            );
        }
        assert_eq!(
            (cache.encodes(), cache.hits(), cache.bytes(), cache.len()),
            (0, 0, 0, 0)
        );
        for (width, height) in [(1024, 1), (1, 1024), (256, 256)] {
            let bytes = cache.encode(&cell, width, height).unwrap();
            assert_pixels(&cell, width, height, &bytes);
        }
        let mut output = TileBytes::default();
        output.write_all(&vec![b'?'; MAX_TILE_BYTES]).unwrap();
        assert_eq!(
            output.write_all(b"?").unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(output.0.len(), MAX_TILE_BYTES);
        assert!(output.0.capacity() <= MAX_TILE_BYTES);
    }

    fn assert_cache_bounds(cache: &SixelCache) {
        assert!(cache.len() <= MAX_CACHE_ENTRIES);
        assert!(cache.entries.capacity() <= MAX_CACHE_ENTRIES);
        assert!(cache.bytes() + MAX_CACHE_ENTRIES * ENTRY_OVERHEAD <= MAX_CACHE_BYTES);
        assert_eq!(
            cache.bytes(),
            cache
                .entries
                .iter()
                .map(|entry| entry.encoded.len())
                .sum::<usize>()
        );
    }

    #[test]
    fn entry_limit_evicts_least_recently_used_and_keeps_external_arcs_valid() {
        let mut cell = cell(1, 1);
        cell.target.width = 1025;
        let mut cache = SixelCache::default();
        let first = cache.encode(&cell, 1, 1).unwrap();
        for x in 1..1024 {
            cell.x = x;
            cache.encode(&cell, 1, 1).unwrap();
            assert_cache_bounds(&cache);
        }
        cell.x = 0;
        assert!(Arc::ptr_eq(&first, &cache.encode(&cell, 1, 1).unwrap()));
        cell.x = 1024;
        cache.encode(&cell, 1, 1).unwrap();
        assert_eq!(cache.len(), 1024);
        assert_eq!(cache.entries.front().unwrap().key.x, 2);
        cell.x = 1;
        cache.encode(&cell, 1, 1).unwrap();
        assert_eq!(cache.encodes(), 1026);
        assert_eq!(cache.hits(), 1);
        assert_cache_bounds(&cache);
        drop(cache);
        assert_pixels(&cell, 1, 1, &first);
    }

    #[test]
    fn byte_limit_is_independent_of_entry_limit() {
        let mut cell = cell(1, 1);
        let mut cache = SixelCache::default();
        // Synthetic maximum-size payloads exercise the exact cache budget
        // without depending on the compression ratio of a particular image.
        let payload: Arc<[u8]> = vec![b'?'; MAX_TILE_BYTES].into();
        for x in 0..10 {
            cell.x = x;
            cache
                .insert(Key::new(&cell, 1, 1), Arc::clone(&payload))
                .unwrap();
            assert_cache_bounds(&cache);
        }
        assert_eq!(cache.len(), MAX_CACHE_PAYLOAD / MAX_TILE_BYTES);
        assert_eq!(cache.entries.back().unwrap().key.x, 9);
        assert_eq!(payload.len(), MAX_TILE_BYTES);
    }

    #[test]
    fn cache_does_not_retain_source_storage_and_clones_share_only_encoded_bytes() {
        let mut cell = cell(3, 3);
        let mut cache = SixelCache::default();
        let encoded = cache.encode(&cell, 8, 12).unwrap();
        let source_pointer = cell.canvas.pixels().as_ptr();
        let mut cloned = cache.clone();
        cell.canvas.set_pixel(0, 0, Rgb(255, 255, 255));
        // A retained source snapshot would force COW to allocate another buffer.
        assert_eq!(source_pointer, cell.canvas.pixels().as_ptr());
        assert!(Arc::ptr_eq(&encoded, &cloned.entries[0].encoded));
        cloned.encode(&cell, 8, 12).unwrap();
        assert_eq!((cache.len(), cloned.len()), (1, 2));
        assert_eq!((cache.encodes(), cloned.encodes()), (1, 2));
    }
}
