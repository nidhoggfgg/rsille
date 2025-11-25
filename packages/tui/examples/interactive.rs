//! Interactive Widget Example
//!
//! Demonstrates the Interactive wrapper for adding mouse event handling
//! to any widget.
//!
//! This example shows:
//! - Making labels clickable
//! - Hover events
//! - Wrapping complex widgets (containers)
//! - Press and release events
//! - Combining multiple event handlers
//!
//! Controls:
//! - Click on any interactive element to trigger events
//! - Hover over elements to see hover effects
//! - Press and hold to see press/release events
//! - Esc or 'q': Quit
//!
//! Note: This example uses fullscreen mode (run()) to enable mouse support.
//! Mouse events are NOT supported in inline mode (run_inline()) because
//! terminal scrolling would break coordinate tracking.
//!
//! Run with: cargo run --example interactive

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct State {
    last_event: String,
    click_count: usize,
    hover_count: usize,
    press_count: usize,
}

/// Messages from UI interactions
#[derive(Clone, Debug)]
enum Message {
    LabelClicked,
    LabelHovered,
    BoxClicked,
    ButtonPressed,
    ButtonReleased,
    Card1Clicked,
    Card2Clicked,
    Card3Clicked,
    Quit,
}

/// Update function - handles messages
fn update(state: &mut State, msg: Message) {
    match msg {
        Message::LabelClicked => {
            state.last_event = "Label Clicked".to_string();
            state.click_count += 1;
        }
        Message::LabelHovered => {
            state.last_event = "Label Hovered".to_string();
            state.hover_count += 1;
        }
        Message::BoxClicked => {
            state.last_event = "Box Clicked".to_string();
            state.click_count += 1;
        }
        Message::ButtonPressed => {
            state.last_event = "Button Pressed".to_string();
            state.press_count += 1;
        }
        Message::ButtonReleased => {
            state.last_event = "Button Released".to_string();
        }
        Message::Card1Clicked => {
            state.last_event = "Card 1 Clicked".to_string();
            state.click_count += 1;
        }
        Message::Card2Clicked => {
            state.last_event = "Card 2 Clicked".to_string();
            state.click_count += 1;
        }
        Message::Card3Clicked => {
            state.last_event = "Card 3 Clicked".to_string();
            state.click_count += 1;
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
        .child(label("Interactive Widget Example").fg(Color::Cyan).bold())
        .child(spacer().height(1))
        // Status display
        .child(
            label(format!(
                "Last Event: {} | Clicks: {} | Hovers: {} | Presses: {}",
                state.last_event, state.click_count, state.hover_count, state.press_count
            ))
            .fg(Color::Green),
        )
        .child(spacer().height(1))
        .child(label("Examples:").fg(Color::Yellow).bold())
        .child(spacer().height(1))
        // Example 1: Clickable label with hover
        .child(
            row().gap(2).child(label("1.").fg(Color::Indexed(8))).child(
                interactive(label("Clickable Label (hover to see)").fg(Color::Blue))
                    .on_click(|| Message::LabelClicked)
                    .on_hover(|| Message::LabelHovered),
            ),
        )
        .child(spacer().height(1))
        // Example 2: Clickable box with border
        .child(
            row().gap(2).child(label("2.").fg(Color::Indexed(8))).child(
                interactive(
                    col()
                        .border(BorderStyle::Single)
                        .padding(Padding::uniform(1))
                        .child(label("Entire box is clickable"))
                        .child(label("Click anywhere inside"))
                        .style(Style::default().bg(Color::Indexed(236))),
                )
                .on_click(|| Message::BoxClicked),
            ),
        )
        .child(spacer().height(1))
        // Example 3: Press and release events
        .child(
            row().gap(2).child(label("3.").fg(Color::Indexed(8))).child(
                interactive(
                    label("Press and Release Demo")
                        .fg(Color::Magenta)
                        .style(Style::default().bg(Color::Indexed(237))),
                )
                .on_press(|| Message::ButtonPressed)
                .on_release(|| Message::ButtonReleased),
            ),
        )
        .child(spacer().height(1))
        // Example 4: Multiple clickable cards in a row
        .child(label("4. Click on cards:").fg(Color::Indexed(8)))
        .child(
            row()
                .gap(2)
                .child(
                    interactive(
                        col()
                            .border(BorderStyle::Single)
                            .padding(Padding::uniform(1))
                            .child(label("Card 1"))
                            .child(label("Click me"))
                            .style(Style::default().bg(Color::Indexed(17))),
                    )
                    .on_click(|| Message::Card1Clicked),
                )
                .child(
                    interactive(
                        col()
                            .border(BorderStyle::Single)
                            .padding(Padding::uniform(1))
                            .child(label("Card 2"))
                            .child(label("Or me"))
                            .style(Style::default().bg(Color::Indexed(53))),
                    )
                    .on_click(|| Message::Card2Clicked),
                )
                .child(
                    interactive(
                        col()
                            .border(BorderStyle::Single)
                            .padding(Padding::uniform(1))
                            .child(label("Card 3"))
                            .child(label("Or me!"))
                            .style(Style::default().bg(Color::Indexed(89))),
                    )
                    .on_click(|| Message::Card3Clicked),
                ),
        )
        .child(spacer().height(1))
        // Footer with instructions
        .child(label("Click or hover on any element above to trigger events").fg(Color::Indexed(8)))
        .child(label("Esc or 'q': Quit").fg(Color::Indexed(8)))
        // Keyboard controller for global shortcuts
        .child(keyboard_controller().on('q', || Message::Quit))
}

fn main() -> Result<()> {
    let app = App::new(State {
        last_event: "None".to_string(),
        click_count: 0,
        hover_count: 0,
        press_count: 0,
    });
    // Use run() for fullscreen mode with mouse support
    app.run(update, view)?;
    Ok(())
}
