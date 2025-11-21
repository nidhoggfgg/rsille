//! Checkbox Component Example
//!
//! Demonstrates:
//! - Basic checkbox functionality
//! - Focus navigation with Tab/Shift+Tab
//! - Keyboard toggle with Enter/Space
//! - Mouse click to toggle
//! - Change event handling
//! - Visual focus indicators
//!
//! Controls:
//! - Tab: Focus next checkbox
//! - Shift+Tab: Focus previous checkbox
//! - Enter/Space: Toggle focused checkbox
//! - Mouse Click: Toggle checkbox
//! - Esc: Quit
//!
//! Run with: cargo run --example checkbox

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct State {
    /// Feature flags
    feature_a: bool,
    feature_b: bool,
    feature_c: bool,
    /// Options
    remember_me: bool,
    subscribe: bool,
}

/// Messages from UI interactions
#[derive(Clone, Debug)]
enum Message {
    FeatureAToggled(bool),
    FeatureBToggled(bool),
    FeatureCToggled(bool),
    RememberMeToggled(bool),
    SubscribeToggled(bool),
    Quit,
}

/// Update function - handles messages
fn update(state: &mut State, msg: Message) {
    match msg {
        Message::FeatureAToggled(checked) => {
            state.feature_a = checked;
        }
        Message::FeatureBToggled(checked) => {
            state.feature_b = checked;
        }
        Message::FeatureCToggled(checked) => {
            state.feature_c = checked;
        }
        Message::RememberMeToggled(checked) => {
            state.remember_me = checked;
        }
        Message::SubscribeToggled(checked) => {
            state.subscribe = checked;
        }
        Message::Quit => {
            std::process::exit(0);
        }
    }
}

/// View function - builds the UI
fn view(state: &State) -> Container<Message> {
    col()
        .padding(Padding::new(3, 3, 2, 2))
        .gap(1)
        // Header
        .child(label("Checkbox Component Demo").fg(Color::Cyan).bold())
        .child(label(""))
        // Feature toggles section
        .child(label("Features:").fg(Color::Green).bold())
        .child(
            checkbox("Enable experimental features")
                .checked(state.feature_a)
                .on_change(|checked| Message::FeatureAToggled(checked)),
        )
        .child(
            checkbox("Enable debug mode")
                .checked(state.feature_b)
                .on_change(|checked| Message::FeatureBToggled(checked)),
        )
        .child(
            checkbox("Enable telemetry")
                .checked(state.feature_c)
                .on_change(|checked| Message::FeatureCToggled(checked)),
        )
        .child(label(""))
        // Options section
        .child(label("Options:").fg(Color::Green).bold())
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
        .child(label(""))
        // Disabled example (should not be focusable)
        .child(label("Disabled:").fg(Color::Indexed(8)))
        .child(checkbox("Disabled option").checked(false).disabled(true))
        .child(label(""))
        // Status display
        .child(
            label(format!(
                "Status: Experimental={} Debug={} Telemetry={} Remember={} Subscribe={}",
                if state.feature_a { "ON" } else { "OFF" },
                if state.feature_b { "ON" } else { "OFF" },
                if state.feature_c { "ON" } else { "OFF" },
                if state.remember_me { "ON" } else { "OFF" },
                if state.subscribe { "ON" } else { "OFF" }
            ))
            .fg(Color::Yellow),
        )
        // Footer with instructions
        .child(label(""))
        .child(label("Tab/Shift+Tab: Navigate | Enter/Space: Toggle | Esc: Quit").fg(Color::Indexed(8)))
        // Keyboard controller for global shortcuts
        .child(keyboard_controller().on('q', || Message::Quit))
}

fn main() -> Result<()> {
    let app = App::new(State {
        feature_a: false,
        feature_b: false,
        feature_c: false,
        remember_me: false,
        subscribe: false,
    });
    app.run_inline(update, view)?;
    Ok(())
}
