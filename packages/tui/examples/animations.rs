//! Animation Showcase Example
//!
//! Demonstrates various animation capabilities including:
//! - Fade in/out transitions
//! - Slide animations
//! - Color transitions
//! - Easing functions
//! - Looping animations
//!
//! Run with: cargo run --example animations

use tui::prelude::*;
use std::time::Duration;

/// Application state
#[derive(Debug)]
struct AnimationState {
    // Fade animation
    fade_opacity: OpacityTransition,
    fade_visible: bool,

    // Color animation
    color_transition: ColorTransition,
    color_index: usize,

    // Position animation
    position: PositionTransition,
    position_x: f32,

    // Easing comparison
    easing_animations: Vec<(String, Animation)>,
    easing_started: bool,

    // Looping animation
    pulse_scale: ScaleTransition,
    pulse_direction: bool,
}

/// Messages that can update the state
#[derive(Clone, Debug)]
enum Message {
    ToggleFade,
    NextColor,
    MoveLeft,
    MoveRight,
    StartEasings,
    TogglePulse,
    Tick, // For animation updates
}

/// Update function - handles state changes
fn update(state: &mut AnimationState, msg: Message) {
    match msg {
        Message::ToggleFade => {
            if state.fade_visible {
                state.fade_opacity.fade_out(Duration::from_millis(500));
                state.fade_visible = false;
            } else {
                state.fade_opacity.fade_in(Duration::from_millis(500));
                state.fade_visible = true;
            }
        }

        Message::NextColor => {
            let colors = [
                Color::Red,
                Color::Green,
                Color::Blue,
                Color::Yellow,
                Color::Magenta,
                Color::Cyan,
            ];
            state.color_index = (state.color_index + 1) % colors.len();
            state.color_transition.transition_to(
                colors[state.color_index],
                Duration::from_millis(600),
            );
        }

        Message::MoveLeft => {
            state.position_x = (state.position_x - 10.0).max(0.0);
            state.position.slide_to(state.position_x, 0.0, Duration::from_millis(400));
        }

        Message::MoveRight => {
            state.position_x = (state.position_x + 10.0).min(50.0);
            state.position.slide_to(state.position_x, 0.0, Duration::from_millis(400));
        }

        Message::StartEasings => {
            if !state.easing_started {
                for (_, anim) in &mut state.easing_animations {
                    anim.start();
                }
                state.easing_started = true;
            } else {
                // Reset
                for (_, anim) in &mut state.easing_animations {
                    anim.reset();
                }
                state.easing_started = false;
            }
        }

        Message::TogglePulse => {
            if state.pulse_direction {
                state.pulse_scale.scale_to(1.5, Duration::from_millis(500));
            } else {
                state.pulse_scale.scale_to(1.0, Duration::from_millis(500));
            }
            state.pulse_direction = !state.pulse_direction;
        }

        Message::Tick => {
            // Animation updates handled automatically through rendering
        }
    }
}

