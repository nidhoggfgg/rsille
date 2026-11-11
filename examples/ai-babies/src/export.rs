//! Export character fallbacks (Quadrant or raster HalfBlock), not native Sixel pixels.
use std::{error::Error, fs, time::Instant};

use crossterm::style::Color;
use rsille::render::{area::Size, buffer::Buffer, chunk::Chunk, Draw};

use crate::{
    options::{Options, Output},
    Nursery, PREVIEW_WIDTH,
};

pub fn frame(nursery: &mut Nursery, size: Size) -> Result<Buffer, Box<dyn Error>> {
    let mut buffer = Buffer::new(size);
    nursery.draw(Chunk::new(&mut buffer, size.into())?)?;
    Ok(buffer)
}

pub fn rgba(buffer: &Buffer) -> Vec<[u8; 4]> {
    let size = buffer.size();
    // Two output pixels across a cell and four down it, matching the static
    // Quadrant export. Every Quadrant sample occupies a 1x2 output rectangle.
    let mut pixels = Vec::with_capacity(usize::from(size.width) * usize::from(size.height) * 8);
    for y in 0..size.height * 4 {
        for x in 0..size.width * 2 {
            let cell = buffer.get((x / 2, y / 4).into()).unwrap();
            // Mask bits run bottom-left, bottom-right, top-left, top-right.
            let mask = " ▖▗▄▘▌▚▙▝▞▐▟▀▛▜█"
                .chars()
                .position(|symbol| Some(symbol) == cell.as_char())
                .expect("Quadrant or HalfBlock canvas symbol");
            let bit = (1 - (y / 2) % 2) * 2 + x % 2;
            let foreground = mask & (1 << bit) != 0;
            let color = cell.style.colors.and_then(|colors| {
                if foreground {
                    colors.foreground
                } else {
                    colors.background
                }
            });
            pixels.push(match color {
                Some(Color::Rgb { r, g, b }) => [r, g, b, 255],
                _ => [0, 0, 0, 0],
            });
        }
    }
    pixels
}

pub fn snapshot(buffer: &Buffer) -> Vec<u8> {
    let size = buffer.size();
    let mut bytes = format!("P6\n{} {}\n255\n", size.width * 2, size.height * 4).into_bytes();
    // PPM has no alpha: black is only an export fallback, never a terminal fill.
    for pixel in rgba(buffer) {
        bytes.extend_from_slice(&pixel[..3]);
    }
    bytes
}

fn save(path: impl AsRef<std::path::Path>, buffer: &Buffer) -> Result<(), Box<dyn Error>> {
    let path = path.as_ref();
    let mask_path = path.with_extension("pgm");
    if path == mask_path {
        return Err("Snapshot filename must not use the reserved .pgm mask extension".into());
    }
    let size = buffer.size();
    let mut mask = format!("P5\n{} {}\n255\n", size.width * 2, size.height * 4).into_bytes();
    mask.extend(rgba(buffer).iter().map(|pixel| pixel[3]));
    fs::write(path, snapshot(buffer))?;
    fs::write(mask_path, mask)?;
    Ok(())
}

