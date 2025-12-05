//! Modal/Dialog Component Example
//!
//! Demonstrates:
//! - Alert modals with single action button
//! - Confirm modals with OK/Cancel buttons
//! - Custom modals with flexible content
//! - Modal size variants (Small, Medium, Large)
//! - Keyboard navigation (Tab, Enter, Esc)
//! - Focus trapping within modal
//! - Modal overlays on top of existing content
//! - Main page content remains visible behind modal
//!
//! Controls:
//! - Click buttons to show different modal types
//! - Tab/Shift+Tab: Navigate between buttons in modal
//! - Enter/Space: Activate focused button
//! - Esc: Close modal (always available)
//! - Esc (on main screen): Quit
//!
//! Run with: cargo run --example modal

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct State {
    /// Currently displayed modal (if any)
    active_modal: Option<DialogType>,
    /// Action log
    action_log: Vec<String>,
}

/// Types of modals available in the demo
#[derive(Debug, Clone, PartialEq)]
enum DialogType {
    Alert,
    Confirm,
    Custom,
    LargeModal,
}

/// Messages from UI interactions
#[derive(Clone, Debug)]
enum Message {
    /// Show a specific modal
    ShowModal(DialogType),
    /// Modal was confirmed
    ModalConfirmed,
    /// Modal was cancelled
    ModalCancelled,
    /// Modal was closed
    ModalClosed,
    /// Custom action from custom modal
    CustomAction(String),
    /// Quit application
    Quit,
}

impl From<DialogMessage> for Message {
    fn from(msg: DialogMessage) -> Self {
        match msg {
            DialogMessage::Confirmed => Message::ModalConfirmed,
            DialogMessage::Cancelled => Message::ModalCancelled,
            DialogMessage::Closed => Message::ModalClosed,
        }
    }
}

/// Update function - handles messages
fn update(state: &mut State, msg: Message) {
    match msg {
        Message::ShowModal(modal_type) => {
            state.active_modal = Some(modal_type.clone());
            state
                .action_log
                .push(format!("Opened {:?} modal", modal_type));
        }
        Message::ModalConfirmed => {
            state.action_log.push("Modal confirmed".to_string());
            state.active_modal = None;
        }
        Message::ModalCancelled => {
            state.action_log.push("Modal cancelled".to_string());
            state.active_modal = None;
        }
        Message::ModalClosed => {
            state.action_log.push("Modal closed (ESC)".to_string());
            state.active_modal = None;
        }
        Message::CustomAction(action) => {
            state.action_log.push(format!("Custom action: {}", action));
            state.active_modal = None;
        }
        Message::Quit => {
            std::process::exit(0);
        }
    }
}

