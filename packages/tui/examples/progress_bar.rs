use tui::prelude::*;

#[derive(Clone, Debug)]
enum Message {}

#[derive(Debug)]
struct AppState {}

impl AppState {
    fn new() -> Self {
        Self {}
    }
}

fn update(_state: &mut AppState, _message: Message) {}

fn view(_state: &AppState) -> impl Layout<Message> {
    col()
        .padding(Padding::new(2, 2, 2, 2))
        .gap(2)
        .child(label("ProgressBar Examples").fg(Color::Cyan).bold())
        .child(divider().horizontal())
        // Basic progress bar
        .child(label("Basic Progress (50%):"))
        .child(progress_bar().progress(0.5).width(40))
        // Progress bar with percentage
        .child(label("With Percentage Display:"))
        .child(progress_bar().progress(0.75).show_percentage().width(40))
        // Progress bar with custom label
        .child(label("With Custom Label:"))
        .child(progress_bar().progress(0.3).label("Downloading").width(40))
        // Progress bar with label and percentage
        .child(label("Label + Percentage:"))
        .child(
            progress_bar()
                .progress(0.65)
                .label("Installing")
                .show_percentage()
                .width(40),
        )
        // Label on the right
        .child(label("Label Position: Right"))
        .child(
            progress_bar()
                .progress(0.85)
                .label("Processing")
                .show_percentage()
                .label_position(LabelPosition::Right)
                .width(40),
        )
        // Indeterminate mode
        .child(label("Indeterminate Mode:"))
        .child(progress_bar().indeterminate().label("Loading...").width(40))
        // Custom colors
        .child(label("Custom Colors:"))
        .child(
            progress_bar()
                .progress(0.9)
                .fg(Color::Green)
                .show_percentage()
                .width(40),
        )
        // Custom characters
        .child(label("Custom Characters:"))
        .child(
            progress_bar()
                .progress(0.6)
                .filled_char('━')
                .empty_char('─')
                .show_percentage()
                .width(40),
        )
        // Full width (flexible)
        .child(label("Full Width (Flexible):"))
        .child(progress_bar().progress(0.45).show_percentage())
        .child(divider().horizontal())
        .child(label("Different Progress Levels:").fg(Color::Yellow))
        .child(
            col()
                .gap(1)
                .child(progress_bar().progress(0.0).show_percentage().width(30))
                .child(progress_bar().progress(0.25).show_percentage().width(30))
                .child(progress_bar().progress(0.5).show_percentage().width(30))
                .child(progress_bar().progress(0.75).show_percentage().width(30))
                .child(progress_bar().progress(1.0).show_percentage().width(30)),
        )
}

fn main() -> WidgetResult<()> {
    let app = App::new(AppState::new());
    app.run_inline(update, view)?;
    Ok(())
}
