//! A badge in the top-right corner showing how the last shell command went.
//!
//! Run with `cargo run -p rsille-overlay --example status_badge` from fish or
//! zsh, then run a few commands. Click `×` to clear the badge. Press Ctrl+]
//! or type `exit` to leave.
use rsille_overlay::{Anchor, Session, ShellEvent, WidgetOverlay};
use rsille_tui::prelude::*;
use std::time::{Duration, Instant};

#[derive(Default)]
struct Badge {
    running: Option<(String, Instant)>,
    last: Option<(String, i32, Duration)>,
}

#[derive(Debug)]
enum Msg {
    Exec(String),
    Done(i32),
    Clear,
}

fn update(badge: &mut Badge, msg: Msg) {
    match msg {
        Msg::Exec(line) => badge.running = Some((line, Instant::now())),
        Msg::Done(status) => {
            if let Some((line, start)) = badge.running.take() {
                badge.last = Some((line, status, start.elapsed()));
            }
        }
        Msg::Clear => badge.last = None,
    }
}

fn shorten(line: &str, max: usize) -> String {
    let mut out: String = line.chars().take(max).collect();
    if line.chars().count() > max {
        out.pop();
        out.push('…');
    }
    out
}

fn view(badge: &Badge) -> impl Widget<Msg> {
    let text = match (&badge.running, &badge.last) {
        (Some((line, _)), _) => label(format!("… {}", shorten(line, 22))).fg(Color::Yellow),
        (None, Some((line, 0, took))) => label(format!(
            "✔ {} {:.1}s",
            shorten(line, 18),
            took.as_secs_f32()
        ))
        .fg(Color::Green),
        (None, Some((line, status, _))) => {
            label(format!("✘ {status} {}", shorten(line, 18))).fg(Color::Red)
        }
        (None, None) => label(""),
    };
    row::<Msg>()
        .child(text.width(29))
        .child(button("×").on_click(|| Msg::Clear))
}

fn main() -> std::io::Result<()> {
    let app = App::new(Badge::default())
        .update(|badge, msg, _ctx| update(badge, msg))
        .view(view)
        .build();
    let overlay = WidgetOverlay::new(app, (32, 1))
        .anchor(Anchor::TopRight)
        .on_shell(|event| match event {
            ShellEvent::Exec(line) => Some(Msg::Exec(line.to_owned())),
            ShellEvent::Done(status) => Some(Msg::Done(status)),
            _ => None,
        });
    Session::new().mouse(true).run(overlay)?;
    Ok(())
}
