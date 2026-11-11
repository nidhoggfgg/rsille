use super::*;
use rsille::render::{
    area::Size,
    chunk::Chunk,
    raster::{RasterCell, RasterOptions},
    raster_output::{CellPixels, RasterCapabilities, RasterOutput},
    Builder, Draw, DrawErr,
};

struct Scene(RasterCanvas);
impl Draw for Scene {
    fn draw(&mut self, mut chunk: Chunk) -> Result<Size, DrawErr> {
        for y in 0..14 {
            for x in 0..70 {
                chunk.set_raster_cell(
                    x,
                    y,
                    RasterCell {
                        canvas: self.0.clone(),
                        target: (70, 14).into(),
                        x,
                        y,
                        options: RasterOptions::default(),
                    },
                )?;
            }
        }
        Ok((70, 14).into())
    }
}

fn number(bytes: &[u8], i: &mut usize) -> usize {
    let mut n = 0;
    while *i < bytes.len() && bytes[*i].is_ascii_digit() {
        n = n * 10 + usize::from(bytes[*i] - b'0');
        *i += 1;
    }
    n
}

// Decode the actual emitted color planes (including repeats and partial bands),
// not a second implementation of the sampler. Retain palette indices so the
// expected RGB332 quantization can be checked exactly for every output pixel.
fn decode(bytes: &[u8], width: usize, height: usize) -> Vec<u8> {
    assert_eq!(bytes.first(), Some(&b'"'));
    let mut i = 1;
    let mut attributes = [0; 4];
    for (index, value) in attributes.iter_mut().enumerate() {
        if index > 0 {
            assert_eq!(bytes[i], b';');
            i += 1;
        }
        *value = number(bytes, &mut i);
    }
    assert_eq!(&attributes[..3], &[1, 1, width]);
    let encoded_height = attributes[3];
    let top_padding = encoded_height.checked_sub(height).expect("short raster");
    // Bottom-edge fullscreen tiles may be anchored up to five cells higher.
    assert_eq!(top_padding % height, 0);
    assert!(top_padding <= 5 * height);
    let mut pixels = vec![None; width * encoded_height];
    let (mut x, mut y, mut color) = (0, 0, 0);
    while i < bytes.len() {
        let command = bytes[i];
        i += 1;
        match command {
            b'"' | b'#' => {
                let n = number(bytes, &mut i);
                if command == b'#' {
                    color = n as u8;
                }
                while i < bytes.len() && bytes[i] == b';' {
                    i += 1;
                    number(bytes, &mut i);
                }
            }
            b'$' => x = 0,
            b'-' => {
                x = 0;
                y += 6;
            }
            b'!' | b'?'..=b'~' => {
                let (count, bits) = if command == b'!' {
                    let count = number(bytes, &mut i);
                    let bits = bytes[i] - b'?';
                    i += 1;
                    (count, bits)
                } else {
                    (1, command - b'?')
                };
                for _ in 0..count {
                    assert!(x < width);
                    for bit in 0..6 {
                        if bits & (1 << bit) != 0 {
                            assert!(y + bit < encoded_height, "painted band padding");
                            let pixel = &mut pixels[(y + bit) * width + x];
                            assert!(pixel.is_none(), "overlapping color planes");
                            *pixel = Some(color);
                        }
                    }
                    x += 1;
                }
            }
            _ => panic!("unexpected Sixel command {command}"),
        }
    }
    let prefix = top_padding * width;
    assert!(
        pixels[..prefix].iter().all(Option::is_none),
        "painted top padding"
    );
    pixels
        .into_iter()
        .skip(prefix)
        .map(|p| p.expect("missing output pixel"))
        .collect()
}

fn palette(Rgb(r, g, b): Rgb) -> u8 {
    (r & 0xe0) | ((g >> 3) & 0x1c) | (b >> 6)
}

