//! Visual effect wrapper.
//!
//! Run with: `cargo run -p tui --example visual`

use tui::prelude::*;

#[derive(Debug, Clone)]
struct Msg(FrameInfo);

fn main() -> WidgetResult<()> {
    App::new(0.0)
        .on_frame(Msg)
        .mode(Mode::Inline)
        .update(|progress, msg, _ctx| *progress = (msg.0.since_start.as_secs_f64() % 2.0) / 2.0)
        .view(view)
        .run()
        .map(|_| ())
}

fn view(progress: &f64) -> impl Widget<Msg> {
    panel::<Msg>()
        .title("Visual")
        .padding(Padding::uniform(1))
        .gap(1)
        .child(
            visual(
                panel::<Msg>()
                    .title("Fade")
                    .padding(Padding::uniform(1))
                    .child(label("Visual effects wrap any widget.")),
            )
            .progress(*progress)
            .effect(VisualEffect::fade_in()),
        )
        .child(
            visual(label("Gradient phase").bold()).progress(1.0).effect(
                VisualEffect::gradient(Color::Cyan, Color::Magenta, GradientDirection::Horizontal)
                    .phase(*progress),
            ),
        )
}