/// View function - builds the UI
fn view(state: &State) -> Box<dyn Layout<Message>> {
    // Build main content
    let main_content = col()
        .padding(Padding::new(3, 3, 2, 2))
        .gap(1)
        // Header
        .child(label("Modal/Dialog Component Demo").fg(Color::Cyan).bold())
        .child(spacer().height(1))
        // Instructions
        .child(label("Click buttons to show different modal types:").fg(Color::Indexed(8)))
        .child(spacer().height(1))
        // Modal type buttons
        .child(
            row()
                .gap(2)
                .child(
                    button("Alert Modal")
                        .variant(ButtonVariant::Primary)
                        .on_click(|| Message::ShowModal(DialogType::Alert)),
                )
                .child(
                    button("Confirm Modal")
                        .variant(ButtonVariant::Secondary)
                        .on_click(|| Message::ShowModal(DialogType::Confirm)),
                )
                .child(
                    button("Custom Modal")
                        .variant(ButtonVariant::Ghost)
                        .on_click(|| Message::ShowModal(DialogType::Custom)),
                ),
        )
        .child(spacer().height(1))
        .child(
            row().gap(2).child(
                button("Large Modal")
                    .variant(ButtonVariant::Link)
                    .on_click(|| Message::ShowModal(DialogType::LargeModal)),
            ),
        )
        .child(spacer().height(2))
        // Action log
        .child(label("Action Log:").fg(Color::Yellow).bold())
        .child(
            col().gap(0).children(
                state
                    .action_log
                    .iter()
                    .rev()
                    .take(5)
                    .map(|log| label(format!("  • {}", log)).fg(Color::Green).into_widget())
                    .collect::<Vec<Box<dyn Widget<Message>>>>(),
            ),
        )
        .child(spacer().height(2))
        .child(label("Press ESC to quit").fg(Color::Indexed(8)).italic());

    // If there's an active modal, show modal on top of main content
    if let Some(ref modal_type) = state.active_modal {
        match modal_type {
            DialogType::Alert => {
                // Show alert modal overlaying main content
                Box::new(
                    alert(
                        main_content,
                        "Operation Successful",
                        "Your changes have been saved successfully!",
                    )
                    .size(DialogSize::Small),
                )
            }
            DialogType::Confirm => {
                // Show confirm modal overlaying main content
                Box::new(
                    confirm(
                        main_content,
                        "Delete File?",
                        "This action cannot be undone. Are you sure?",
                    )
                    .size(DialogSize::Medium),
                )
            }
            DialogType::Custom => {
                // Show custom modal overlaying main content
                let custom_content = col()
                    .gap(2)
                    .padding(Padding::new(2, 2, 2, 2))
                    .border(BorderStyle::Rounded)
                    .style(ThemeManager::global().with_theme(|theme| {
                        Style::default()
                            .bg(theme.colors.surface)
                            .fg(theme.colors.text)
                    }))
                    .children(vec![
                        label("Unsaved Changes")
                            .bold()
                            .fg(Color::Yellow)
                            .into_widget(),
                        label("You have unsaved changes in your document.").into_widget(),
                        label("What would you like to do?").into_widget(),
                        row()
                            .gap(2)
                            .justify_content(JustifyContent::Center)
                            .children(vec![
                                button("Save")
                                    .variant(ButtonVariant::Primary)
                                    .on_click(|| Message::CustomAction("Save".to_string()))
                                    .into_widget(),
                                button("Don't Save")
                                    .variant(ButtonVariant::Destructive)
                                    .on_click(|| Message::CustomAction("Don't Save".to_string()))
                                    .into_widget(),
                                button("Cancel")
                                    .variant(ButtonVariant::Secondary)
                                    .on_click(|| Message::ModalCancelled)
                                    .into_widget(),
                            ])
                            .into_widget(),
                    ]);

                Box::new(
                    Dialog::new(main_content, custom_content)
                        .size(DialogSize::Medium)
                        .on_close(|| Message::ModalClosed),
                )
            }
            DialogType::LargeModal => {
                // Show large modal overlaying main content
                let large_content = col()
                    .gap(2)
                    .padding(Padding::new(2, 2, 2, 2))
                    .border(BorderStyle::Rounded)
                    .style(ThemeManager::global().with_theme(|theme| {
                        Style::default()
                            .bg(theme.colors.surface)
                            .fg(theme.colors.text)
                    }))
                    .children(vec![
                        label("Large Modal Example")
                            .bold()
                            .fg(Color::Cyan)
                            .into_widget(),
                        spacer().height(1).into_widget(),
                        label("This is a large modal with more content.").into_widget(),
                        label("It overlays on top of the main page content.").into_widget(),
                        label("The background content remains visible.").into_widget(),
                        spacer().height(1).into_widget(),
                        label("Modal Features:")
                            .bold()
                            .fg(Color::Yellow)
                            .into_widget(),
                        label("  • Centered positioning").into_widget(),
                        label("  • Overlays existing content").into_widget(),
                        label("  • Focus trap (Tab/Shift+Tab stay inside)").into_widget(),
                        label("  • ESC key to close").into_widget(),
                        label("  • Theme integration").into_widget(),
                        label("  • Easy to use API").into_widget(),
                        spacer().height(1).into_widget(),
                        row()
                            .gap(2)
                            .justify_content(JustifyContent::Center)
                            .children(vec![button("Close")
                                .variant(ButtonVariant::Primary)
                                .on_click(|| Message::ModalClosed)
                                .into_widget()])
                            .into_widget(),
                    ]);

                Box::new(
                    Dialog::new(main_content, large_content)
                        .size(DialogSize::Large)
                        .on_close(|| Message::ModalClosed),
                )
            }
        }
    } else {
        // No modal, show main content directly
        Box::new(main_content)
    }
}

fn main() -> WidgetResult<()> {
    let initial_state = State {
        active_modal: None,
        action_log: vec!["Application started".to_string()],
    };

    App::new(initial_state)
        .on_key(KeyCode::Esc, || Message::Quit)
        .run(update, view)
}
