//! Export the actual terminal cells, including Canvas's half-block encoding.
use std::{error::Error, fs, time::Instant};

use crossterm::style::Color;
use rsille::render::{area::Size, buffer::Buffer, chunk::Chunk, Draw};

use crate::{Aquarium, INLINE_HEIGHT, PREVIEW_WIDTH};

pub fn frame(aquarium: &mut Aquarium, size: Size) -> Result<Buffer, Box<dyn Error>> {
    let mut buffer = Buffer::new(size);
    aquarium.draw(Chunk::new(&mut buffer, size.into())?)?;
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
        "--snapshot" if (2..=3).contains(&args.len()) => {
            let time: f64 = args.get(2).map_or(Ok(8.0), |s| s.parse())?;
            if !time.is_finite() || !(0.0..=86400.0).contains(&time) {
                return Err("SECONDS must be finite and between 0 and 86400".into());
            }
            let mut aquarium = Aquarium { time, ..Aquarium::default() };
            fs::write(&args[1], ppm(&frame(&mut aquarium, (PREVIEW_WIDTH, INLINE_HEIGHT).into())?))?;
            println!("Saved terminal frame to {} at {time}s", args[1]);
        }
        "--frames" if args.len() == 2 => {
            fs::create_dir_all(&args[1])?;
            let mut aquarium = Aquarium::default();
            for index in 0..120 {
                aquarium.time = index as f64 / 10.0;
                let path = std::path::Path::new(&args[1]).join(format!("{index:04}.ppm"));
                fs::write(path, ppm(&frame(&mut aquarium, (PREVIEW_WIDTH, INLINE_HEIGHT).into())?))?;
            }
            println!("Saved 12 seconds at 10 FPS to {}", args[1]);
        }
        "--headless" if args.len() == 1 => {
            let mut aquarium = Aquarium::default();
            let start = Instant::now();
            for index in 0..90 {
                aquarium.time = index as f64 * 0.75;
                let buffer = frame(&mut aquarium, (PREVIEW_WIDTH, INLINE_HEIGHT).into())?;
                assert!(buffer.content().iter().all(|cell| !cell.content.is_empty()));
                assert!(aquarium.sea.pixels.iter().flatten().all(|v| v.is_finite()));
            }
            println!("90 frames passed at {PREVIEW_WIDTH}x{INLINE_HEIGHT}; mean {:.2} ms/frame (drawing + encoding; no terminal I/O)", start.elapsed().as_secs_f64() * 1000.0 / 90.0);
        }
        "--help" | "-h" if args.len() == 1 => println!(
            "JELLYFISH / a quiet deep-sea aquarium\n\nRun without arguments for automatic inline playback (up to 28 rows).\nq / Esc / Ctrl+C: exit\n\n--headless\n--snapshot FILE.ppm [SECONDS]\n--frames DIRECTORY  (120 PPM frames at 10 FPS)"
        ),
        _ => return Err("Unknown or incomplete arguments; use --help".into()),
    }
    Ok(())
}
