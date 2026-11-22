//! Kitty graphics for fullscreen and inline sessions. A resource is a sampled block in target-local
//! coordinates. Its placements are exact, cell-aligned crops of that block.
use super::output::CellPixels;
use super::presenter::{check_limits, unsupported, Payload, RasterPlan, MAX_FRAME_BYTES};
use super::{RasterCell, RasterOptions, Rgb};
use crate::render::{
    area::{Position, Size},
    buffer::Buffer,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::{
    collections::{HashMap, HashSet},
    hash::{BuildHasher, RandomState},
    io::{self, Write},
    sync::{
        atomic::{AtomicU32, Ordering},
        OnceLock,
    },
};

// Disjoint IDs across live renderers in this process, with a randomized
// starting point to minimize collisions with unrelated terminal clients.
fn image_id() -> io::Result<u32> {
    static NEXT: AtomicU32 = AtomicU32::new(1);
    static SALT: OnceLock<u32> = OnceLock::new();
    let salt = *SALT.get_or_init(|| RandomState::new().hash_one("rsille") as u32);
    loop {
        let value = NEXT
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| unsupported("Kitty image IDs exhausted"))?
            ^ salt;
        if value != 0 {
            return Ok(value);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Rect {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Surface {
    canvas: u64,
    version: u64,
    target: Size,
    options: RasterOptions,
    block: Rect,
}

impl Surface {
    fn new(cell: &RasterCell, pixels: CellPixels) -> Self {
        // Only touched blocks of very large/offscreen targets are allocated.
        let cols = (512 / pixels.width).max(1);
        let rows = (512 / pixels.height).max(1);
        let x = cell.x / cols * cols;
        let y = cell.y / rows * rows;
        Self {
            canvas: cell.canvas.id(),
            version: cell.canvas.version(),
            target: cell.target,
            options: cell.options,
            block: Rect {
                x,
                y,
                w: cols.min(cell.target.width - x),
                h: rows.min(cell.target.height - y),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Placement {
    image: u32,
    screen: Rect,
    crop_x: u16,
    crop_y: u16,
    layer: usize,
}

#[derive(Debug, Default)]
struct Display {
    images: HashMap<Surface, u32>,
    placements: HashMap<Placement, u32>,
}

pub(super) struct KittyFrame {
    next: Display,
    pub commands: Vec<u8>,
}

#[derive(Debug)]
pub(super) struct KittyPresenter {
    pixels: CellPixels,
    displayed: Display,
    uncertain_ids: Option<HashSet<u32>>,
}

// Cloning a renderer must never transfer ownership of terminal resources.
impl Clone for KittyPresenter {
    fn clone(&self) -> Self {
        Self::new(self.pixels)
    }
}

impl KittyPresenter {
    pub fn new(pixels: CellPixels) -> Self {
        Self {
            pixels,
            displayed: Display::default(),
            uncertain_ids: None,
        }
    }

    pub fn prepare(
        &self,
        buffer: &Buffer,
        origin: Position,
        inline_extent: Option<u16>,
    ) -> io::Result<RasterPlan> {
        if buffer.has_unsupported_raster_composition() {
            return Err(unsupported(
                "Kitty does not support this raster composition",
            ));
        }
        let native = buffer.has_raster();
        let pixels = self.pixels;
        if native {
            CellPixels::resolve(Some(pixels))?;
            check_limits(buffer, origin, pixels)?;
        }
        let (sources, groups) = visible_surfaces(buffer, pixels);
        let empty = Display::default();
        // On recovery the writer cleans all uncertain IDs before this plan starts.
        let old = if self.uncertain() {
            &empty
        } else {
            &self.displayed
        };
        let reuse = buffer.previous().is_some();
        let (mut next, uploads) = plan_surfaces(sources, old, pixels, reuse)?;
        let additions = plan_placements(groups, origin, old, &mut next);

        let mut commands = Vec::new();
        let mut retained = HashSet::new();
        for (surface, &id) in &old.images {
            if next.images.get(surface) == Some(&id) {
                retained.insert(id);
            } else {
                delete_image(&mut commands, id)?;
            }
        }
        for (place, id) in &old.placements {
            if retained.contains(&place.image) && !next.placements.contains_key(place) {
                write!(commands, "\x1b_Ga=d,d=i,i={},p={id},q=2\x1b\\", place.image)?;
            }
        }
        for (surface, cell, id) in uploads {
            upload(&mut commands, surface.block, cell, pixels, id)?;
        }
        place(&mut commands, &mut next, additions, pixels, inline_extent)?;
        if commands.len() > MAX_FRAME_BYTES {
            return Err(unsupported("Kitty frame exceeds 32 MiB"));
        }
        Ok(RasterPlan {
            native,
            overlay_frame: native || !self.displayed.images.is_empty() || self.uncertain(),
            clear: false,
            damage_pixels: None,
            inline_padding: 0,
            payload: Payload::Kitty(KittyFrame { commands, next }),
        })
    }

    pub fn begin(&mut self, frame: &KittyFrame) {
        if !self.displayed.images.is_empty() || !frame.next.images.is_empty() {
            self.uncertain_ids = Some(
                self.displayed
                    .images
                    .values()
                    .chain(frame.next.images.values())
                    .copied()
                    .collect(),
            );
        }
    }
    pub fn commit(&mut self, frame: KittyFrame) {
        self.displayed = frame.next;
        self.uncertain_ids = None;
    }
    // Committed inline placements now belong to terminal scrollback.
    pub fn forget_region(&mut self) {
        self.displayed = Display::default();
    }

    pub fn uncertain(&self) -> bool {
        self.uncertain_ids.is_some()
    }

    pub fn cleanup(&mut self, out: &mut impl Write) -> io::Result<()> {
        if self.displayed.images.is_empty() && !self.uncertain() {
            return Ok(());
        }
        let interrupted = self.uncertain();
        let ids = self
            .uncertain_ids
            .get_or_insert_with(|| self.displayed.images.values().copied().collect());
        if interrupted {
            out.write_all(b"\x18\x1b\\")?;
            crossterm::queue!(out, crossterm::terminal::EndSynchronizedUpdate)?;
        }
        for &id in ids.iter() {
            delete_image(out, id)?;
        }
        out.flush()?;
        self.displayed = Display::default();
        self.uncertain_ids = None;
        Ok(())
    }
}

type Sources = HashMap<Surface, RasterCell>;
// Each group has one target origin and one stacking depth.
type Groups = HashMap<(Surface, i32, i32, usize), Vec<(u16, u16)>>;
type Upload = (Surface, RasterCell, u32);

fn visible_surfaces(buffer: &Buffer, pixels: CellPixels) -> (Sources, Groups) {
    let mut sources = HashMap::new();
    let mut groups = HashMap::<_, Vec<(u16, u16)>>::new();
    let mut add = |x: u16, y: u16, cell: &RasterCell, layer: usize| {
        let surface = Surface::new(cell, pixels);
        sources.entry(surface).or_insert_with(|| cell.clone());
        let dx = i32::from(x) - i32::from(cell.x);
        let dy = i32::from(y) - i32::from(cell.y);
        groups
            .entry((surface, dx, dy, layer))
            .or_default()
            .push((y, x));
    };
    for (x, y, cell) in buffer.raster_cells() {
        add(x, y, cell, 0);
    }
    let mut prev = None;
    let mut layer = 0;
    for (x, y, cell) in buffer.overlay_cells() {
        if prev != Some((x, y)) {
            layer = 0;
            prev = Some((x, y));
        }
        layer += 1;
        add(x, y, cell, layer);
    }

    (sources, groups)
}

fn plan_surfaces(
    sources: Sources,
    old: &Display,
    pixels: CellPixels,
    reuse: bool,
) -> io::Result<(Display, Vec<Upload>)> {
    let mut next = Display::default();
    let mut uploads = Vec::new();
    let mut resident_bytes = 0u64;
    for (surface, cell) in sources {
        let bytes = u64::from(surface.block.w)
            * u64::from(surface.block.h)
            * u64::from(pixels.width)
            * u64::from(pixels.height)
            * 4;
        resident_bytes += bytes;
        if resident_bytes > MAX_FRAME_BYTES as u64 {
            return Err(unsupported("Kitty resident RGBA surfaces exceed 32 MiB"));
        }
        let id = match old.images.get(&surface).filter(|_| reuse) {
            Some(&id) => id,
            None => {
                let id = image_id()?;
                uploads.push((surface, cell, id));
                id
            }
        };
        next.images.insert(surface, id);
    }

    Ok((next, uploads))
}

fn plan_placements(
    groups: Groups,
    origin: Position,
    old: &Display,
    next: &mut Display,
) -> Vec<Placement> {
    // Placement IDs are scoped to their image. Reserve retained IDs first.
    let mut additions = Vec::new();
    for ((surface, dx, dy, layer), cells) in groups {
        let image = next.images[&surface];
        for rect in rectangles(&cells) {
            let placement = Placement {
                image,
                layer,
                crop_x: (i32::from(rect.x) - dx) as u16 - surface.block.x,
                crop_y: (i32::from(rect.y) - dy) as u16 - surface.block.y,
                screen: Rect {
                    x: origin.x + rect.x,
                    y: origin.y + rect.y,
                    ..rect
                },
            };
            if let Some(&id) = old.placements.get(&placement) {
                next.placements.insert(placement, id);
            } else {
                additions.push(placement);
            }
        }
    }

    additions
}

fn place(
    commands: &mut Vec<u8>,
    next: &mut Display,
    additions: Vec<Placement>,
    pixels: CellPixels,
    inline_extent: Option<u16>,
) -> io::Result<()> {
    let used: HashSet<_> = next
        .placements
        .iter()
        .map(|(p, id)| (p.image, *id))
        .collect();
    let mut counters = HashMap::new();
    if additions.is_empty() {
        return Ok(());
    }
    if inline_extent.is_none() {
        commands.extend_from_slice(b"\x1b7");
    }
    for p in &additions {
        let id = counters.entry(p.image).or_insert(0u32);
        loop {
            *id += 1;
            if !used.contains(&(p.image, *id)) {
                break;
            }
        }
        let Rect { x, y, w, h } = p.screen;
        if let Some(extent) = inline_extent {
            // The inline renderer saved the bottom row after reserving space.
            commands.extend_from_slice(b"\x1b8");
            let up = extent - 1 - y;
            if up > 0 {
                write!(commands, "\x1b[{up}F")?;
            }
            write!(commands, "\x1b[{}G", x + 1)?;
        } else {
            write!(commands, "\x1b[{};{}H", y + 1, x + 1)?;
        }
        write!(
            commands,
            "\x1b_Ga=p,i={},p={},x={},y={},w={},h={},c={w},r={h},z={},C=1,q=2\x1b\\",
            p.image,
            *id,
            u32::from(p.crop_x) * u32::from(pixels.width),
            u32::from(p.crop_y) * u32::from(pixels.height),
            u32::from(w) * u32::from(pixels.width),
            u32::from(h) * u32::from(pixels.height),
            p.layer + 1
        )?;
        next.placements.insert(*p, *id);
    }
    commands.extend_from_slice(b"\x1b8");
    Ok(())
}

fn delete_image(out: &mut impl Write, id: u32) -> io::Result<()> {
    write!(out, "\x1b_Ga=d,d=I,i={id},q=2\x1b\\")
}

// Group row-major cells into runs, then extend identical runs vertically.
fn rectangles(cells: &[(u16, u16)]) -> Vec<Rect> {
    let mut rects = Vec::<Rect>::new();
    let mut last = HashMap::<(u16, u16), usize>::new();
    let mut cells = cells.iter().copied().peekable();
    while let Some((y, x)) = cells.next() {
        let mut w = 1;
        while cells.next_if_eq(&(y, x + w)).is_some() {
            w += 1;
        }
        let index = last.entry((x, w)).or_insert(rects.len());
        if let Some(rect) = rects.get_mut(*index) {
            if rect.y + rect.h == y {
                rect.h += 1;
                continue;
            }
        }
        *index = rects.len();
        rects.push(Rect { x, y, w, h: 1 });
    }
    rects
}

fn upload(
    out: &mut Vec<u8>,
    block: Rect,
    mut cell: RasterCell,
    pixels: CellPixels,
    id: u32,
) -> io::Result<()> {
    let width = u32::from(block.w) * u32::from(pixels.width);
    let height = u32::from(block.h) * u32::from(pixels.height);
    let len = width as usize * height as usize * 4;
    let mut rgba = Vec::with_capacity(len);
    for y in 0..height {
        cell.y = block.y + (y / u32::from(pixels.height)) as u16;
        for x in 0..width {
            cell.x = block.x + (x / u32::from(pixels.width)) as u16;
            let sample = cell.sample_visible(
                x % u32::from(pixels.width),
                y % u32::from(pixels.height),
                pixels.width,
                pixels.height,
            );
            rgba.extend_from_slice(&match sample {
                Some(Rgb(r, g, b)) => [r, g, b, 255],
                None => [0; 4],
            });
        }
    }
    transmit(out, id, width, height, &rgba, b"")
}

/// A single Kitty image owned by an application that manages its own terminal
/// I/O (for example, a PTY overlay). Call [`Self::delete`] before teardown.
#[derive(Debug)]
pub struct KittyImage(u32);

impl KittyImage {
    /// Allocate a process-local, nonzero protocol ID.
    pub fn new() -> io::Result<Self> {
        image_id().map(Self)
    }

    /// Replace the image at the cursor with unscaled RGBA pixels. The cursor
    /// stays in place. The caller owns positioning and flushing. The existing
    /// buffer, encoded upload and placement together must fit within 32 MiB.
    /// A rejected draw leaves the buffer intact.
    pub fn draw(
        &self,
        out: &mut Vec<u8>,
        width: u16,
        height: u16,
        rgba: &[[u8; 4]],
        z_index: i32,
    ) -> io::Result<()> {
        if width == 0 || height == 0 || rgba.len() != usize::from(width) * usize::from(height) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Kitty RGBA dimensions do not match",
            ));
        }
        let placement = format!("\x1b_Ga=p,i={},p=1,z={z_index},C=1,q=2\x1b\\", self.0);
        transmit(
            out,
            self.0,
            width.into(),
            height.into(),
            rgba.as_flattened(),
            placement.as_bytes(),
        )
    }

    /// Remove this image's placements and pixel data from the terminal.
    pub fn delete(&self, out: &mut impl Write) -> io::Result<()> {
        delete_image(out, self.0)
    }
}

const CHUNK_BYTES: usize = 3072;
const MORE: &[u8] = b"q=2,m=1;";
const LAST: &[u8] = b"q=2,m=0;";
const END: &[u8] = b"\x1b\\";
const CONTINUE: &[u8] = b"\x1b_G";

fn transmit(
    out: &mut Vec<u8>,
    id: u32,
    width: u32,
    height: u32,
    rgba: &[u8],
    placement: &[u8],
) -> io::Result<()> {
    if rgba.len() > MAX_FRAME_BYTES {
        return Err(unsupported("Kitty image exceeds 32 MiB"));
    }
    let header = format!("\x1b_Ga=t,f=32,t=d,i={id},s={width},v={height},");
    // CHUNK_BYTES is divisible by three, so only the final chunk can need padding.
    let chunks = rgba.len().div_ceil(CHUNK_BYTES);
    let encoded = rgba.len().div_ceil(3) * 4;
    let total = header.len()
        + chunks.saturating_sub(1) * (CONTINUE.len() + MORE.len() + END.len())
        + LAST.len()
        + END.len()
        + encoded
        + placement.len();
    if total > MAX_FRAME_BYTES.saturating_sub(out.len()) {
        return Err(unsupported("encoded Kitty frame exceeds 32 MiB"));
    }
    out.try_reserve(total)
        .map_err(|e| io::Error::new(io::ErrorKind::OutOfMemory, e))?;
    out.extend_from_slice(header.as_bytes());
    let mut chunks = rgba.chunks(CHUNK_BYTES).peekable();
    let mut encoded = [0; 4096];
    while let Some(chunk) = chunks.next() {
        let more = chunks.peek().is_some();
        out.extend_from_slice(if more { MORE } else { LAST });
        let n = STANDARD
            .encode_slice(chunk, &mut encoded)
            .expect("4096 bytes fit a 3072 byte chunk");
        out.extend_from_slice(&encoded[..n]);
        out.extend_from_slice(END);
        if more {
            out.extend_from_slice(CONTINUE);
        }
    }
    out.extend_from_slice(placement);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::raster::{RasterAlign, RasterCanvas, RasterScale};

    #[test]
    fn direct_draw_preflights_encoded_chunks_and_placement_before_appending() {
        let image = KittyImage::new().unwrap();
        let mut out = b"previous commands".to_vec();
        let large = vec![[0; 4]; 4096 * 2048];
        let error = image.draw(&mut out, 4096, 2048, &large, 0).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Unsupported);
        assert_eq!(out, b"previous commands");
        drop(large);

        // Cross a chunk boundary and include the longest signed z-index.
        let pixels = [[0; 4]; 769];
        let mut commands = Vec::new();
        image
            .draw(&mut commands, 769, 1, &pixels, i32::MIN)
            .unwrap();
        let prefix_len = MAX_FRAME_BYTES - commands.len();
        out.resize(prefix_len + 1, 7);
        assert!(image.draw(&mut out, 769, 1, &pixels, i32::MIN).is_err());
        assert_eq!(out.len(), prefix_len + 1);
        assert!(out.starts_with(b"previous commands"));
        assert!(out[b"previous commands".len()..].iter().all(|&c| c == 7));
        out.pop();
        image.draw(&mut out, 769, 1, &pixels, i32::MIN).unwrap();
        assert_eq!(out.len(), MAX_FRAME_BYTES);
        assert_eq!(&out[prefix_len..], commands);
    }

    #[test]
    fn clipped_surfaces_sample_the_original_target_including_alpha_and_fit_padding() {
        let mut canvas = RasterCanvas::new_transparent(3, 2).unwrap();
        canvas.set_pixel(1, 0, Rgb(13, 57, 91));
        canvas.set_pixel(2, 1, Rgb(200, 115, 75));
        let pixels = CellPixels {
            width: 10,
            height: 20,
        };
        for scale in [
            RasterScale::Fit,
            RasterScale::Original,
            RasterScale::Stretch,
        ] {
            let mut cell = RasterCell {
                canvas: canvas.clone(),
                x: 2,
                y: 1,
                target: (5, 3).into(),
                options: RasterOptions {
                    scale,
                    align_x: RasterAlign::End,
                    align_y: RasterAlign::Center,
                    background: Rgb(8, 9, 10),
                },
            };
            let mut buffer = Buffer::new((1, 1).into());
            buffer.overlay_raster((0, 0).into(), cell.clone()).unwrap();
            let plan = KittyPresenter::new(pixels)
                .prepare(&buffer, (0, 0).into(), None)
                .unwrap();
            let Payload::Kitty(frame) = plan.payload else {
                panic!()
            };
            let mut rgba = Vec::new();
            for cmd in std::str::from_utf8(&frame.commands)
                .unwrap()
                .split("\x1b_G")
                .skip(1)
            {
                let command = cmd.split_once("\x1b\\").unwrap().0;
                if let Some((_, data)) = command.split_once(';') {
                    rgba.extend(STANDARD.decode(data).unwrap());
                }
            }
            let place = frame.next.placements.keys().next().unwrap();
            assert_eq!((place.crop_x, place.crop_y), (2, 1));
            assert_eq!((place.screen.w, place.screen.h), (1, 1));
            assert_eq!(rgba.len(), 50 * 60 * 4);
            for y in 0..60 {
                for x in 0..50 {
                    cell.x = (x / 10) as u16;
                    cell.y = (y / 20) as u16;
                    let expected = cell
                        .sample_visible(x % 10, y % 20, 10, 20)
                        .map_or([0; 4], |Rgb(r, g, b)| [r, g, b, 255]);
                    let offset = (y * 50 + x) as usize * 4;
                    assert_eq!(rgba[offset..offset + 4], expected);
                }
            }
            cell.target = (u16::MAX, u16::MAX).into();
            cell.x = u16::MAX - 1;
            cell.y = u16::MAX - 1;
            let block = Surface::new(&cell, pixels).block;
            assert!(u32::from(block.w) * 10 <= 512 && u32::from(block.h) * 20 <= 512);
            assert!(block.x > 0 && block.y > 0);
        }
    }
}
