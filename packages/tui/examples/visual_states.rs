//! Visual States Example
//!
//! Demonstrates how UI can change based on application state.
//! Shows different visual representations for various states:
//! loading, success, error, warning, and information.
//!
//! Run with: cargo run --example visual_states

use tui::prelude::*;

/// Application states
#[derive(Debug, Clone, Copy, PartialEq)]
enum AppState {
    Loading,
    Success,
    Error,
    Warning,
    Info,
}

impl AppState {
    fn next(&self) -> Self {
        match self {
            AppState::Loading => AppState::Success,
            AppState::Success => AppState::Error,
            AppState::Error => AppState::Warning,
            AppState::Warning => AppState::Info,
            AppState::Info => AppState::Loading,
        }
    }

    fn prev(&self) -> Self {
        match self {
            AppState::Loading => AppState::Info,
            AppState::Info => AppState::Warning,
            AppState::Warning => AppState::Error,
            AppState::Error => AppState::Success,
            AppState::Success => AppState::Loading,
        }
    }

    fn name(&self) -> &str {
        match self {
            AppState::Loading => "Loading",
            AppState::Success => "Success",
            AppState::Error => "Error",
            AppState::Warning => "Warning",
            AppState::Info => "Information",
        }
    }

    fn icon(&self) -> &str {
        match self {
            AppState::Loading => "⟳",
            AppState::Success => "✓",
            AppState::Error => "✗",
            AppState::Warning => "⚠",
            AppState::Info => "ℹ",
        }
    }

    fn color(&self) -> Color {
        match self {
            AppState::Loading => Color::Cyan,
            AppState::Success => Color::Green,
            AppState::Error => Color::Red,
            AppState::Warning => Color::Yellow,
            AppState::Info => Color::Blue,
        }
    }

    fn description(&self) -> &str {
        match self {
            AppState::Loading => "Processing your request...",
            AppState::Success => "Operation completed successfully!",
            AppState::Error => "An error occurred during processing.",
            AppState::Warning => "Please review the following warnings.",
            AppState::Info => "Here is some useful information.",
        }
    }

    fn details(&self) -> Vec<&str> {
        match self {
            AppState::Loading => vec![
                "• Connecting to server",
                "• Authenticating user",
                "• Fetching data",
                "• Please wait...",
            ],
            AppState::Success => vec![
                "• All systems operational",
                "• Data synchronized",
                "• 100% complete",
                "• Ready to proceed",
            ],
            AppState::Error => vec![
                "• Connection timeout",
                "• Failed to authenticate",
                "• Retry in 30 seconds",
                "• Check network settings",
            ],
            AppState::Warning => vec![
                "• Disk space low (15% remaining)",
                "• Update available",
                "• Certificate expires soon",
                "• Review recommended",
            ],
            AppState::Info => vec![
                "• Version: 1.0.0",
                "• Last updated: 2 hours ago",
                "• Active users: 42",
                "• System healthy",
            ],
        }
    }
}

/// Application state
#[derive(Debug)]
struct VisualState {
    current_state: AppState,
    show_details: bool,
}

/// Messages that can update the state
#[derive(Clone, Debug)]
enum Message {
    NextState,
    PrevState,
    ToggleDetails,
    SetState(AppState),
}

/// Update function - handles state changes
fn update(state: &mut VisualState, msg: Message) {
    match msg {
        Message::NextState => {
            state.current_state = state.current_state.next();
        }
        Message::PrevState => {
            state.current_state = state.current_state.prev();
        }
        Message::ToggleDetails => {
            state.show_details = !state.show_details;
        }
        Message::SetState(new_state) => {
            state.current_state = new_state;
        }
    }
}

/// Create status indicator box
fn status_box(state: AppState) -> Vec<AnyWidget<Message>> {
    let color = state.color();
    let icon = state.icon();
    let name = state.name();

    vec![
        Label::new(format!("╔══════════════════════════════════════╗"))
            .style(Style::default().fg(color))
            .into(),
        Label::new(format!("║                                      ║"))
            .style(Style::default().fg(color))
            .into(),
        Label::new(format!("║       {}  {:^24}    ║", icon, name))
            .style(Style::default().fg(color).bold())
            .into(),
        Label::new(format!("║                                      ║"))
            .style(Style::default().fg(color))
            .into(),
        Label::new(format!("╚══════════════════════════════════════╝"))
            .style(Style::default().fg(color))
            .into(),
    ]
}

