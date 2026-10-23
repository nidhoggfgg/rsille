//! Exercise the real rsille runtime, and export its actual terminal buffer.
use crate::{
    app, update, view,
    world::{Block, World},
};
use rsille::{
    render::buffer::Buffer,
    tui::{prelude::*, runtime::ManualClock},
};
use std::{
    fmt::Write as _,
    time::{Duration, Instant},
};

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
}
fn mouse(kind: MouseEventKind, column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::empty(),
    })
}
fn text(frame: &Buffer) -> String {
    frame
        .content()
        .iter()
        .map(|cell| cell.content.symbol())
        .collect()
}

pub fn smoke() -> Result<(), Box<dyn std::error::Error>> {
    let clock = ManualClock::default();
    let mut runtime = app(World::default())
        .with_clock(clock.clone())
        .update(update)
        .view(view)
        .build()
        .size(120, 40);
    runtime.step(&[])?;
    assert!(text(runtime.frame()).contains("M I N E C R A F T"));
    assert!(
        runtime
            .frame()
            .content()
            .iter()
            .filter(|c| c.content.as_char() == Some('▀'))
            .count()
            > 2500
    );
    let spawn = runtime.state().player;
    runtime.step(&[key(KeyCode::Char('w'))])?;
    assert_eq!(
        runtime.state().player,
        spawn,
        "input must wait for frame time"
    );
    clock.advance(Duration::from_secs_f64(1.0 / 60.0));
    runtime.step(&[])?;
    assert!(runtime.state().player[2] > spawn[2]);
    let yaw = runtime.state().yaw;
    let pitch = runtime.state().pitch;
    runtime.step(&[
        key(KeyCode::Right),
        key(KeyCode::Up),
        key(KeyCode::Char('4')),
    ])?;
    assert_eq!(runtime.state().selected, 3);
    assert_eq!(runtime.state().yaw, yaw);
    assert_eq!(runtime.state().pitch, pitch);
    clock.advance(Duration::from_secs_f64(1.0 / 60.0));
    runtime.step(&[])?;
    assert!(runtime.state().yaw > yaw && runtime.state().yaw - yaw < 0.09);
    assert!(runtime.state().pitch > pitch && runtime.state().pitch - pitch < 0.075);
    let yaw = runtime.state().yaw;
    runtime.step(&[
        mouse(MouseEventKind::Down(MouseButton::Middle), 50, 15),
        mouse(MouseEventKind::Drag(MouseButton::Middle), 54, 13),
        mouse(MouseEventKind::Up(MouseButton::Middle), 54, 13),
        mouse(MouseEventKind::ScrollDown, 50, 15),
    ])?;
    assert!(runtime.state().yaw > yaw);
    assert_eq!(runtime.state().selected, 4);
    // Click the second slot in the responsive 120-column hotbar.
    runtime.step(&[mouse(MouseEventKind::Down(MouseButton::Left), 32, 36)])?;
    assert_eq!(runtime.state().selected, 1);
    runtime.step(&[
        key(KeyCode::Char('f')),
        key(KeyCode::Char(' ')),
        key(KeyCode::Char('n')),
    ])?;
    assert!(runtime.state().flying && runtime.state().night);
    clock.advance(Duration::from_secs_f64(1.0 / 60.0));
    runtime.step(&[])?;
    assert!(runtime.state().player[1] > spawn[1]);
    runtime.step(&[key(KeyCode::Char('?'))])?;
    assert_eq!(runtime.focused_key(), Some("resume"));
    let before = runtime.state().player;
    let time = runtime.state().time;
    clock.advance(Duration::from_secs(1));
    runtime.step(&[
        key(KeyCode::Char('w')),
        key(KeyCode::Char('e')),
        key(KeyCode::Char('r')),
    ])?;
    assert_eq!(runtime.state().player, before);
    assert_eq!(runtime.state().time, time);
    runtime.step(&[key(KeyCode::Enter), key(KeyCode::Esc)])?;
    assert!(!runtime.state().help && runtime.state().paused);
    runtime.step(&[key(KeyCode::Esc)])?;
    assert!(!runtime.state().paused);
    // A pause/guide must not leave queued motion to replay when closed.
    let stopped = runtime.state().player;
    let yaw = runtime.state().yaw;
    clock.advance(Duration::from_millis(80));
    runtime.step(&[])?;
    assert_eq!(runtime.state().player, stopped);
    assert_eq!(runtime.state().yaw, yaw);
    for open in [KeyCode::Char('p'), KeyCode::Char('?')] {
        runtime.step(&[key(KeyCode::Char('w')), key(KeyCode::Right), key(open)])?;
        runtime.step(&[key(KeyCode::Esc)])?;
        clock.advance(Duration::from_millis(80));
        runtime.step(&[])?;
        assert_eq!(runtime.state().player, stopped);
        assert_eq!(runtime.state().yaw, yaw);
    }
    for (w, h) in [(80, 24), (32, 10), (20, 8), (1, 1), (120, 40)] {
        runtime.step(&[Event::Resize(w, h)])?;
        assert_eq!(runtime.frame().size().width, w);
    }
    runtime.step(&[key(KeyCode::Char('q'))])?;
    assert!(runtime.exit_reason().is_some());

    // Real keyboard and mouse editing against a controlled target.
    let mut world = World {
        player: [28.5, 18.88, 17.5],
        yaw: 0.0,
        pitch: 0.0,
        flying: true,
        ..World::default()
    };
    world.set([28, 20, 20], Block::Stone);
    let mut runtime = app(world)
        .with_clock(clock.clone())
        .update(update)
        .view(view)
        .build()
        .size(80, 24);
    runtime.step(&[])?;
    runtime.step(&[key(KeyCode::Char('4')), key(KeyCode::Char('r'))])?;
    assert_eq!(runtime.state().get([28, 20, 19]), Block::Wood);
    runtime.step(&[mouse(MouseEventKind::Down(MouseButton::Left), 30, 12)])?;
    assert_eq!(runtime.state().get([28, 20, 19]), Block::Air);
    runtime.step(&[mouse(MouseEventKind::Down(MouseButton::Right), 30, 12)])?;
    assert_eq!(runtime.state().get([28, 20, 19]), Block::Wood);
    runtime.step(&[
        key(KeyCode::Char('w')),
        key(KeyCode::Right),
        key(KeyCode::Home),
    ])?;
    let spawn = runtime.state().player;
    let yaw = runtime.state().yaw;
    clock.advance(Duration::from_millis(80));
    runtime.step(&[])?;
    assert_eq!(runtime.state().player, spawn);
    assert_eq!(runtime.state().yaw, yaw);
    assert_eq!(runtime.state().get([28, 20, 19]), Block::Wood);
    assert!(runtime.state().free(runtime.state().player));
    println!("PASS: voxel frame, smooth keyboard walk/look, direct mouse drag, hotbar, flight, night, help focus, pause/input reset, resize, editing, spawn and exit.");
    Ok(())
}

