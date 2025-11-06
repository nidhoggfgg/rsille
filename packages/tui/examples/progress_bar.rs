//! Progress Bar Example
//!
//! Demonstrates progress bars with different visual representations
//! and progress tracking.
//!
//! Run with: cargo run --example progress_bar

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct ProgressState {
    download_progress: f32,
    upload_progress: f32,
    install_progress: f32,
}

/// Messages that can update the state
#[derive(Clone, Debug)]
enum Message {
    IncrementDownload,
    IncrementUpload,
    IncrementInstall,
    Reset,
}

/// Update function - handles state changes
fn update(state: &mut ProgressState, msg: Message) {
    match msg {
        Message::IncrementDownload => {
            state.download_progress = (state.download_progress + 0.05).min(1.0);
        }
        Message::IncrementUpload => {
            state.upload_progress = (state.upload_progress + 0.03).min(1.0);
        }
        Message::IncrementInstall => {
            state.install_progress = (state.install_progress + 0.07).min(1.0);
        }
        Message::Reset => {
            state.download_progress = 0.0;
            state.upload_progress = 0.0;
            state.install_progress = 0.0;
        }
    }
}

/// Helper function to render a progress bar manually
fn render_progress_bar(label: &str, progress: f32, color: Color) -> AnyWidget<Message> {
    let percentage = (progress * 100.0) as u32;
    let filled_width = (progress * 40.0) as usize;
    let bar = format!(
        "[{}{}] {}%",
        "█".repeat(filled_width),
        "░".repeat(40 - filled_width),
        percentage
    );

    Container::horizontal(vec![
        Label::new(format!("{:12}", label))
            .style(Style::default().fg(Color::Yellow))
            .into(),
        Label::new(bar)
            .style(Style::default().fg(color))
            .into(),
    ])
    .gap(1)
    .into()
}

/// View function - renders the UI from current state
fn view(state: &ProgressState) -> Container<Message> {
    Container::vertical(vec![
        // Title
        Label::new("📊 Progress Bar Example")
            .style(Style::default().fg(Color::Cyan).bold())
            .into(),
        Label::new("")
            .into(),
        // Progress bars
        render_progress_bar("Download:", state.download_progress, Color::Green),
        render_progress_bar("Upload:", state.upload_progress, Color::Blue),
        render_progress_bar("Install:", state.install_progress, Color::Magenta),
        Label::new("")
            .into(),
        // Status
        Label::new(format!(
            "Overall: {:.0}%",
            ((state.download_progress + state.upload_progress + state.install_progress) / 3.0)
                * 100.0
        ))
        .style(Style::default().fg(Color::White).bold())
        .into(),
        Label::new("")
            .into(),
        // Keyboard controls
        KeyboardController::new()
            .on_key(KeyCode::Char('d'), || Message::IncrementDownload)
            .on_key(KeyCode::Char('u'), || Message::IncrementUpload)
            .on_key(KeyCode::Char('i'), || Message::IncrementInstall)
            .on_key(KeyCode::Char('r'), || Message::Reset)
            .into(),
        // Instructions
        Label::new("Controls:")
            .style(Style::default().fg(Color::Yellow))
            .into(),
        Label::new("  d: increment download | u: increment upload | i: increment install")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
        Label::new("  r: reset all | Esc/q: quit")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
    ])
    .gap(0)
    .padding(Padding::new(2, 2, 1, 1))
}

fn main() -> Result<()> {
    let app = App::new(ProgressState {
        download_progress: 0.2,
        upload_progress: 0.5,
        install_progress: 0.8,
    });
    app.run(update, view)?;
    Ok(())
}
