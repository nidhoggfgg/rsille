//! LogViewer widget.
//!
//! Run with: `cargo run -p tui --example log_viewer`

use tui::prelude::*;

fn main() -> WidgetResult<()> {
    App::new(())
        .mode(Mode::Inline)
        .update(|_, _, _ctx| {})
        .view(view)
        .run()
        .map(|_| ())
}

fn view(_: &()) -> impl Widget<()> {
    log_viewer().key("logs").height(9).lines([
        LogLine::new(LogLevel::Trace, "TRACE layout pass start"),
        LogLine::new(LogLevel::Debug, "DEBUG cache hit"),
        LogLine::new(LogLevel::Info, "INFO rendered 42 widgets"),
        LogLine::new(LogLevel::Warn, "WARN slow frame"),
        LogLine::new(LogLevel::Error, "ERROR example failure"),
    ])
}