pub fn snapshot(path: &str, night: bool) -> Result<(), Box<dyn std::error::Error>> {
    let world = World {
        night,
        ..World::default()
    };
    let mut runtime = app(world)
        .with_clock(ManualClock::default())
        .update(update)
        .view(view)
        .build()
        .size(144, 44);
    runtime.step(&[])?;
    std::fs::write(path, svg(runtime.frame()))?;
    println!("Saved actual rsille terminal frame to {path}");
    Ok(())
}

pub fn benchmark() -> Result<(), Box<dyn std::error::Error>> {
    let clock = ManualClock::default();
    let mut runtime = app(World::default())
        .with_clock(clock.clone())
        .update(update)
        .view(view)
        .build()
        .size(144, 44);
    let mut times = Vec::new();
    for i in 0..100 {
        clock.advance(Duration::from_secs_f64(1.0 / 60.0));
        let start = Instant::now();
        runtime.step(&[])?;
        if i >= 10 {
            times.push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
    let mean = times.iter().sum::<f64>() / times.len() as f64;
    times.sort_by(f64::total_cmp);
    println!(
        "144x44, CPU update + render, excluding terminal I/O: mean {mean:.2} ms / p95 {:.2} ms",
        times[85]
    );
    Ok(())
}

fn svg(buffer: &Buffer) -> String {
    fn rgb(value: Option<Color>, fallback: &str) -> String {
        match value {
            Some(Color::Rgb(r, g, b)) => format!("#{r:02x}{g:02x}{b:02x}"),
            _ => fallback.to_owned(),
        }
    }
    let width = buffer.size().width as usize;
    let mut output = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\" shape-rendering=\"crispEdges\"><rect width=\"100%\" height=\"100%\" fill=\"#1c1e1a\"/><g font-family=\"Menlo,monospace\" font-size=\"14\">", width * 10, buffer.size().height as usize * 20, width * 10, buffer.size().height as usize * 20);
    for (index, cell) in buffer.content().iter().enumerate() {
        let (x, y) = (index % width * 10, index / width * 20);
        let colors = cell.content.style.colors;
        let convert = |c| match c {
            crossterm::style::Color::Rgb { r, g, b } => Color::Rgb(r, g, b),
            _ => scene_fallback(),
        };
        let bg = rgb(colors.and_then(|c| c.background).map(convert), "#1c1e1a");
        let fg = rgb(colors.and_then(|c| c.foreground).map(convert), "#ececdb");
        let _ = write!(
            output,
            "<rect x=\"{x}\" y=\"{y}\" width=\"10\" height=\"20\" fill=\"{bg}\"/>"
        );
        if cell.content.as_char() == Some('▀') {
            let _ = write!(
                output,
                "<rect x=\"{x}\" y=\"{y}\" width=\"10\" height=\"10\" fill=\"{fg}\"/>"
            );
        } else {
            let glyph = cell
                .content
                .symbol()
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");
            let _ = write!(
                output,
                "<text x=\"{x}\" y=\"{}\" fill=\"{fg}\" xml:space=\"preserve\">{glyph}</text>",
                y + 16
            );
        }
    }
    output.push_str("</g></svg>");
    output
}
fn scene_fallback() -> Color {
    Color::Rgb(236, 236, 219)
}

#[cfg(test)]
mod tests {
    #[test]
    fn real_game_flow() {
        super::smoke().unwrap();
    }
}
