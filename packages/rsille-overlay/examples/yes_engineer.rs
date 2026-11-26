//! Click the button to send "yes, continue" and press Enter in the child.
use rsille_overlay::{Anchor, Session, WidgetOverlay};
use rsille_tui::prelude::*;
use std::{sync::mpsc, time::Duration};

enum Msg {
    TypeYes,
    Ready,
}

fn main() -> std::io::Result<()> {
    let (input, receiver) = mpsc::channel();
    let app = App::new(false)
        .with_theme(Theme::tokyo_night())
        .update(move |sent, msg, ctx| match msg {
            Msg::TypeYes => {
                input
                    .send(b"yes, continue\r".to_vec())
                    .expect("session is running");
                *sent = true;
                ctx.debounce("flash", Duration::from_millis(650), Msg::Ready);
            }
            Msg::Ready => *sent = false,
        })
        .view(|sent| {
            let caption = if *sent {
                "✓   SUBMITTED   ✓"
            } else {
                "» YES, CONTINUE «"
            };
            button(caption)
                .variant(ButtonVariant::Ghost)
                .focusable(false)
                .on_click(|| Msg::TypeYes)
        })
        .build();
    let overlay = WidgetOverlay::new(app, (23, 3)).anchor(Anchor::TopRight);
    Session::new()
        .mouse(true)
        .run_with_input(overlay, receiver)?;
    Ok(())
}
