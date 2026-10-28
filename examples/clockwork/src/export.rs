//! Capture the actual Draw output, so previews exercise the terminal encoders too.
use std::{error::Error, fmt::Write as _, fs, io, time::Instant};

use crossterm::style::Color;
use rsille::render::{area::Size, buffer::Buffer, chunk::Chunk, Draw};

use crate::{
    machine::{NAMES, PERIOD},
    Demo,
};
use rsille::canvas::Encoding;

pub fn frame(demo: &mut Demo, size: Size) -> Result<Buffer, Box<dyn Error>> {
    let mut buffer = Buffer::new(size);
    demo.draw(Chunk::new(&mut buffer, size.into())?)?;
    Ok(buffer)
}

pub fn command(args: &[String]) -> Result<(), Box<dyn Error>> {
    match args[0].as_str() {
        "--snapshot" if (2..=5).contains(&args.len()) => {
            let form = match args.get(2).map(String::as_str).unwrap_or("auto") {
                "auto" => None,
                value => {
                    let index: usize = value.parse()?;
                    if !(1..=3).contains(&index) { return Err(invalid("form must be auto or 1 through 3")); }
                    Some(index-1)
                }
            };
            let seconds: f64 = args.get(3).map_or(Ok(7.0), |s| s.parse())?;
            if !seconds.is_finite() || !(0.0..=180.0).contains(&seconds) { return Err(invalid("seconds must be between 0 and 180")); }
            let encoding = match args.get(4).map(String::as_str).unwrap_or("block") {
                "block" => Encoding::HalfBlock, "braille" => Encoding::Braille,
                _ => return Err(invalid("encoding must be block or braille")),
            };
            let mut demo = Demo { encoding, canvas: rsille::canvas::Canvas::with_encoding(encoding), ..Demo::default() };
            if let Some(index) = form { demo.machine.select(index); }
            demo.advance(seconds);
            fs::write(&args[1],svg(&frame(&mut demo,(144,44).into())?))?;
            println!("Saved {}: {} / {seconds}s",args[1],NAMES[demo.machine.form]);
        }
        "--benchmark" if args.len()==1 => {
            for (form,name) in NAMES.iter().enumerate() {
                let mut demo = Demo::default();
                demo.machine.select(form);
                demo.advance(7.0);
                let mut buffer = Buffer::new((144,44).into());
                let mut samples = Vec::new();
                for index in 0..70 {
                    let start = Instant::now();
                    demo.advance(1.0/30.0);
                    buffer.clear();
                    demo.draw(Chunk::new(&mut buffer,Size::from((144,44)).into())?)?;
                    if index>=10 { samples.push(start.elapsed().as_secs_f64()*1000.0); }
                }
                samples.sort_by(f64::total_cmp);
                println!("{name:<18} mean {:6.2} ms  p95 {:6.2} ms",samples.iter().sum::<f64>()/samples.len() as f64,samples[samples.len()*95/100]);
            }
            println!("144x44 Half-block; simulation + drawing + Canvas encoding, excluding terminal I/O.");
        }
        "--headless" if args.len()==1 => {
            let mut demo = Demo::default();
            demo.advance(PERIOD*3.0+7.0);
            let buffer = frame(&mut demo,(80,24).into())?;
            assert_eq!(demo.machine.form,0);
            assert!(buffer.content().iter().all(|c| !c.content.is_empty()));
            demo.machine.kick();
            demo.advance(4.0);
            assert!(demo.machine.parts.iter().all(|p|p.pose.center.length().is_finite()));
            let targets = crate::machine::targets(demo.machine.form,demo.machine.time as f32);
            let error = demo.machine.parts.iter().zip(targets).map(|(p,t)|(p.pose.center-t.center).length()).sum::<f32>()/crate::machine::PARTS as f32;
            assert!(error<0.3,"magnetic docking failed to recover after the pulse");
            println!("Headless journey passed: three assemblies, three releases, cycle wrap, magnetic pulse and reassembly.");
        }
        "--help" | "-h" if args.len()==1 => println!(
            "CLOCKWORK — a self-assembling mechanical menagerie\n\nRun without arguments for fullscreen playback.\n\
             --snapshot FILE.svg [auto|1-3] [SECONDS] [block|braille]\n--headless\n--benchmark\n\n\
             1-3: form; a: auto; k/right click: magnetic pulse; x: exploded view\n\
             drag/arrows: orbit; wheel/+/-: zoom; o: camera sway; b: encoder; h: clean\n\
             space/p: pause; q/esc: quit"
        ),
        _ => return Err(invalid("unknown or incomplete arguments; use --help")),
    }
    Ok(())
}

