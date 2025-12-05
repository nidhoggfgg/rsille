//! Enhanced Wrapper Demo
//!
//! Demonstrates the Enhanced wrapper for adding multiple capabilities
//! to any widget through a single, non-nested API.
//!
//! Features shown:
//! - Click events
//! - Hover effects
//! - Keyboard focus (Tab navigation)
//! - State-aware styling
//! - Borders with automatic focus rings
//! - Padding
//! - Combining all features
//!
//! Controls:
//! - Click: Trigger click events
//! - Hover: See visual feedback
//! - Tab/Shift+Tab: Navigate focus
//! - Enter/Space: Activate focused element
//! - q/Esc: Quit

use tui::prelude::*;

/// Application state
struct State {
    last_event: String,
    click_count: usize,
    mouse_enter_count: usize,
    mouse_leave_count: usize,
    focus_count: usize,
}

/// Messages from UI interactions
#[derive(Clone, Debug)]
enum Message {
    SimpleClicked,
    MouseEntered,
    MouseLeft,
    BorderedFocused,
    Card1Clicked,
    Card2Clicked,
    Card3Clicked,
    FocusGained(String),
    FocusLost,
    Quit,
}

/// Update function - handles messages
fn update(state: &mut State, msg: Message) {
    match msg {
        Message::SimpleClicked => {
            state.last_event = "Simple Click".into();
            state.click_count += 1;
        }
        Message::MouseEntered => {
            state.last_event = "Mouse Entered".into();
            state.mouse_enter_count += 1;
        }
        Message::MouseLeft => {
            state.last_event = "Mouse Left".into();
            state.mouse_leave_count += 1;
        }
        Message::FocusGained(name) => {
            state.last_event = format!("{} Focused", name);
            state.focus_count += 1;
        }
        Message::FocusLost => {
            // No-op for now
        }
        Message::BorderedFocused => {
            state.last_event = "Bordered Focused".into();
            state.focus_count += 1;
        }
        Message::Card1Clicked => {
            state.last_event = "Card 1 Clicked".into();
            state.click_count += 1;
        }
        Message::Card2Clicked => {
            state.last_event = "Card 2 Clicked".into();
            state.click_count += 1;
        }
        Message::Card3Clicked => {
            state.last_event = "Card 3 Clicked".into();
            state.click_count += 1;
        }
        Message::Quit => {}
    }
}

/// View function - builds UI from state
fn view(state: &State) -> Flex<Message> {
    col()
        .gap(2)
        .child(
            // Header
            label("Enhanced Wrapper Demo").bold().fg(Color::Cyan),
        )
        .child(
            // Section 1: Basic Features
            col()
                .gap(1)
                .child(label("=== Basic Features ===").bold())
                .child(enhanced(label("1. Click me!")).on_click(|| Message::SimpleClicked))
                .child(
                    enhanced(label("2. Move mouse over me"))
                        .hoverable()
                        .hover_fg(Color::Cyan)
                        .on_mouse_enter(|| Message::MouseEntered)
                        .on_mouse_leave(|| Message::MouseLeft),
                )
                .child(
                    enhanced(label("3. Press Tab to focus me"))
                        .focusable()
                        .focus_fg(Color::Yellow)
                        .on_focus(|| Message::FocusGained("Label".into())),
                ),
        )
        .child(
            // Section 2: Styled Components
            col()
                .gap(1)
                .child(label("=== Styled Components ===").bold())
                .child(
                    enhanced(label("Bordered with focus ring"))
                        .focusable()
                        .bordered(BorderStyle::Single)
                        .padding(Padding::uniform(1))
                        .on_focus(|| Message::BorderedFocused),
                )
                .child(
                    enhanced(label("Rounded border + padding"))
                        .bordered(BorderStyle::Rounded)
                        .padding(Padding::new(1, 2, 1, 2))
                        .fg(Color::Green),
                ),
        )
        .child(
            // Section 3: Interactive Cards
            col()
                .gap(1)
                .child(label("=== Interactive Cards ===").bold())
                .child(
                    row()
                        .gap(2)
                        .child(create_card("Card 1", Message::Card1Clicked))
                        .child(create_card("Card 2", Message::Card2Clicked))
                        .child(create_card("Card 3", Message::Card3Clicked)),
                ),
        )
        .child(
            // Stats
            col()
                .gap(0)
                .child(label("=== Event Stats ===").bold())
                .child(label(&format!("Last Event: {}", state.last_event)))
                .child(label(&format!("Clicks: {}", state.click_count)))
                .child(label(&format!("Mouse Enters: {}", state.mouse_enter_count)))
                .child(label(&format!("Mouse Leaves: {}", state.mouse_leave_count)))
                .child(label(&format!("Focus Events: {}", state.focus_count))),
        )
        .child(label("Press 'q' or Esc to quit").fg(Color::White))
}

/// Helper function to create an interactive card
fn create_card(title: &str, on_click_msg: Message) -> Enhanced<Message, Flex<Message>> {
    let title_owned = title.to_string();

    enhanced(
        col()
            .gap(1)
            .child(label(title).bold())
            .child(label("Click or Tab"))
            .child(label("to interact")),
    )
    .focusable()
    .hoverable()
    .bordered(BorderStyle::Rounded)
    .padding(Padding::uniform(1))
    .fg(Color::White)
    .hover_fg(Color::Cyan)
    .focus_fg(Color::Yellow)
    .focus_border_color(Color::Cyan)
    .on_click(move || on_click_msg.clone())
    .on_focus(move || Message::FocusGained(title_owned.clone()))
}

fn main() -> WidgetResult<()> {
    let initial_state = State {
        last_event: "None".to_string(),
        click_count: 0,
        mouse_enter_count: 0,
        mouse_leave_count: 0,
        focus_count: 0,
    };

    App::new(initial_state)
        .on_key(KeyCode::Char('q'), || Message::Quit)
        .on_key(KeyCode::Esc, || Message::Quit)
        .run(update, view)
}