/// View function - renders the UI from current state
fn view(state: &VisualState) -> Container<Message> {
    let current = state.current_state;
    let color = current.color();

    let mut widgets = vec![
        // Title
        Label::new("🎭 Visual States Demo")
            .style(Style::default().fg(Color::Cyan).bold())
            .into(),
        Label::new("").into(),
    ];

    // Status box
    widgets.extend(status_box(current));

    widgets.extend(vec![
        Label::new("").into(),
        // Description
        Label::new(current.description())
            .style(Style::default().fg(color))
            .into(),
        Label::new("").into(),
    ]);

    // Details
    if state.show_details {
        widgets.push(
            Label::new("Details:")
                .style(Style::default().fg(Color::Yellow).bold())
                .into(),
        );
        for detail in current.details() {
            widgets.push(
                Label::new(detail)
                    .style(Style::default().fg(Color::White))
                    .into(),
            );
        }
        widgets.push(Label::new("").into());
    }

    // Quick state buttons
    widgets.extend(vec![
        Label::new("Quick Select:")
            .style(Style::default().fg(Color::Yellow).bold())
            .into(),
        Container::horizontal(vec![
            Button::new("[L] Loading")
                .style(if current == AppState::Loading {
                    Style::default().fg(Color::Cyan).bold()
                } else {
                    Style::default().fg(Color::Indexed(8))
                })
                .on_click(|| Message::SetState(AppState::Loading))
                .into(),
            Button::new("[S] Success")
                .style(if current == AppState::Success {
                    Style::default().fg(Color::Green).bold()
                } else {
                    Style::default().fg(Color::Indexed(8))
                })
                .on_click(|| Message::SetState(AppState::Success))
                .into(),
            Button::new("[E] Error")
                .style(if current == AppState::Error {
                    Style::default().fg(Color::Red).bold()
                } else {
                    Style::default().fg(Color::Indexed(8))
                })
                .on_click(|| Message::SetState(AppState::Error))
                .into(),
        ])
        .gap(2)
        .into(),
        Container::horizontal(vec![
            Button::new("[W] Warning")
                .style(if current == AppState::Warning {
                    Style::default().fg(Color::Yellow).bold()
                } else {
                    Style::default().fg(Color::Indexed(8))
                })
                .on_click(|| Message::SetState(AppState::Warning))
                .into(),
            Button::new("[I] Info")
                .style(if current == AppState::Info {
                    Style::default().fg(Color::Blue).bold()
                } else {
                    Style::default().fg(Color::Indexed(8))
                })
                .on_click(|| Message::SetState(AppState::Info))
                .into(),
        ])
        .gap(2)
        .into(),
        Label::new("").into(),
        // Keyboard controls
        KeyboardController::new()
            .on_key(KeyCode::Right, || Message::NextState)
            .on_key(KeyCode::Left, || Message::PrevState)
            .on_key(KeyCode::Char('d'), || Message::ToggleDetails)
            .on_key(KeyCode::Char('l'), || Message::SetState(AppState::Loading))
            .on_key(KeyCode::Char('s'), || Message::SetState(AppState::Success))
            .on_key(KeyCode::Char('e'), || Message::SetState(AppState::Error))
            .on_key(KeyCode::Char('w'), || Message::SetState(AppState::Warning))
            .on_key(KeyCode::Char('i'), || Message::SetState(AppState::Info))
            .into(),
        // Instructions
        Label::new("Controls:")
            .style(Style::default().fg(Color::Yellow))
            .into(),
        Label::new("  Left/Right arrows: cycle states | d: toggle details")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
        Label::new("  l/s/e/w/i: jump to specific state | Esc/q: quit")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
    ]);

    Container::vertical(widgets)
        .gap(0)
        .padding(Padding::new(2, 2, 1, 1))
}

fn main() -> Result<()> {
    let app = App::new(VisualState {
        current_state: AppState::Loading,
        show_details: true,
    });
    app.run(update, view)?;
    Ok(())
}
