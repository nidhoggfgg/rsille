//! Export the actual terminal cells, including Canvas's half-block encoding.
use std::{error::Error, fs, time::Instant};

use crossterm::style::Color;
use rsille::render::{area::Size, buffer::Buffer, chunk::Chunk, Draw};

use crate::{Nursery, INLINE_HEIGHT, PREVIEW_WIDTH};

pub fn frame(nursery: &mut Nursery, size: Size) -> Result<Buffer, Box<dyn Error>> {
    let mut buffer = Buffer::new(size);
    nursery.draw(Chunk::new(&mut buffer, size.into())?)?;
    Ok(buffer)
}

fn rgb(color: Option<Color>) -> [u8; 3] {
    match color {
        Some(Color::Rgb { r, g, b }) => [r, g, b],
        _ => [0, 0, 0],
    }
}

fn ppm(buffer: &Buffer) -> Vec<u8> {
    let size = buffer.size();
    let mut bytes = format!("P6\n{} {}\n255\n", size.width, size.height * 2).into_bytes();
    for y in 0..size.height * 2 {
        for x in 0..size.width {
            let cell = buffer.get((x, y / 2).into()).unwrap();
            let colors = cell.style.colors;
            let foreground = matches!(cell.as_char(), Some('█'))
                || (cell.as_char() == Some('▀') && y % 2 == 0)
                || (cell.as_char() == Some('▄') && y % 2 == 1);
            bytes.extend_from_slice(&rgb(colors.and_then(|c| {
                if foreground {
                    c.foreground
                } else {
                    c.background
                }
            })));
        }
    }
    bytes
}

pub fn command(args: &[String]) -> Result<(), Box<dyn Error>> {
    match args[0].as_str() {
        "--snapshot" if [2, 3, 5].contains(&args.len()) => {
            let time: f64 = args.get(2).map_or(Ok(0.0), |s| s.parse())?;
            if !time.is_finite() || !(0.0..=86400.0).contains(&time) {
                return Err("SECONDS must be finite and between 0 and 86400".into());
            }
            let mut nursery = Nursery { time, ..Nursery::default() };
            let size = if args.len() == 5 {
                let width: u16 = args[3].parse()?;
                let height: u16 = args[4].parse()?;
                if width == 0 || height == 0 || width > 512 || height > 128 {
                    return Err("size must be within 1..512 columns and 1..128 rows".into());
                }
                (width, height).into()
            } else { (PREVIEW_WIDTH, INLINE_HEIGHT).into() };
            fs::write(&args[1], ppm(&frame(&mut nursery, size)?))?;
            println!("Saved terminal frame to {} at {time}s", args[1]);
        }
        "--frames" if args.len() == 2 => {
            fs::create_dir_all(&args[1])?;
            let mut nursery = Nursery::default();
            for index in 0..192 {
                nursery.time = index as f64 / 24.0;
                let path = std::path::Path::new(&args[1]).join(format!("{index:04}.ppm"));
                fs::write(path, ppm(&frame(&mut nursery, (PREVIEW_WIDTH, INLINE_HEIGHT).into())?))?;
            }
            println!("Saved 8 seconds at 24 FPS to {}", args[1]);
        }
        "--headless" if args.len() == 1 => {
            let mut nursery = Nursery::default();
            let start = Instant::now();
            for index in 0..90 {
                nursery.time = index as f64 * 0.75;
                let buffer = frame(&mut nursery, (PREVIEW_WIDTH, INLINE_HEIGHT).into())?;
                assert!(buffer.content().iter().all(|cell| !cell.content.is_empty()));
            }
            println!("90 frames passed at {PREVIEW_WIDTH}x{INLINE_HEIGHT}; mean {:.2} ms/frame (drawing + encoding; no terminal I/O)", start.elapsed().as_secs_f64() * 1000.0 / 90.0);
        }
        "--help" | "-h" if args.len() == 1 => println!(
            "AI BABIES / quiet raster companions\n\nRun without arguments for automatic inline playback (up to 32 rows).\nq / Esc / Ctrl+C: exit; Space: pause; 1/2/3: focus; 0: all\n\n--static [--height ROWS] [--snapshot FILE.ppm]  (default: 144 columns × 24 rows; height: 1..40)\n--headless\n--snapshot FILE.ppm [SECONDS [COLUMNS ROWS]]\n--frames DIRECTORY  (192 PPM frames at 24 FPS)"
        ),
        _ => return Err("Unknown or incomplete arguments; use --help".into()),
    }
    Ok(())
}