/// View function - renders the UI from current state
fn view(state: &AnimationState) -> Container<Message> {
    // Fade demo
    let fade_text = format!(
        "Fade Animation (Opacity: {:.2})",
        state.fade_opacity.opacity()
    );
    let fade_label = if state.fade_opacity.is_visible() {
        Label::new(fade_text)
            .style(Style::default().fg(Color::Green))
            .into()
    } else {
        Label::new(fade_text)
            .style(Style::default().fg(Color::Indexed(8)))
            .into()
    };

    // Color transition demo
    let current_color = state.color_transition.current();
    let color_label = Label::new("Color Transition Demo")
        .style(Style::default().fg(current_color).bold())
        .into();

    // Position animation demo
    let position_padding = (state.position.position().0 as u16).min(50);
    let position_label = Label::new("→ Sliding Text")
        .style(Style::default().fg(Color::Cyan))
        .into();
    let position_container: AnyWidget<Message> =
        Container::horizontal(vec![position_label])
            .padding(Padding::new(position_padding, 0, 0, 0))
            .into();

    // Easing comparison
    let mut easing_widgets: Vec<AnyWidget<Message>> = vec![
        Label::new("Easing Functions Comparison:")
            .style(Style::default().fg(Color::Yellow).bold())
            .into(),
    ];

    for (name, animation) in &state.easing_animations {
        let progress = animation.eased_progress();
        let bar_width = (progress * 40.0) as usize;
        let bar = format!(
            "{:15} [{}{}] {:.0}%",
            name,
            "█".repeat(bar_width),
            "░".repeat(40 - bar_width),
            progress * 100.0
        );
        easing_widgets.push(
            Label::new(bar)
                .style(Style::default().fg(Color::Blue))
                .into(),
        );
    }

    // Pulse/scale demo
    let pulse_text = format!("Pulse Scale: {:.2}x", state.pulse_scale.scale());
    let pulse_label = Label::new(pulse_text)
        .style(Style::default().fg(Color::Magenta).bold())
        .into();

    Container::vertical(vec![
        // Title
        Label::new("🎨 Animation System Demo")
            .style(Style::default().fg(Color::Cyan).bold())
            .into(),
        Label::new("").into(),
        // Fade section
        Label::new("1. Fade Transition:")
            .style(Style::default().fg(Color::White).bold())
            .into(),
        fade_label,
        Label::new("").into(),
        // Color section
        Label::new("2. Color Transition:")
            .style(Style::default().fg(Color::White).bold())
            .into(),
        color_label,
        Label::new("").into(),
        // Position section
        Label::new("3. Position Transition:")
            .style(Style::default().fg(Color::White).bold())
            .into(),
        position_container,
        Label::new("").into(),
        // Easing section
        Label::new("4. Easing Functions:")
            .style(Style::default().fg(Color::White).bold())
            .into(),
        Container::vertical(easing_widgets).gap(0).into(),
        Label::new("").into(),
        // Pulse section
        Label::new("5. Scale/Pulse Effect:")
            .style(Style::default().fg(Color::White).bold())
            .into(),
        pulse_label,
        Label::new("").into(),
        // Controls
        KeyboardController::new()
            .on_key(KeyCode::Char('f'), || Message::ToggleFade)
            .on_key(KeyCode::Char('c'), || Message::NextColor)
            .on_key(KeyCode::Left, || Message::MoveLeft)
            .on_key(KeyCode::Right, || Message::MoveRight)
            .on_key(KeyCode::Char('e'), || Message::StartEasings)
            .on_key(KeyCode::Char('p'), || Message::TogglePulse)
            .into(),
        // Instructions
        Label::new("Controls:")
            .style(Style::default().fg(Color::Yellow))
            .into(),
        Label::new("  f: toggle fade | c: cycle colors | ←/→: move position")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
        Label::new("  e: start/reset easings | p: toggle pulse | Esc/q: quit")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
    ])
    .gap(0)
    .padding(Padding::new(2, 2, 1, 1))
}

fn main() -> Result<()> {
    // Initialize easing animations for comparison
    let easing_animations = vec![
        ("Linear".to_string(), Animation::new(Duration::from_secs(2))),
        (
            "EaseInQuad".to_string(),
            Animation::new(Duration::from_secs(2)).with_easing(Easing::EaseInQuad),
        ),
        (
            "EaseOutQuad".to_string(),
            Animation::new(Duration::from_secs(2)).with_easing(Easing::EaseOutQuad),
        ),
        (
            "EaseInOutCubic".to_string(),
            Animation::new(Duration::from_secs(2)).with_easing(Easing::EaseInOutCubic),
        ),
        (
            "EaseOutBounce".to_string(),
            Animation::new(Duration::from_secs(2)).with_easing(Easing::EaseOutBounce),
        ),
        (
            "EaseOutElastic".to_string(),
            Animation::new(Duration::from_secs(2)).with_easing(Easing::EaseOutElastic),
        ),
    ];

    let app = App::new(AnimationState {
        fade_opacity: OpacityTransition::new(),
        fade_visible: true,
        color_transition: ColorTransition::new(Color::Red),
        color_index: 0,
        position: PositionTransition::new(),
        position_x: 0.0,
        easing_animations,
        easing_started: false,
        pulse_scale: ScaleTransition::new(),
        pulse_direction: false,
    });

    app.run(update, view)?;
    Ok(())
}
