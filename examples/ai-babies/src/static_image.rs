//! Aspect-corrected reference at selectable heights through the 2×2 encoder.
use std::{
    error::Error,
    fs,
    io::{self, Write},
};

use crossterm::style::{Color, ResetColor};
use rsille::{
    canvas::{Canvas, Encoding},
    render::{buffer::Buffer, chunk::Chunk, Draw},
};

const SOURCE_WIDTH: usize = 240;
const SOURCE_HEIGHT: usize = 80;
const RGB: &[u8; SOURCE_WIDTH * SOURCE_HEIGHT * 3] =
    include_bytes!("../assets/chibi-trio-240x80.rgb");
// Canvas mask order: bottom left, bottom right, top left, top right.
const SYMBOLS: &str = " ▖▗▄▘▌▞▙▝▚▐▟▀▛▜█";

fn frame(rows: u16) -> Result<Buffer, Box<dyn Error>> {
    // 3:1 image aspect with 1:2 terminal cells requires six columns per row.
    let width = usize::from(rows) * 12;
    let height = usize::from(rows) * 2;
    let size = (rows * 6, rows).into();
    let mut canvas = Canvas::with_encoding(Encoding::Quadrant);
    canvas.set_bound((0, i32::from(rows) * 6 - 1), (0, i32::from(rows) - 1));
    for y in 0..height {
        for x in 0..width {
            // Sample pixel centers directly from the original, avoiding repeated resizing.
            let source_x = (2 * x + 1) * SOURCE_WIDTH / (2 * width);
            let source_y = (2 * y + 1) * SOURCE_HEIGHT / (2 * height);
            let index = (source_y * SOURCE_WIDTH + source_x) * 3;
            canvas.set_color(
                x as f64,
                (height - 1 - y) as f64,
                Color::Rgb {
                    r: RGB[index],
                    g: RGB[index + 1],
                    b: RGB[index + 2],
                },
            );
        }
    }
    let mut buffer = Buffer::new(size);
    canvas.draw(Chunk::new(&mut buffer, size.into())?)?;
    Ok(buffer)
}

/// Reconstruct encoded quadrants, using a typical 1:2 terminal cell aspect.
fn snapshot(buffer: &Buffer) -> Vec<u8> {
    let width = usize::from(buffer.size().width) * 2;
    let height = usize::from(buffer.size().height) * 4;
    let mut bytes = format!("P6\n{width} {height}\n255\n").into_bytes();
    for y in 0..height {
        for x in 0..width {
            let sample_y = y / 2;
            let cell = buffer
                .get((x as u16 / 2, sample_y as u16 / 2).into())
                .unwrap();
            let mask = SYMBOLS
                .chars()
                .position(|c| Some(c) == cell.as_char())
                .unwrap();
            let bit = (1 - sample_y % 2) * 2 + x % 2;
            let colors = cell.style.colors.unwrap();
            let color = if mask & (1 << bit) != 0 {
                colors.foreground
            } else {
                colors.background
            };
            let Some(Color::Rgb { r, g, b }) = color else {
                unreachable!("RGB canvas")
            };
            bytes.extend_from_slice(&[r, g, b]);
        }
    }
    bytes
}

pub fn command(args: &[String]) -> Result<(), Box<dyn Error>> {
    let (rows, args) = if args.first().is_some_and(|arg| arg == "--height") {
        let rows: u16 = args.get(1).ok_or("Missing height")?.parse()?;
        if !(1..=40).contains(&rows) {
            return Err("Height must be between 1 and 40 rows".into());
        }
        (rows, &args[2..])
    } else {
        (24, args)
    };
    match args {
        [] => {
            let buffer = frame(rows)?;
            let mut output = io::BufWriter::new(io::stdout().lock());
            for row in buffer
                .content()
                .chunks_exact(usize::from(buffer.size().width))
            {
                for cell in row {
                    cell.queue(&mut output)?;
                }
                crossterm::queue!(output, ResetColor)?;
                writeln!(output)?;
            }
            output.flush()?;
        }
        [flag, path] if flag == "--snapshot" => {
            fs::write(path, snapshot(&frame(rows)?))?;
            println!(
                "Saved {}×{} Quadrant cells to {path} (1:2 cell aspect)",
                rows * 6,
                rows
            );
        }
        _ => return Err("Usage: --static [--height ROWS] [--snapshot FILE.ppm]".into()),
    }
    Ok(())
}
