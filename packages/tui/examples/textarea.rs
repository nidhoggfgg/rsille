//! Textarea Component Example
//!
//! Demonstrates:
//! - Multi-line text input (Textarea)
//! - Different textarea variants (Default, Borderless)
//! - Line wrapping support
//! - Vertical scrolling
//! - Optional line numbers
//! - Cursor positioning across lines
//! - Text selection support (Ctrl+A)
//! - Focus management
//!
//! Controls:
//! - Type: Enter text
//! - Enter: New line
//! - Ctrl+Enter: Submit textarea
//! - Backspace/Delete: Remove characters
//! - Arrow keys: Move cursor (Up/Down/Left/Right)
//! - Home/End: Jump to start/end of line
//! - Ctrl+A: Select all text
//! - Tab: Focus next textarea
//! - Shift+Tab: Focus previous textarea
//! - Esc: Quit
//!
//! Run with: cargo run --example textarea

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct State {
    /// Code editor value
    code: String,
    /// Comments value
    comments: String,
    /// Notes value
    notes: String,
    /// Submission status
    submitted: Option<String>,
}

/// Messages from UI interactions
#[derive(Clone, Debug)]
enum Message {
    CodeChanged(String),
    CommentsChanged(String),
    NotesChanged(String),
    Submit,
    Quit,
}

/// Update function - handles messages
fn update(state: &mut State, msg: Message) {
    match msg {
        Message::CodeChanged(value) => {
            state.code = value;
        }
        Message::CommentsChanged(value) => {
            state.comments = value;
        }
        Message::NotesChanged(value) => {
            state.notes = value;
        }
        Message::Submit => {
            let lines = state.code.lines().count();
            let chars = state.code.len();
            state.submitted = Some(format!("Submitted: {} lines, {} chars", lines, chars));
        }
        Message::Quit => {
            std::process::exit(0);
        }
    }
}

/// View function - builds the UI
fn view(state: &State) -> impl Layout<Message> {
    col()
        .padding(Padding::new(3, 3, 2, 2))
        .gap(1)
        // Header
        .child(label("Textarea Component Demo").fg(Color::Cyan).bold())
        .child(spacer().height(1))
        // Code editor with line numbers
        .child(label("Code Editor (with line numbers):").fg(Color::Indexed(8)))
        .child(
            textarea()
                .variant(TextareaVariant::Default)
                .placeholder("fn main() {\n    // Your code here...\n}")
                .value(state.code.clone())
                .line_numbers(true)
                .line_wrap(true)
                .on_change(|text| Message::CodeChanged(text))
                .on_submit(|_| Message::Submit),
        )
        .child(spacer().height(1))
        // Comments textarea
        .child(label("Comments:").fg(Color::Indexed(8)))
        .child(
            textarea()
                .variant(TextareaVariant::Default)
                .placeholder("Enter your comments here...")
                .value(state.comments.clone())
                .line_numbers(false)
                .line_wrap(true)
                .on_change(|text| Message::CommentsChanged(text)),
        )
        .child(spacer().height(1))
        // Borderless variant for notes
        .child(label("Notes (Borderless):").fg(Color::Indexed(8)))
        .child(
            textarea()
                .variant(TextareaVariant::Borderless)
                .placeholder("Quick notes...")
                .value(state.notes.clone())
                .line_numbers(false)
                .line_wrap(true)
                .on_change(|text| Message::NotesChanged(text)),
        )
        .child(spacer().height(1))
        // Submit button
        .child(
            button("Submit (or Ctrl+Enter)")
                .variant(ButtonVariant::Primary)
                .on_click(|| Message::Submit),
        )
        // Status display
        .child(if let Some(ref status) = state.submitted {
            label(status).fg(Color::Green)
        } else {
            label("Fill in the textareas and press Ctrl+Enter or click Submit").fg(Color::Indexed(8))
        })
        // Footer with instructions
        .child(
            label("Enter for newline | Ctrl+Enter to submit | Arrows to navigate | Ctrl+A to select all | Esc to quit")
                .fg(Color::Indexed(8)),
        )
}

fn main() -> WidgetResult<()> {
    let app = App::new(State {
        code: String::new(),
        comments: String::new(),
        notes: String::new(),
        submitted: None,
    });
    app.on_key(KeyCode::Esc, || Message::Quit)
        .on_key(KeyCode::Char('q'), || Message::Quit)
        .run_inline(update, view)?;
    Ok(())
}
