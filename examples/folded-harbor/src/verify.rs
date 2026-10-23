//! Real application checks, frame export and CPU frame timing; no TTY needed.
use crate::{
    app, update, view,
    world::{Phase, World},
};
use rsille::{
    render::buffer::Buffer,
    tui::{prelude::*, runtime::ManualClock},
};
use std::{
    fmt::Write as _,
    time::{Duration, Instant},
};

fn key(c: KeyCode) -> Event {
    Event::Key(KeyEvent::new(c, KeyModifiers::empty()))
}
fn text(buffer: &Buffer) -> String {
    buffer
        .content()
        .chunks(buffer.size().width.max(1) as usize)
        .map(|row| {
            row.iter()
                .map(|cell| cell.content.symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn mouse(kind: MouseEventKind, column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::empty(),
    })
}

pub fn smoke() -> Result<(), Box<dyn std::error::Error>> {
    let clock = ManualClock::default();
    let mut runtime = app(World::default(), false)
        .with_clock(clock.clone())
        .update(update)
        .view(view)
        .build()
        .size(144, 44);
    runtime.step(&[])?;
    for _ in 0..13 {
        clock.advance(Duration::from_millis(100));
        runtime.step(&[])?;
    }
    assert!(text(runtime.frame()).contains("F O L D E D   H A R B O R"));
    assert!(text(runtime.frame()).contains("A / DEPARTURE"));
    assert!(runtime.frame().content().iter().any(|cell| cell
        .content
        .as_char()
        .is_some_and(|c| ('\u{2801}'..='\u{28ff}').contains(&c))));
    let yaw = runtime.state().target_yaw;
    runtime.step(&[key(KeyCode::Right)])?;
    assert!(runtime.state().target_yaw > yaw);
    let pitch = runtime.state().target_pitch;
    runtime.step(&[
        mouse(MouseEventKind::Down(MouseButton::Left), 30, 26),
        mouse(MouseEventKind::Drag(MouseButton::Left), 34, 24),
        mouse(MouseEventKind::Up(MouseButton::Left), 34, 24),
    ])?;
    assert!(
        runtime.state().target_pitch > pitch,
        "drag must bubble through the canvas"
    );
    runtime.step(&[mouse(MouseEventKind::ScrollUp, 30, 26)])?;
    assert!(runtime.state().target_zoom > 1.0);
    runtime.step(&[key(KeyCode::Char(' '))])?;
    assert_eq!(runtime.state().phase, Phase::Align);
    assert!(runtime.state().notice > 0.0);
    runtime.step(&[key(KeyCode::Char('h'))])?;
    for _ in 0..20 {
        clock.advance(Duration::from_millis(100));
        runtime.step(&[])?;
    }
    assert!(runtime.state().ready());
    assert_eq!(runtime.state().assists, 1);
    assert!(text(runtime.frame()).contains("PROJECTION LOCKED"));
    // Click the normal button anchored to the projected 3D gate.
    let state = runtime.state();
    let (from, _) = state.pair();
    let (x, y) = state.camera.project(from, 228.0, 148.0);
    let column = ((x / 2.0).round() as i32 + 2).clamp(0, 114 - 17) as u16 + 4;
    let row = ((y / 4.0).round() as i32 + 4).clamp(0, 36) as u16 + 5;
    runtime.step(&[
        mouse(MouseEventKind::Down(MouseButton::Left), column, row),
        mouse(MouseEventKind::Up(MouseButton::Left), column, row),
    ])?;
    assert_eq!(
        runtime.state().phase,
        Phase::Transit,
        "projected widget must be clickable"
    );
    runtime.step(&[key(KeyCode::Char('?'))])?;
    assert_eq!(runtime.focused_key(), Some("close-manual"));
    let time = runtime.state().time;
    clock.advance(Duration::from_secs(1));
    runtime.step(&[key(KeyCode::Tab), key(KeyCode::Char('r'))])?;
    assert_eq!(runtime.state().time, time);
    assert_eq!(runtime.focused_key(), Some("close-manual"));
    assert_eq!(runtime.state().phase, Phase::Transit);
    runtime.step(&[key(KeyCode::Esc), key(KeyCode::Char('p'))])?;
    let before = runtime.state().transit;
    clock.advance(Duration::from_secs(1));
    runtime.step(&[])?;
    assert_eq!(runtime.state().transit, before);
    runtime.step(&[key(KeyCode::Esc)])?;
    for _ in 0..27 {
        clock.advance(Duration::from_millis(100));
        runtime.step(&[])?;
    }
    assert_eq!(runtime.state().gate, 1);
    assert_eq!(runtime.state().phase, Phase::Align);
    for (w, h) in [(80, 24), (40, 12), (1, 1), (144, 44)] {
        runtime.step(&[Event::Resize(w, h)])?;
        assert_eq!(runtime.frame().size().width, w);
        if w == 80 {
            assert!(!text(runtime.frame()).contains("THE OBSERVER"));
        }
    }
    assert!(text(runtime.frame()).contains("THE OBSERVER"));
    runtime.step(&[key(KeyCode::Char('r')), key(KeyCode::Char('t'))])?;
    assert_eq!(runtime.state().gate, 0);
    assert_eq!(runtime.state().assists, 0);
    assert!(runtime.state().demo);
    runtime.step(&[key(KeyCode::Left)])?;
    assert!(!runtime.state().demo);
    runtime.step(&[key(KeyCode::Char('q'))])?;
    assert!(runtime.exit_reason().is_some());
    // Finish a real final crossing, then use the modal button to enter a new chapter.
    let mut last = World::default();
    last.gate = 2;
    last.phase = Phase::Transit;
    last.transit = 0.99;
    let mut runtime = app(last, false)
        .with_clock(clock.clone())
        .update(update)
        .view(view)
        .build()
        .size(144, 44);
    runtime.step(&[])?;
    clock.advance(Duration::from_millis(100));
    runtime.step(&[])?;
    assert_eq!(runtime.state().phase, Phase::Complete);
    assert_eq!(runtime.focused_key(), Some("continue"));
    runtime.step(&[key(KeyCode::Enter)])?;
    assert_eq!(runtime.state().chapter, 1);
    assert_eq!(runtime.state().phase, Phase::Align);
    println!("PASS: 3D projection, drag, zoom, lock, anchored button, fold transit, help focus, pause, resize, restart, demo override, chapter modal and exit.");
    Ok(())
}

fn demo_at(seconds: f64) -> World {
    let mut world = World::default();
    world.demo = true;
    for _ in 0..(seconds * 120.0).round() as usize {
        world.advance(1.0 / 120.0);
    }
    world
}
pub fn snapshot(path: &str, seconds: f64) -> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = app(demo_at(seconds), false)
        .with_clock(ManualClock::default())
        .update(update)
        .view(view)
        .build()
        .size(144, 44);
    runtime.step(&[])?;
    std::fs::write(path, svg(runtime.frame()))?;
    println!("Saved actual rsille frame at {seconds:.2}s to {path}");
    Ok(())
}
pub fn benchmark() -> Result<(), Box<dyn std::error::Error>> {
    for seconds in [1.8, 4.6, 22.0] {
        let clock = ManualClock::default();
        let mut runtime = app(demo_at(seconds), false)
            .with_clock(clock.clone())
            .update(update)
            .view(view)
            .build()
            .size(144, 44);
        let mut times = Vec::new();
        for i in 0..36 {
            clock.advance(Duration::from_secs_f64(1.0 / 60.0));
            let start = Instant::now();
            runtime.step(&[])?;
            if i >= 6 {
                times.push(start.elapsed().as_secs_f64() * 1000.0);
            }
        }
        let mean = times.iter().sum::<f64>() / times.len() as f64;
        times.sort_by(f64::total_cmp);
        println!("144x44 at t={seconds:4.1}: mean {mean:.2}ms / p95 {:.2}ms (update + widgets + raster; no terminal I/O)",times[28]);
    }
    Ok(())
}

fn svg(buffer: &Buffer) -> String {
    fn color(value: Option<crossterm::style::Color>, fallback: &str) -> String {
        match value {
            Some(crossterm::style::Color::Rgb { r, g, b }) => format!("#{r:02x}{g:02x}{b:02x}"),
            _ => fallback.to_owned(),
        }
    }
    let width = usize::from(buffer.size().width);
    let (w, h) = (width * 9 + 32, usize::from(buffer.size().height) * 18 + 32);
    let mut output = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\"><rect width=\"100%\" height=\"100%\" fill=\"#050a16\"/><g font-family=\"Menlo,DejaVu Sans Mono,monospace\" font-size=\"14\">");
    for (index, cell) in buffer.content().iter().enumerate() {
        let (x, y) = (index % width * 9 + 16, index / width * 18 + 16);
        let colors = cell.content.style.colors;
        let bg = color(colors.and_then(|c| c.background), "#050a16");
        let fg = color(colors.and_then(|c| c.foreground), "#daeaf3");
        let glyph = cell
            .content
            .symbol()
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        let _ = write!(output, "<rect x=\"{x}\" y=\"{y}\" width=\"9\" height=\"18\" fill=\"{bg}\"/><text x=\"{x}\" y=\"{}\" fill=\"{fg}\" xml:space=\"preserve\">{glyph}</text>", y + 14);
    }
    output.push_str("</g></svg>");
    output
}

#[cfg(test)]
mod tests {
    #[test]
    fn real_application_flow() {
        super::smoke().unwrap();
    }
}