fn invalid(message: &str) -> Box<dyn Error> {
    io::Error::new(io::ErrorKind::InvalidInput, message).into()
}

fn rgb(color: Option<Color>) -> [u8; 3] {
    match color {
        Some(Color::Rgb { r, g, b }) => [r, g, b],
        _ => [0; 3],
    }
}

pub fn svg(buffer: &Buffer) -> String {
    let size = buffer.size();
    let mut output = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\"><rect width=\"100%\" height=\"100%\" fill=\"#000\"/><g font-family=\"DejaVu Sans Mono,monospace\" font-size=\"14\" xml:space=\"preserve\">",
        size.width as usize * 9, size.height as usize * 18, size.width as usize * 9, size.height as usize * 18
    );
    for y in 0..size.height {
        for x in 0..size.width {
            let Some(cell) = buffer.get((x, y).into()) else {
                continue;
            };
            let colors = cell.style.colors;
            let [r, g, b] = rgb(colors.and_then(|c| c.background));
            write!(output, "<rect x=\"{}\" y=\"{}\" width=\"9\" height=\"18\" fill=\"#{r:02x}{g:02x}{b:02x}\"/>", x as usize * 9, y as usize * 18).unwrap();
            // Block glyphs represent exact sample rectangles. Avoid font metrics
            // introducing artificial gaps between pixels in the exported preview.
            if let Some(ch @ ('▀' | '▄' | '█')) = cell.as_char() {
                let [r, g, b] = rgb(colors.and_then(|c| c.foreground));
                let offset = if ch == '▄' { 9 } else { 0 };
                let height = if ch == '█' { 18 } else { 9 };
                write!(output,"<rect x=\"{}\" y=\"{}\" width=\"9\" height=\"{height}\" fill=\"#{r:02x}{g:02x}{b:02x}\"/>",x as usize*9,y as usize*18+offset).unwrap();
                continue;
            }
            if let Some(ch @ '\u{2800}'..='\u{28ff}') = cell.as_char() {
                let mask = ch as u32 - 0x2800;
                let [r, g, b] = rgb(colors.and_then(|c| c.foreground));
                for (bit, dx, dy) in [
                    (1, 0, 0),
                    (2, 0, 1),
                    (4, 0, 2),
                    (8, 1, 0),
                    (16, 1, 1),
                    (32, 1, 2),
                    (64, 0, 3),
                    (128, 1, 3),
                ] {
                    if mask & bit != 0 {
                        write!(output,"<circle cx=\"{}\" cy=\"{}\" r=\"1.55\" fill=\"#{r:02x}{g:02x}{b:02x}\"/>",x as f32*9.0+2.25+dx as f32*4.5,y as f32*18.0+2.25+dy as f32*4.5).unwrap();
                    }
                }
                continue;
            }
            let symbol = cell.symbol();
            if symbol.trim().is_empty() {
                continue;
            }
            let text = symbol
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");
            let [r, g, b] = rgb(colors.and_then(|c| c.foreground));
            write!(
                output,
                "<text x=\"{}\" y=\"{}\" fill=\"#{r:02x}{g:02x}{b:02x}\">{text}</text>",
                x as usize * 9,
                y as usize * 18 + 14
            )
            .unwrap();
        }
    }
    output.push_str("</g></svg>\n");
    output
}
