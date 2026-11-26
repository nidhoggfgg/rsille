//! Two independent widget overlays over any program.
//!
//! `cargo run -p rsille-overlay --example click_counter` wraps your shell;
//! `cargo run -p rsille-overlay --example click_counter -- top` wraps `top`.
//! Click the buttons in the corners. Press Ctrl+] to leave.
use rsille_overlay::{Anchor, Session, Stack, WidgetOverlay};
use rsille_tui::prelude::*;

#[derive(Debug)]
enum Msg {
    Add(i32),
    Reset,
}

fn counter() -> WidgetOverlay<i32, Msg> {
    let app = App::new(0)
        .update(|count: &mut i32, msg, _ctx| match msg {
            Msg::Add(n) => *count += n,
            Msg::Reset => *count = 0,
        })
        .view(|count: &i32| {
            row::<Msg>()
                .gap(1)
                .child(label(format!("{count:>4}")).bold())
                .child(button("+1").on_click(|| Msg::Add(1)))
                .child(button("-1").on_click(|| Msg::Add(-1)))
                .child(button("reset").on_click(|| Msg::Reset))
        })
        .build();
    WidgetOverlay::new(app, (24, 1))
}

fn main() -> std::io::Result<()> {
    let command: Vec<_> = std::env::args_os().skip(1).collect();
    let mut session = Session::new().mouse(true);
    if !command.is_empty() {
        session = session.command(command);
    }
    let overlays = Stack::new()
        .push(counter().anchor(Anchor::TopRight))
        .push(counter().anchor(Anchor::BottomRight));
    let status = session.run(overlays)?;
    std::process::exit(status);
}
