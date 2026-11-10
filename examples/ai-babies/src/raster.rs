//! Cached opaque snapshots for raster playback, decoded from embedded PNGs.
//!
//! The 320px frames are upscaled directly from native 256px source cells using
//! the packers' stable motion/palette rules, not newly authored 320px detail.
//! Python is only needed to rebuild assets, never for runtime playback.
use crate::scene;
use rsille::raster::{RasterCanvas, Rgb};
use std::{error::Error, io::Cursor};

pub const SIDE: u32 = 320;
const COLUMNS: u32 = 4;
const ROWS: u32 = 6;
const COUNT: usize = (COLUMNS * ROWS) as usize;

pub struct Frames {
    frames: Vec<RasterCanvas>,
}

impl Frames {
    /// Decode once and composite all 24 source snapshots onto `background`.
    pub fn new(character: scene::Character, background: Rgb) -> Result<Self, Box<dyn Error>> {
        let atlas: &[u8] = match character {
            scene::Character::ChatGpt => {
                include_bytes!("../assets/chatgpt-actions-v2-320x320.png")
            }
            scene::Character::DeepSeek => {
                include_bytes!("../assets/deepseek-actions-320x320.png")
            }
            scene::Character::Claude => include_bytes!("../assets/claude-actions-320x320.png"),
        };
        Self::from_png(atlas, background)
    }

    fn from_png(atlas: &[u8], background: Rgb) -> Result<Self, Box<dyn Error>> {
        let mut decoder = png::Decoder::new(Cursor::new(atlas));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info()?;
        if reader.info().width != COLUMNS * SIDE || reader.info().height != ROWS * SIDE {
            return Err("Raster atlas must be a 4x6 grid of 320x320 frames".into());
        }
        let mut pixels = vec![
            0;
            reader
                .output_buffer_size()
                .ok_or("Raster atlas is too large")?
        ];
        let info = reader.next_frame(&mut pixels)?;
        if info.width != COLUMNS * SIDE || info.height != ROWS * SIDE {
            return Err("Unexpected decoded raster atlas dimensions".into());
        }
        // Inspect the decoded format, not the header: palette/tRNS expansion can
        // produce RGBA even when the PNG was stored as indexed or RGB data.
        let channels = match (info.color_type, info.bit_depth) {
            (png::ColorType::Rgb, png::BitDepth::Eight) => 3,
            (png::ColorType::Rgba, png::BitDepth::Eight) => 4,
            _ => return Err("Raster atlas must decode to eight-bit RGB or RGBA".into()),
        };
        let stride = info.width as usize * channels;
        if info.line_size != stride || info.buffer_size() != stride * info.height as usize {
            return Err("Unexpected raster atlas pixel layout".into());
        }
        let mut frames = Vec::with_capacity(COUNT);
        for index in 0..COUNT {
            let left = index % COLUMNS as usize * SIDE as usize;
            let top = index / COLUMNS as usize * SIDE as usize;
            let mut canvas = RasterCanvas::new(SIDE, SIDE, background)?;
            for y in 0..SIDE as usize {
                for x in 0..SIDE as usize {
                    let offset = (top + y) * stride + (left + x) * channels;
                    let pixel = &pixels[offset..offset + channels];
                    let alpha = if channels == 4 { pixel[3] } else { 255 };
                    canvas.set_pixel(x as i32, y as i32, composite(pixel, alpha, background));
                }
            }
            frames.push(canvas);
        }
        Ok(Self { frames })
    }

    /// Borrow an immutable snapshot; the caller selects the index with
    /// `scene::current_frame`. Panics if `index` is outside `0..24`.
    pub fn frame(&self, index: usize) -> &RasterCanvas {
        &self.frames[index]
    }
}

