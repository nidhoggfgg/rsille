//! RadioGroup Component Example
//!
//! Demonstrates:
//! - Basic radio group functionality
//! - Vertical and horizontal layout
//! - Option navigation with Tab/Shift+Tab (within group)
//! - Option navigation with Up/Down arrows (vertical) or Left/Right arrows (horizontal)
//! - Selection with Enter/Space
//! - Mouse click to select
//! - Change event handling
//! - Visual focus indicators
//!
//! Controls:
//! - Tab: Navigate to next option within group
//! - Shift+Tab: Navigate to previous option within group
//! - Up/Down: Navigate options within vertical radio group
//! - Left/Right: Navigate options within horizontal radio group
//! - Enter/Space: Select focused option
//! - Mouse Click: Select option
//! - Esc: Quit
//!
//! Run with: cargo run --example radio

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct State {
    /// Selected size (horizontal layout)
    size: Option<usize>,
    /// Selected color (vertical layout)
    color: Option<usize>,
    /// Selected priority (horizontal layout)
    priority: Option<usize>,
    /// Selected delivery method (vertical layout)
    delivery: Option<usize>,
}

/// Messages from UI interactions
#[derive(Clone, Debug)]
enum Message {
    SizeSelected(usize),
    ColorSelected(usize),
    PrioritySelected(usize),
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
        Message::PrioritySelected(index) => {
            state.priority = Some(index);
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
    let sizes = ["S", "M", "L", "XL"];
    let colors = ["Red", "Green", "Blue", "Yellow"];
    let priorities = ["Low", "Medium", "High"];
    let delivery_methods = ["Standard (5-7 days)", "Express (2-3 days)", "Next Day"];

    col()
        .padding(Padding::new(3, 3, 2, 2))
        .gap(1)
        // Header
        .child(label("RadioGroup Component Demo").fg(Color::Cyan).bold())
        .child(spacer().height(1))
        // Horizontal layout example
        .child(label("Size (Horizontal - use Left/Right):").fg(Color::Green).bold())
        .child({
            let mut rg = radio_group(sizes)
                .direction(RadioDirection::Horizontal)
                .on_change(|index| Message::SizeSelected(index));
            if let Some(selected) = state.size {
                rg = rg.selected(selected);
            }
            rg
        })
        .child(spacer().height(1))
        // Another horizontal layout example
        .child(label("Priority (Horizontal - use Left/Right):").fg(Color::Green).bold())
        .child({
            let mut rg = radio_group(priorities)
                .direction(RadioDirection::Horizontal)
                .on_change(|index| Message::PrioritySelected(index));
            if let Some(selected) = state.priority {
                rg = rg.selected(selected);
            }
            rg
        })
        .child(spacer().height(1))
        // Vertical layout example
        .child(label("Color (Vertical - use Up/Down):").fg(Color::Green).bold())
        .child({
            let mut rg = radio_group(colors).on_change(|index| Message::ColorSelected(index));
            if let Some(selected) = state.color {
                rg = rg.selected(selected);
            }
            rg
        })
        .child(spacer().height(1))
        // Another vertical layout example
        .child(label("Delivery (Vertical - use Up/Down):").fg(Color::Green).bold())
        .child({
            let mut rg =
                radio_group(delivery_methods).on_change(|index| Message::DeliverySelected(index));
            if let Some(selected) = state.delivery {
                rg = rg.selected(selected);
            }
            rg
        })
        .child(spacer().height(1))
        // Disabled example
        .child(label("Disabled:").fg(Color::Indexed(8)))
        .child(radio_group(["Option 1", "Option 2"]).disabled(true))
        .child(spacer().height(1))
        // Status display
        .child(
            label(format!(
                "Selection: Size={} Priority={} Color={} Delivery={}",
                state.size.map(|i| sizes[i]).unwrap_or("None"),
                state
                    .priority
                    .map(|i| priorities[i])
                    .unwrap_or("None"),
                state.color.map(|i| colors[i]).unwrap_or("None"),
                state
                    .delivery
                    .map(|i| delivery_methods[i])
                    .unwrap_or("None")
            ))
            .fg(Color::Yellow),
        )
        // Footer with instructions
        .child(spacer().height(1))
        .child(
            label("Tab: Next option | Up/Down or Left/Right: Navigate | Enter/Space: Select | Esc: Quit")
                .fg(Color::Indexed(8)),
        )
        // Keyboard controller for global shortcuts
        .child(keyboard_controller().on('q', || Message::Quit))
}

fn main() -> Result<()> {
    let app = App::new(State {
        size: None,
        color: None,
        priority: None,
        delivery: None,
    });
    app.run_inline(update, view)?;
    Ok(())
}
