//! RadioGroup Component Example
//!
//! Demonstrates:
//! - Basic radio group functionality
//! - Focus navigation with Tab/Shift+Tab
//! - Option navigation with Up/Down arrows
//! - Selection with Enter/Space
//! - Mouse click to select
//! - Change event handling
//! - Visual focus indicators
//!
//! Controls:
//! - Tab: Focus next widget
//! - Shift+Tab: Focus previous widget
//! - Up/Down: Navigate options within focused radio group
//! - Enter/Space: Select focused option
//! - Mouse Click: Select option
//! - Esc: Quit
//!
//! Run with: cargo run --example radio

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct State {
    /// Selected size
    size: Option<usize>,
    /// Selected color
    color: Option<usize>,
    /// Selected delivery method
    delivery: Option<usize>,
}

/// Messages from UI interactions
#[derive(Clone, Debug)]
enum Message {
    SizeSelected(usize),
    ColorSelected(usize),
    DeliverySelected(usize),
    Quit,
}

/// Update function - handles messages
fn update(state: &mut State, msg: Message) {
    match msg {
        Message::SizeSelected(index) => {
            state.size = Some(index);
        }
        Message::ColorSelected(index) => {
            state.color = Some(index);
        }
        Message::DeliverySelected(index) => {
            state.delivery = Some(index);
        }
        Message::Quit => {
            std::process::exit(0);
        }
    }
}

/// View function - builds the UI
fn view(state: &State) -> Container<Message> {
    let sizes = ["Small", "Medium", "Large", "Extra Large"];
    let colors = ["Red", "Green", "Blue", "Yellow"];
    let delivery_methods = ["Standard (5-7 days)", "Express (2-3 days)", "Next Day"];

    col()
        .padding(Padding::new(3, 3, 2, 2))
        .gap(1)
        // Header
        .child(label("RadioGroup Component Demo").fg(Color::Cyan).bold())
        .child(label(""))
        // Size selection
        .child(label("Select Size:").fg(Color::Green).bold())
        .child({
            let mut rg = radio_group(sizes).on_change(|index| Message::SizeSelected(index));
            if let Some(selected) = state.size {
                rg = rg.selected(selected);
            }
            rg
        })
        .child(label(""))
        // Color selection
        .child(label("Select Color:").fg(Color::Green).bold())
        .child({
            let mut rg = radio_group(colors).on_change(|index| Message::ColorSelected(index));
            if let Some(selected) = state.color {
                rg = rg.selected(selected);
            }
            rg
        })
        .child(label(""))
        // Delivery method selection
        .child(label("Delivery Method:").fg(Color::Green).bold())
        .child({
            let mut rg =
                radio_group(delivery_methods).on_change(|index| Message::DeliverySelected(index));
            if let Some(selected) = state.delivery {
                rg = rg.selected(selected);
            }
            rg
        })
        .child(label(""))
        // Disabled example (should not be focusable)
        .child(label("Disabled:").fg(Color::Indexed(8)))
        .child(radio_group(["Option 1", "Option 2"]).disabled(true))
        .child(label(""))
        // Status display
        .child(
            label(format!(
                "Selection: Size={} Color={} Delivery={}",
                state
                    .size
                    .map(|i| sizes[i])
                    .unwrap_or("None"),
                state
                    .color
                    .map(|i| colors[i])
                    .unwrap_or("None"),
                state
                    .delivery
                    .map(|i| delivery_methods[i])
                    .unwrap_or("None")
            ))
            .fg(Color::Yellow),
        )
        // Footer with instructions
        .child(label(""))
        .child(label("Tab: Next | Up/Down: Navigate options | Enter/Space: Select | Esc: Quit").fg(Color::Indexed(8)))
        // Keyboard controller for global shortcuts
        .child(keyboard_controller().on('q', || Message::Quit))
}

fn main() -> Result<()> {
    let app = App::new(State {
        size: None,
        color: None,
        delivery: None,
    });
    app.run_inline(update, view)?;
    Ok(())
}
