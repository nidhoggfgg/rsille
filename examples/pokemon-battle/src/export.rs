//! Export actual rsille terminal cells, including Half-block and Braille encoding.
use crate::{app, update, view, Game};
use crossterm::style::Color;
use rsille::{
    canvas::Encoding,
    render::{area::Size, buffer::Buffer},
    tui::runtime::ManualClock,
};
use std::{error::Error, fmt::Write as _, fs, time::Duration};

pub fn frame(game: &Game, size: Size) -> Result<Buffer, Box<dyn Error>> {
    let mut runtime = app(Game {
        viewport: size,
        ..game.clone()
    })
    .with_clock(ManualClock::default())
    .update(update)
    .view(view)
    .build()
    .size(size.width, size.height);
    runtime.step(&[])?;
    Ok(runtime.frame().clone())
}

pub fn command(args: &[String]) -> Result<(), Box<dyn Error>> {
    match args[0].as_str() {
        "--snapshot" if (2..=4).contains(&args.len()) => {
            let time: f64 = args.get(2).map_or(Ok(0.0), |v| v.parse())?;
            if !time.is_finite() || !(0.0..=3600.0).contains(&time) {
                return Err("seconds must be finite and between 0 and 3600".into());
            }
            let encoding = match args.get(3).map(String::as_str).unwrap_or("block") {
                "block" => Encoding::HalfBlock,
                "braille" => Encoding::Braille,
                _ => return Err("encoding must be block or braille".into()),
            };
            let mut demo = Game {
                encoding,
                ..Game::default()
            };
            if time > 0.0 {
                demo.battle.choose(0);
                demo.battle.advance(time);
            }
            fs::write(&args[1], svg(&frame(&demo, (160, 50).into())?))?;
            println!("Saved actual rsille frame to {}", args[1]);
        }
        "--headless" if args.len() == 1 => {
            let clock = ManualClock::default();
            let mut runtime = app(Game {
                viewport: (80, 24).into(),
                ..Game::default()
            })
            .with_clock(clock.clone())
            .update(update)
            .view(view)
            .build()
            .size(80, 24);
            runtime.step(&[])?;
            for _ in 0..16 {
                if runtime.state().battle.phase != crate::battle::Phase::Choose {
                    break;
                }
                let battle = runtime.state().battle;
                let key = if !battle.paralyzed {
                    '4'
                } else if battle.pp[0] > 0 {
                    '1'
                } else {
                    '3'
                };
                runtime.step(&[crossterm::event::Event::Key(
                    crossterm::event::KeyEvent::new(
                        crossterm::event::KeyCode::Char(key),
                        crossterm::event::KeyModifiers::NONE,
                    ),
                )])?;
                for _ in 0..40 {
                    clock.advance(Duration::from_millis(100));
                    runtime.step(&[])?;
                }
            }
            assert_eq!(runtime.state().battle.phase, crate::battle::Phase::Won);
            println!("PASS: native TUI runtime, skill commands, PP, damage, replies, paralysis and victory.");
        }
        "--help" | "-h" if args.len() == 1 => println!(
            "Pokemon Battle: a turn-based 3D duel\n\nRun without arguments to play as Pikachu.\n\
            --snapshot FILE.svg [SECONDS] [block|braille]\n--headless\n\n\
            1-4: use move; arrows/Tab + Enter/Space: select and confirm; click: use move\n\
            drag/WASD: orbit; wheel/+/-: zoom; C: reset camera; B: encoding\n\
            P: pause; R: rematch; Q/Esc/Ctrl+C: quit"
        ),
        _ => return Err("unknown or incomplete arguments; use --help".into()),
    }
    Ok(())
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
