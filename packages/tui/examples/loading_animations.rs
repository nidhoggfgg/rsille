//! Loading Animations Example
//!
//! Demonstrates various loading and progress animations:
//! - Spinner animations
//! - Pulse effects
//! - Progress bars with smooth transitions
//! - Skeleton loading screens
//!
//! Run with: cargo run --example loading_animations

use std::time::Duration;
use tui::prelude::*;

/// Spinner frame characters
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Application state
#[derive(Debug)]
struct LoadingState {
    // Spinner animation
    spinner_frame: usize,
    spinner_animation: Animation,

    // Pulse animation
    pulse_opacity: OpacityTransition,
    pulse_direction: bool,

    // Progress animation
    progress: AnimatedValue<f32>,

    // Color pulse for status indicator
    status_color: ColorTransition,

    // Simulated loading stages
    loading_stage: usize,
    stage_names: Vec<String>,
}

/// Messages
#[allow(unused)]
#[derive(Clone, Debug)]
enum Message {
    Tick,
    StartLoading,
    NextStage,
    Reset,
}

/// Update function
fn update(state: &mut LoadingState, msg: Message) {
    match msg {
        Message::Tick => {
            // Update spinner
            if !state.spinner_animation.is_complete() {
                let progress = state.spinner_animation.eased_progress();
                state.spinner_frame = ((progress * SPINNER_FRAMES.len() as f32) as usize)
                    .min(SPINNER_FRAMES.len() - 1);
            }

            // Update pulse
            if state.pulse_opacity.is_complete() {
                // Reverse pulse direction
                state.pulse_direction = !state.pulse_direction;
                if state.pulse_direction {
                    state.pulse_opacity.fade_to(1.0, Duration::from_millis(800));
                } else {
                    state.pulse_opacity.fade_to(0.3, Duration::from_millis(800));
                }
            }

            // Restart spinner for continuous animation
            if state.spinner_animation.is_complete() {
                state.spinner_animation.reset();
                state.spinner_animation.start();
            }
        }

        Message::StartLoading => {
            state.spinner_animation.start();
            state.pulse_opacity.fade_to(0.3, Duration::from_millis(800));
            state.loading_stage = 0;

            // Animate progress to first stage
            let target = 1.0 / state.stage_names.len() as f32;
            state.progress = AnimatedValue::transition(0.0, target, Duration::from_millis(1500));
            state.progress.start();

            state
                .status_color
                .transition_to(Color::Yellow, Duration::from_millis(300));
        }

        Message::NextStage => {
            state.loading_stage += 1;

            if state.loading_stage < state.stage_names.len() {
                // Animate to next stage
                let current = state.progress.current();
                let target = (state.loading_stage + 1) as f32 / state.stage_names.len() as f32;
                state.progress =
                    AnimatedValue::transition(current, target, Duration::from_millis(1500));
                state.progress.start();

                // Change status color
                let color = match state.loading_stage {
                    0..=1 => Color::Yellow,
                    2..=3 => Color::Cyan,
                    _ => Color::Green,
                };
                state
                    .status_color
                    .transition_to(color, Duration::from_millis(300));
            } else {
                // Completed
                state
                    .status_color
                    .transition_to(Color::Green, Duration::from_millis(300));
            }
        }

        Message::Reset => {
            state.spinner_frame = 0;
            state.spinner_animation.reset();
            state.loading_stage = 0;
            state.progress = AnimatedValue::new(0.0);
            state.pulse_opacity = OpacityTransition::with_opacity(1.0);
            state
                .status_color
                .transition_to(Color::White, Duration::from_millis(300));
        }
    }
}