fn composite(pixel: &[u8], alpha: u8, background: Rgb) -> Rgb {
    let alpha = u32::from(alpha);
    let blend = |source: u8, background: u8| {
        ((u32::from(source) * alpha + u32::from(background) * (255 - alpha) + 127) / 255) as u8
    };
    Rgb(
        blend(pixel[0], background.0),
        blend(pixel[1], background.1),
        blend(pixel[2], background.2),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn all_characters_have_24_square_opaque_frames_with_background() {
        for character in scene::Character::ALL {
            for background in [Rgb(0, 0, 0), Rgb(17, 43, 89)] {
                let frames = Frames::new(character, background).unwrap();
                assert_eq!(frames.frames.len(), COUNT);
                for index in 0..COUNT {
                    let frame = frames.frame(index);
                    assert_eq!((frame.width(), frame.height()), (SIDE, SIDE));
                    assert_eq!(frame.background(), background);
                    let pixels = frame.pixels();
                    let side = SIDE as usize;
                    assert_eq!(pixels.len(), side * side);
                    assert!(pixels.iter().any(|&pixel| pixel != background));
                    for i in 0..side {
                        assert_eq!(pixels[i], background);
                        assert_eq!(pixels[(side - 1) * side + i], background);
                        assert_eq!(pixels[i * side], background);
                        assert_eq!(pixels[i * side + side - 1], background);
                    }
                }
            }
        }
    }

    #[test]
    fn authored_poses_are_distinct_and_snapshot_ids_remain_stable() {
        for character in scene::Character::ALL {
            let frames = Frames::new(character, Rgb(17, 43, 89)).unwrap();
            let identities: Vec<_> = (0..COUNT)
                .map(|i| (frames.frame(i).id(), frames.frame(i).version()))
                .collect();
            assert_eq!(
                identities.iter().map(|p| p.0).collect::<HashSet<_>>().len(),
                COUNT
            );
            for row in 0..ROWS as usize {
                let poses: HashSet<_> = (0..COLUMNS as usize)
                    .map(|col| frames.frame(row * COLUMNS as usize + col).pixels())
                    .collect();
                assert_eq!(poses.len(), if row == 0 { 2 } else { 4 });
            }
            for _ in 0..3 {
                for index in (0..COUNT).rev() {
                    let frame = frames.frame(index);
                    assert_eq!((frame.id(), frame.version()), identities[index]);
                    assert!(std::ptr::eq(frame, frames.frame(index)));
                }
            }
        }
    }

    fn encode(width: u32, height: u32, color: png::ColorType, pixels: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, width, height);
            encoder.set_color(color);
            encoder.set_depth(png::BitDepth::Eight);
            if color == png::ColorType::Indexed {
                encoder.set_palette(vec![200, 100, 50, 20, 220, 240]);
                encoder.set_trns(vec![0, 128]);
            }
            encoder
                .write_header()
                .unwrap()
                .write_image_data(pixels)
                .unwrap();
        }
        bytes
    }

    #[test]
    fn rgba_alpha_is_composited_and_atlas_cells_are_row_major() {
        let width = COLUMNS * SIDE;
        let mut pixels = vec![0; (width * ROWS * SIDE * 4) as usize];
        for index in 0..COUNT {
            let x = index as u32 % COLUMNS * SIDE + 1;
            let y = index as u32 / COLUMNS * SIDE + 2;
            let offset = ((y * width + x) * 4) as usize;
            pixels[offset..offset + 4].copy_from_slice(&[200, 100, 50, index as u8 * 11]);
        }
        // Alpha zero must discard hidden RGB, not just assume it is black.
        pixels[..4].copy_from_slice(&[255, 100, 200, 0]);
        pixels[4..8].copy_from_slice(&[200, 100, 50, 255]);
        pixels[8..12].copy_from_slice(&[200, 100, 50, 128]);
        let bytes = encode(width, ROWS * SIDE, png::ColorType::Rgba, &pixels);
        let background = Rgb(10, 20, 30);
        let frames = Frames::from_png(&bytes, background).unwrap();
        assert_eq!(frames.frame(0).pixels()[0], background);
        assert_eq!(frames.frame(0).pixels()[1], Rgb(200, 100, 50));
        assert_eq!(frames.frame(0).pixels()[2], Rgb(105, 60, 40));
        for index in 0..COUNT {
            let alpha = index as u32 * 11;
            let expected = Rgb(
                ((200 * alpha + 10 * (255 - alpha) + 127) / 255) as u8,
                ((100 * alpha + 20 * (255 - alpha) + 127) / 255) as u8,
                ((50 * alpha + 30 * (255 - alpha) + 127) / 255) as u8,
            );
            assert_eq!(
                frames.frame(index).pixels()[2 * SIDE as usize + 1],
                expected
            );
        }
    }

    #[test]
    fn indexed_transparency_expands_to_rgba_and_rgb_is_opaque() {
        let width = COLUMNS * SIDE;
        let height = ROWS * SIDE;
        let mut pixels = vec![0; (width * height) as usize];
        pixels[1] = 1;
        let bytes = encode(width, height, png::ColorType::Indexed, &pixels);
        let background = Rgb(10, 20, 30);
        let frames = Frames::from_png(&bytes, background).unwrap();
        assert_eq!(frames.frame(0).pixels()[0], background);
        assert_eq!(frames.frame(0).pixels()[1], Rgb(15, 120, 135));

        let bytes = encode(width, height, png::ColorType::Rgb, &pixels.repeat(3));
        let frames = Frames::from_png(&bytes, background).unwrap();
        assert_eq!(frames.frame(0).pixels()[0], Rgb(0, 1, 0));
        assert_eq!(frames.frame(0).pixels()[1], Rgb(0, 0, 0));
    }

    #[test]
    fn invalid_dimensions_format_and_png_are_rejected() {
        let background = Rgb(0, 0, 0);
        let bytes = encode(1, 1, png::ColorType::Rgba, &[0, 0, 0, 0]);
        assert!(Frames::from_png(&bytes, background).is_err());
        let bytes = encode(
            COLUMNS * SIDE,
            ROWS * SIDE,
            png::ColorType::Grayscale,
            &vec![0; (COLUMNS * ROWS * SIDE * SIDE) as usize],
        );
        assert!(Frames::from_png(&bytes, background).is_err());
        assert!(Frames::from_png(b"not a PNG", background).is_err());
        let bytes = include_bytes!("../assets/chatgpt-actions-v2-320x320.png");
        assert!(Frames::from_png(&bytes[..bytes.len() / 2], background).is_err());
    }
}
