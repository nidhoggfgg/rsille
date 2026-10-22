//! LoadingIndicator widget.
//!
//! Run with: `cargo run -p tui --example loading_indicator`

use tui::prelude::*;

#[derive(Debug, Clone)]
struct Msg(FrameInfo);

fn main() -> WidgetResult<()> {
    App::new(0usize)
        .on_frame(Msg)
        .mode(Mode::Inline)
        .update(|frame, msg, _ctx| *frame = msg.0.frame as usize)
        .view(view)
        .run()
        .map(|_| ())
}

fn view(frame: &usize) -> impl Widget<Msg> {
    panel::<Msg>()
        .title("LoadingIndicator")
        .padding(Padding::uniform(1))
        .gap(1)
        .child(loading_indicator().frame(*frame).label("Syncing"))
        .child(loading_indicator().frame(frame / 2).label("Indexing"))
        .child(label(format!("frame: {frame}")))
}