/// View function
fn view(state: &LoadingState) -> Container<Message> {
    // Spinner display
    let spinner_char = SPINNER_FRAMES[state.spinner_frame];
    let spinner_label = Label::new(format!("  {}  Loading...", spinner_char))
        .style(Style::default().fg(Color::Cyan).bold())
        .into();

    // Pulse indicator
    let pulse_opacity = state.pulse_opacity.opacity();
    let pulse_color = if pulse_opacity > 0.6 {
        Color::Green
    } else {
        Color::Indexed(2)
    };
    let pulse_indicator = Label::new(format!("● Status Pulse (opacity: {:.2})", pulse_opacity))
        .style(Style::default().fg(pulse_color))
        .into();

    // Progress bar
    let progress = state.progress.current();
    let bar_width = (progress * 50.0) as usize;
    let percentage = (progress * 100.0) as u32;
    let progress_bar = format!(
        "[{}{}] {}%",
        "█".repeat(bar_width),
        "░".repeat(50 - bar_width),
        percentage
    );
    let progress_label = Label::new(progress_bar)
        .style(Style::default().fg(Color::Blue))
        .into();

    // Loading stage
    let current_stage = if state.loading_stage < state.stage_names.len() {
        &state.stage_names[state.loading_stage]
    } else {
        "Complete!"
    };

    let stage_color = state.status_color.current();
    let stage_label = Label::new(format!("Current Stage: {}", current_stage))
        .style(Style::default().fg(stage_color).bold())
        .into();

    // Stage list
    let mut stage_widgets: Vec<AnyWidget<Message>> = vec![];
    for (i, stage) in state.stage_names.iter().enumerate() {
        let symbol = if i < state.loading_stage {
            "✓"
        } else if i == state.loading_stage {
            "→"
        } else {
            "○"
        };

        let color = if i < state.loading_stage {
            Color::Green
        } else if i == state.loading_stage {
            Color::Yellow
        } else {
            Color::Indexed(8)
        };

        stage_widgets.push(
            Label::new(format!("  {} {}", symbol, stage))
                .style(Style::default().fg(color))
                .into(),
        );
    }

    Container::vertical(vec![
        // Title
        Label::new("⏳ Loading Animations Demo")
            .style(Style::default().fg(Color::Cyan).bold())
            .into(),
        Label::new("").into(),
        // Spinner
        Label::new("1. Spinner Animation:")
            .style(Style::default().fg(Color::White).bold())
            .into(),
        spinner_label,
        Label::new("").into(),
        // Pulse
        Label::new("2. Pulse Effect:")
            .style(Style::default().fg(Color::White).bold())
            .into(),
        pulse_indicator,
        Label::new("").into(),
        // Progress
        Label::new("3. Smooth Progress Bar:")
            .style(Style::default().fg(Color::White).bold())
            .into(),
        progress_label,
        stage_label,
        Label::new("").into(),
        // Stages
        Label::new("4. Loading Stages:")
            .style(Style::default().fg(Color::White).bold())
            .into(),
        Container::vertical(stage_widgets).gap(0).into(),
        Label::new("").into(),
        // Controls
        KeyboardController::new()
            .on_key(KeyCode::Char('s'), || Message::StartLoading)
            .on_key(KeyCode::Char('r'), || Message::Reset)
            .into(),
        // Instructions
        Label::new("Controls:")
            .style(Style::default().fg(Color::Yellow))
            .into(),
        Label::new("  s: start loading | r: reset | Esc/q: quit")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
    ])
    .gap(0)
    .padding(Padding::new(2, 2, 1, 1))
}

fn main() -> Result<()> {
    let app = App::new(LoadingState {
        spinner_frame: 0,
        spinner_animation: Animation::new(Duration::from_millis(800)).looping(),
        pulse_opacity: OpacityTransition::new(),
        pulse_direction: false,
        progress: AnimatedValue::new(0.0),
        status_color: ColorTransition::new(Color::White),
        loading_stage: 0,
        stage_names: vec![
            "Initializing...".to_string(),
            "Loading resources...".to_string(),
            "Processing data...".to_string(),
            "Finalizing...".to_string(),
            "Ready!".to_string(),
        ],
    });

    app.run(update, view)?;
    Ok(())
}