// Reproduce the pre-AA y=x path: two rounded library lines per column.
// The identity function needs no adaptive subdivision beyond those midpoints.
fn legacy_diagonal(size: (u32, u32)) -> RasterCanvas {
    let mut canvas = Plot {
        functions: vec![],
        cache: RefCell::new(None),
    }
    .render(crate::BOUNDS, size);
    let view = View::new(crate::BOUNDS, size);
    for half in (view.left as i32 * 2)..(view.right as i32 * 2) {
        let x = f64::from(half) / 2.0;
        let mut a = (x, view.y(view.x(x)));
        let mut b = (x + 0.5, view.y(view.x(x + 0.5)));
        if (a.1 < view.top && b.1 < view.top) || (a.1 > view.bottom && b.1 > view.bottom) {
            continue;
        }
        if a.1 < view.top || a.1 > view.bottom {
            let y = a.1.clamp(view.top, view.bottom);
            a.0 += (b.0 - a.0) * (y - a.1) / (b.1 - a.1);
            a.1 = y;
        }
        if b.1 < view.top || b.1 > view.bottom {
            let y = b.1.clamp(view.top, view.bottom);
            b.0 = a.0 + (b.0 - a.0) * (y - a.1) / (b.1 - a.1);
            b.1 = y;
        }
        canvas.line(
            a.0.round() as i32,
            a.1.round() as i32,
            b.0.round() as i32,
            b.1.round() as i32,
            COLORS[0],
        );
    }
    canvas
}

#[test]
fn native_sixel_output_preserves_connected_diagonal_at_large_cell_metrics() {
    for (cw, ch) in [(18_u16, 40_u16), (20, 40)] {
        let size = (70 * u32::from(cw), 14 * u32::from(ch));
        for canvas in [
            legacy_diagonal(size),
            Plot::parse("x").unwrap().render(crate::BOUNDS, size),
        ] {
            let source = canvas
                .pixels()
                .chunks_exact(size.0 as usize)
                .map(|row| row.iter().copied().map(ink).collect())
                .collect();
            assert_eq!(components(source), 1);
            let mut bytes = Vec::new();
            {
                let mut renderer = Builder::new()
                    .size((70, 14))
                    .clear(false)
                    .enable_alt_screen()
                    .raster_output(RasterOutput {
                        capabilities: RasterCapabilities {
                            sixel: true,
                            reliable_erase: true,
                            reliable_inline_erase: false,
                            no_scroll: true,
                            cell_pixels: Some(CellPixels {
                                width: cw,
                                height: ch,
                            }),
                        },
                    })
                    .build_render(Scene(canvas.clone()), &mut bytes);
                renderer.render().unwrap();
            }
            let mut output = vec![vec![false; size.0 as usize]; size.1 as usize];
            let mut tiles = 0;
            let mut start = 0;
            while let Some(relative) = bytes[start..].windows(2).position(|w| w == b"\x1bP") {
                let dcs = start + relative + 2;
                let q = dcs + bytes[dcs..].iter().position(|&b| b == b'q').unwrap() + 1;
                let end = q + bytes[q..].windows(2).position(|w| w == b"\x1b\\").unwrap();
                let decoded = decode(&bytes[q..end], cw as usize, ch as usize);
                let (tx, ty) = (tiles % 70, tiles / 70);
                for (i, &actual) in decoded.iter().enumerate() {
                    let x = tx * cw as usize + i % cw as usize;
                    let y = ty * ch as usize + i / cw as usize;
                    assert_eq!(actual, palette(canvas.pixels()[y * size.0 as usize + x]));
                    // AA need not reach the exact full-color palette bin at every
                    // step. Require substantial cyan, excluding all grid/axis/text
                    // bins (their green-red difference is at most one level).
                    let red = actual >> 5;
                    let green = (actual >> 2) & 7;
                    output[y][x] = green >= red + 2 && actual & 3 >= 2;
                }
                tiles += 1;
                start = end + 2;
            }
            assert_eq!(tiles, 70 * 14);
            assert_eq!(components(output), 1, "{cw}x{ch}");
        }
    }
}
