//! An external producer wakes a static UI. Esc exits.
//! Run with: cargo run -p tui --example background
use std::time::Duration;
use tui::prelude::*;

fn main() -> WidgetResult<()> {
    let app = App::new(false).build(
        |ready, ()| *ready = true,
        |ready| {
            label(if *ready {
                "Background complete"
            } else {
                "Waiting for background work"
            })
        },
    );
    let handle = app.handle();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        let _ = handle.send(());
    });
    app.run()
}
