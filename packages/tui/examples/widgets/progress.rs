//! ProgressBar widget.
//!
//! Run with: `cargo run -p tui --example progress`

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
    panel::<()>()
        .title("ProgressBar")
        .padding(Padding::uniform(1))
        .gap(1)
        .child(
            progress_bar(0.18)
                .label("18%")
                .variant(ProgressBarVariant::Line),
        )
        .child(
            progress_bar(0.42)
                .label("42%")
                .variant(ProgressBarVariant::Block),
        )
        .child(
            progress_bar(0.66)
                .label("66%")
                .variant(ProgressBarVariant::Segmented),
        )
        .child(
            progress_bar(0.9)
                .label("90%")
                .variant(ProgressBarVariant::Classic),
        )
}
