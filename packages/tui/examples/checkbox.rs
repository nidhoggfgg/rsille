//! Checkbox Component Example
//!
//! Demonstrates:
//! - Individual checkbox functionality
//! - CheckboxGroup with vertical and horizontal layout
//! - Option navigation with Tab/Shift+Tab (within group)
//! - Arrow key navigation within groups
//! - Keyboard toggle with Enter/Space
//! - Mouse click to toggle
//! - Change event handling
//! - Visual focus indicators
//!
//! Controls:
//! - Tab: Navigate to next option within group
//! - Shift+Tab: Navigate to previous option within group
//! - Up/Down: Navigate within vertical checkbox group
//! - Left/Right: Navigate within horizontal checkbox group
//! - Enter/Space: Toggle focused checkbox
//! - Mouse Click: Toggle checkbox
//! - Esc: Quit
//!
//! Run with: cargo run --example checkbox

use tui::{prelude::*, widget::common::StatefulWidgetBuilder};

/// Application state
#[derive(Debug)]
struct State {
    /// Individual checkboxes
    remember_me: bool,
    subscribe: bool,
    /// Feature toggles (vertical group)
    features: Vec<bool>,
    features_focus: usize,
    /// Quick options (horizontal group)
    quick_options: Vec<bool>,
    quick_options_focus: usize,
}

/// Messages from UI interactions
#[derive(Clone, Debug)]
enum Message {
    RememberMeToggled(bool),
    SubscribeToggled(bool),
    FeaturesChanged(usize, bool, Vec<bool>),
    QuickOptionsChanged(usize, bool, Vec<bool>),
    Quit,
}

/// Update function - handles messages
fn update(state: &mut State, msg: Message) {
    match msg {
        Message::RememberMeToggled(checked) => {
            state.remember_me = checked;
        }
        Message::SubscribeToggled(checked) => {
            state.subscribe = checked;
        }
        Message::FeaturesChanged(index, _checked, states) => {
            state.features = states;
            state.features_focus = index;
        }
        Message::QuickOptionsChanged(index, _checked, states) => {
            state.quick_options = states;
            state.quick_options_focus = index;
        }
        Message::Quit => {
            std::process::exit(0);
        }
    }
}

/// View function - builds the UI
fn view(state: &State) -> impl Layout<Message> {
    let feature_names = ["Experimental", "Debug Mode", "Telemetry"];
    let quick_option_names = ["A", "B", "C", "D"];

    col()
        .padding(Padding::new(3, 3, 2, 2))
        .gap(1)
        // Header
        .child(label("Checkbox Component Demo").fg(Color::Cyan).bold())
        .child(spacer().height(1))
        // Individual checkboxes
        .child(label("Individual Checkboxes:").fg(Color::Green).bold())
        .child(
            checkbox("Remember me")
                .checked(state.remember_me)
                .on_change(|checked| Message::RememberMeToggled(checked)),
        )
        .child(
            checkbox("Subscribe to newsletter")
                .checked(state.subscribe)
                .on_change(|checked| Message::SubscribeToggled(checked)),
        )
        .child(spacer().height(1))
        // Vertical checkbox group
        .child(label("Feature Toggles (Vertical - use Up/Down):").fg(Color::Green).bold())
        .child(
            checkbox_group(feature_names)
                .checked(state.features.clone())
                .focused_index(state.features_focus)
                .on_change(|index, checked, states| Message::FeaturesChanged(index, checked, states)),
        )
        .child(spacer().height(1))
        // Horizontal checkbox group
        .child(label("Quick Options (Horizontal - use Left/Right):").fg(Color::Green).bold())
        .child(
            checkbox_group(quick_option_names)
                .direction(CheckboxDirection::Horizontal)
                .checked(state.quick_options.clone())
                .focused_index(state.quick_options_focus)
                .on_change(|index, checked, states| Message::QuickOptionsChanged(index, checked, states)),
        )
        .child(spacer().height(1))
        // Disabled example
        .child(label("Disabled:").fg(Color::Indexed(8)))
        .child(checkbox("Disabled option").checked(false).disabled(true))
        .child(spacer().height(1))
        // Status display
        .child(
            label(format!(
                "Individual: Remember={} Subscribe={} | Features: {} | Quick: {}",
                if state.remember_me { "ON" } else { "OFF" },
                if state.subscribe { "ON" } else { "OFF" },
                state
                    .features
                    .iter()
                    .enumerate()
                    .filter(|(_, &checked)| checked)
                    .map(|(i, _)| feature_names[i])
                    .collect::<Vec<_>>()
                    .join(", "),
                state
                    .quick_options
                    .iter()
                    .enumerate()
                    .filter(|(_, &checked)| checked)
                    .map(|(i, _)| quick_option_names[i])
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
            .fg(Color::Yellow),
        )
        // Footer with instructions
        .child(spacer().height(1))
        .child(
            label("Tab: Next option | Up/Down or Left/Right: Navigate group | Enter/Space: Toggle | Esc: Quit")
                .fg(Color::Indexed(8)),
        )
}

fn main() -> WidgetResult<()> {
    let app = App::new(State {
        remember_me: false,
        subscribe: false,
        features: vec![false, false, false],
        features_focus: 0,
        quick_options: vec![false, false, false, false],
        quick_options_focus: 0,
    });
    app.on_key(KeyCode::Char('q'), || Message::Quit)
        .run_inline(update, view)?;
    Ok(())
}