pub fn command(args: &[String], options: Options) -> Result<(), Box<dyn Error>> {
    if options.output == Output::Sixel
        && !matches!(args[0].as_str(), "--headless" | "--help" | "-h")
    {
        return Err("Sixel exports are not supported; use --raster for character-fallback exports or --sixel --headless to check drawing".into());
    }
    let height = options.height;
    match args[0].as_str() {
        "--snapshot" if [2, 3, 5].contains(&args.len()) => {
            let time: f64 = args.get(2).map_or(Ok(0.0), |s| s.parse())?;
            if !time.is_finite() || !(0.0..=86400.0).contains(&time) {
                return Err("SECONDS must be finite and between 0 and 86400".into());
            }
            let mut nursery = Nursery::new(options)?;
            nursery.time = time;
            let size: Size = if args.len() == 5 {
                let width: u16 = args[3].parse()?;
                let height: u16 = args[4].parse()?;
                if width == 0 || height == 0 || width > 512 || height > 128 {
                    return Err("size must be within 1..512 columns and 1..128 rows".into());
                }
                (width, height).into()
            } else { (PREVIEW_WIDTH, height).into() };
            nursery.options.height = size.height;
            save(&args[1], &frame(&mut nursery, size)?)?;
            println!("Saved terminal frame to {} at {time}s", args[1]);
        }
        "--frames" if args.len() == 2 => {
            fs::create_dir_all(&args[1])?;
            let mut nursery = Nursery::new(options)?;
            for index in 0..(crate::scene::PERIOD * 24.0).ceil() as usize {
                nursery.time = index as f64 / 24.0;
                let path = std::path::Path::new(&args[1]).join(format!("{index:04}.ppm"));
                save(path, &frame(&mut nursery, (PREVIEW_WIDTH, height).into())?)?;
            }
            println!("Saved {} seconds at 24 FPS to {}", crate::scene::PERIOD, args[1]);
        }
        "--action" if args.len() == 3 => {
            let action = crate::scene::Action::ALL.into_iter()
                .find(|action| action.name() == args[1]).ok_or("Unknown action; use idle/wave/walk/hop/think/sleep")?;
            fs::create_dir_all(&args[2])?;
            let mut nursery = Nursery::new(options)?;
            nursery.action = Some(action);
            for index in 0..(action.duration() * 24.0).ceil() as usize {
                nursery.time = index as f64 / 24.0;
                let path = std::path::Path::new(&args[2]).join(format!("{index:04}.ppm"));
                save(path, &frame(&mut nursery, (PREVIEW_WIDTH, height).into())?)?;
            }
            println!("Saved {} clip to {}", action.name(), args[2]);
        }
        "--headless" if args.len() == 1 => {
            let mut nursery = Nursery::new(options)?;
            let start = Instant::now();
            for index in 0..90 {
                nursery.time = index as f64 * 0.75;
                let buffer = frame(&mut nursery, (PREVIEW_WIDTH, height).into())?;
                assert!(buffer.content().iter().all(|cell| !cell.content.is_empty()));
                assert_eq!(buffer.has_raster(), options.output != Output::Quadrant);
            }
            println!("90 frames passed at {PREVIEW_WIDTH}x{height}; mean {:.2} ms/frame (drawing + encoding; no terminal I/O)", start.elapsed().as_secs_f64() * 1000.0 / 90.0);
        }
        "--help" | "-h" if args.len() == 1 => println!(
            "AI BABIES / ChatGPT, DeepSeek and Claude pets\n\nRun without arguments for automatic inline playback (up to 24 rows). Use --height ROWS to set the height (24 or 32, limited by the terminal during playback).\nTab: next character; g: ChatGPT; d: DeepSeek; c: Claude; q / Esc / Ctrl+C: exit; Space: pause; 1: idle; 2: wave; 3: walk; 4: hop; 5: think; 6: sleep; 0: automatic\n\n--static [--height ROWS] [--snapshot FILE.ppm]  (default: 144 columns × 24 rows; height: 1..40)\n[--character chatgpt|deepseek|claude] [--height ROWS] [EXPORT OPTION]  (either order, before export options; defaults: chatgpt, 24 rows)\n--raster  (opaque 320x320 source, inline HalfBlock preview; supports fallback exports)\n--sixel   (requires --features sixel, inline animation; no silent fallback)\n--background RRGGBB  (raster/sixel only; default black)\n--cell-pixels WxH    (sixel only: measured physical pixels per character cell, NOT terminal columns x rows; omit for startup discovery)\n--force-cell-pixels  (confirm a measured override matching the terminal grid)\nAll animation modes use inline output without an alternate screen or mouse capture. Sixel asserts terminal/multiplexer support for Sixel, local EL 2 graphics erasure, images following text scrolling, stable palettes, safe positioned Sixel output and transparent partial-band padding that does not erase adjacent cells. Inline reserves a bottom guard margin to prevent final-band scrolling; transparency alone is insufficient. Metrics do not detect Sixel support; restart after font/DPI changes. Native output uses at most 320x320 physical pixels (16 content rows + 1 blank guard row with 10x20 cells when space permits; no extra guard row with 8x18). Guard rows are ceil(((6 - h % 6) % 6) / h) for cell height h (5 at h=1). Content height is capped by --height and terminal height minus guard rows, scaling down to fit. Successful frames leave the cursor on the last content row; an exit newline can use the guard. See README for compatibility details.\nAll playback options precede export options. Sixel permits --headless only, not image exports. Headless checks drawing, not terminal compatibility.\n--headless\n--snapshot FILE.ppm [SECONDS [COLUMNS ROWS]]  (explicit dimensions override --height)\n--frames DIRECTORY  (one complete action playlist at 24 FPS)\n--action idle|wave|walk|hop|think|sleep DIRECTORY"
        ),
        _ => return Err("Unknown or incomplete arguments; use --help".into()),
    }
    Ok(())
}
