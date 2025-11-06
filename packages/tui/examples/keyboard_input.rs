//! Keyboard Input Example
//!
//! Demonstrates keyboard input handling with different key types:
//! characters, function keys, arrow keys, and modifiers.
//!
//! Run with: cargo run --example keyboard_input

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct KeyboardState {
    last_key: String,
    key_history: Vec<String>,
    char_count: usize,
    special_count: usize,
}

/// Messages that can update the state
#[derive(Clone, Debug)]
enum Message {
    KeyPressed(String),
    Clear,
}

/// Update function - handles state changes
fn update(state: &mut KeyboardState, msg: Message) {
    match msg {
        Message::KeyPressed(key) => {
            state.last_key = key.clone();
            state.key_history.push(key.clone());

            // Keep only last 10 keys
            if state.key_history.len() > 10 {
                state.key_history.remove(0);
            }

            // Update counters
            if key.starts_with("Char(") {
                state.char_count += 1;
            } else {
                state.special_count += 1;
            }
        }
        Message::Clear => {
            state.last_key = "None".to_string();
            state.key_history.clear();
            state.char_count = 0;
            state.special_count = 0;
        }
    }
}

/// Helper to format key event
fn format_key(code: KeyCode) -> String {
    match code {
        KeyCode::Char(c) => {
            if c == ' ' {
                "Space".to_string()
            } else {
                format!("Char('{}')", c)
            }
        }
        KeyCode::F(n) => format!("F{}", n),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::BackTab => "BackTab".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Null => "Null".to_string(),
        // Handle other crossterm KeyCode variants
        _ => format!("{:?}", code),
    }
}

/// View function - renders the UI from current state
fn view(state: &KeyboardState) -> Container<Message> {
    // Build history display
    let mut history_widgets: Vec<AnyWidget<Message>> = state
        .key_history
        .iter()
        .enumerate()
        .map(|(i, key)| {
            Label::new(format!("  {}. {}", i + 1, key))
                .style(Style::default().fg(Color::Indexed(8)))
                .into()
        })
        .collect();

    if history_widgets.is_empty() {
        history_widgets.push(
            Label::new("  (No keys pressed yet)")
                .style(Style::default().fg(Color::Indexed(8)))
                .into(),
        );
    }

    let mut widgets = vec![
        // Title
        Label::new("⌨️  Keyboard Input Demo")
            .style(Style::default().fg(Color::Cyan).bold())
            .into(),
        Label::new("")
            .into(),
        // Last key
        Label::new("Last Key Pressed:")
            .style(Style::default().fg(Color::Yellow).bold())
            .into(),
        Label::new(format!("  {}", state.last_key))
            .style(Style::default().fg(Color::Green))
            .into(),
        Label::new("")
            .into(),
        // Statistics
        Label::new("Statistics:")
            .style(Style::default().fg(Color::Yellow).bold())
            .into(),
        Label::new(format!("  Character keys: {}", state.char_count))
            .style(Style::default().fg(Color::White))
            .into(),
        Label::new(format!("  Special keys: {}", state.special_count))
            .style(Style::default().fg(Color::White))
            .into(),
        Label::new(format!("  Total keys: {}", state.char_count + state.special_count))
            .style(Style::default().fg(Color::White))
            .into(),
        Label::new("")
            .into(),
        // History
        Label::new("Recent Key History (last 10):")
            .style(Style::default().fg(Color::Yellow).bold())
            .into(),
    ];

    widgets.extend(history_widgets);

    widgets.extend(vec![
        Label::new("")
            .into(),
        // Keyboard controller to capture all keys
        KeyboardController::new()
            .on_key(KeyCode::Char('c'), || Message::Clear)
            // Capture common keys for demonstration
            .on_key(KeyCode::Up, || Message::KeyPressed(format_key(KeyCode::Up)))
            .on_key(KeyCode::Down, || Message::KeyPressed(format_key(KeyCode::Down)))
            .on_key(KeyCode::Left, || Message::KeyPressed(format_key(KeyCode::Left)))
            .on_key(KeyCode::Right, || Message::KeyPressed(format_key(KeyCode::Right)))
            .on_key(KeyCode::Enter, || Message::KeyPressed(format_key(KeyCode::Enter)))
            .on_key(KeyCode::Backspace, || Message::KeyPressed(format_key(KeyCode::Backspace)))
            .on_key(KeyCode::Delete, || Message::KeyPressed(format_key(KeyCode::Delete)))
            .on_key(KeyCode::Home, || Message::KeyPressed(format_key(KeyCode::Home)))
            .on_key(KeyCode::End, || Message::KeyPressed(format_key(KeyCode::End)))
            .on_key(KeyCode::F(1), || Message::KeyPressed(format_key(KeyCode::F(1))))
            .on_key(KeyCode::F(2), || Message::KeyPressed(format_key(KeyCode::F(2))))
            .on_key(KeyCode::F(3), || Message::KeyPressed(format_key(KeyCode::F(3))))
            .on_key(KeyCode::F(4), || Message::KeyPressed(format_key(KeyCode::F(4))))
            // Letter keys
            .on_key(KeyCode::Char('a'), || Message::KeyPressed(format_key(KeyCode::Char('a'))))
            .on_key(KeyCode::Char('b'), || Message::KeyPressed(format_key(KeyCode::Char('b'))))
            .on_key(KeyCode::Char('d'), || Message::KeyPressed(format_key(KeyCode::Char('d'))))
            .on_key(KeyCode::Char('e'), || Message::KeyPressed(format_key(KeyCode::Char('e'))))
            .on_key(KeyCode::Char('f'), || Message::KeyPressed(format_key(KeyCode::Char('f'))))
            .on_key(KeyCode::Char('g'), || Message::KeyPressed(format_key(KeyCode::Char('g'))))
            .on_key(KeyCode::Char('h'), || Message::KeyPressed(format_key(KeyCode::Char('h'))))
            .on_key(KeyCode::Char('i'), || Message::KeyPressed(format_key(KeyCode::Char('i'))))
            .on_key(KeyCode::Char('j'), || Message::KeyPressed(format_key(KeyCode::Char('j'))))
            .on_key(KeyCode::Char('k'), || Message::KeyPressed(format_key(KeyCode::Char('k'))))
            .on_key(KeyCode::Char('l'), || Message::KeyPressed(format_key(KeyCode::Char('l'))))
            .on_key(KeyCode::Char('m'), || Message::KeyPressed(format_key(KeyCode::Char('m'))))
            .on_key(KeyCode::Char('n'), || Message::KeyPressed(format_key(KeyCode::Char('n'))))
            .on_key(KeyCode::Char('o'), || Message::KeyPressed(format_key(KeyCode::Char('o'))))
            .on_key(KeyCode::Char('p'), || Message::KeyPressed(format_key(KeyCode::Char('p'))))
            .on_key(KeyCode::Char('r'), || Message::KeyPressed(format_key(KeyCode::Char('r'))))
            .on_key(KeyCode::Char('s'), || Message::KeyPressed(format_key(KeyCode::Char('s'))))
            .on_key(KeyCode::Char('t'), || Message::KeyPressed(format_key(KeyCode::Char('t'))))
            .on_key(KeyCode::Char('u'), || Message::KeyPressed(format_key(KeyCode::Char('u'))))
            .on_key(KeyCode::Char('v'), || Message::KeyPressed(format_key(KeyCode::Char('v'))))
            .on_key(KeyCode::Char('w'), || Message::KeyPressed(format_key(KeyCode::Char('w'))))
            .on_key(KeyCode::Char('x'), || Message::KeyPressed(format_key(KeyCode::Char('x'))))
            .on_key(KeyCode::Char('y'), || Message::KeyPressed(format_key(KeyCode::Char('y'))))
            .on_key(KeyCode::Char('z'), || Message::KeyPressed(format_key(KeyCode::Char('z'))))
            // Numbers
            .on_key(KeyCode::Char('0'), || Message::KeyPressed(format_key(KeyCode::Char('0'))))
            .on_key(KeyCode::Char('1'), || Message::KeyPressed(format_key(KeyCode::Char('1'))))
            .on_key(KeyCode::Char('2'), || Message::KeyPressed(format_key(KeyCode::Char('2'))))
            .on_key(KeyCode::Char('3'), || Message::KeyPressed(format_key(KeyCode::Char('3'))))
            .on_key(KeyCode::Char('4'), || Message::KeyPressed(format_key(KeyCode::Char('4'))))
            .on_key(KeyCode::Char('5'), || Message::KeyPressed(format_key(KeyCode::Char('5'))))
            .on_key(KeyCode::Char('6'), || Message::KeyPressed(format_key(KeyCode::Char('6'))))
            .on_key(KeyCode::Char('7'), || Message::KeyPressed(format_key(KeyCode::Char('7'))))
            .on_key(KeyCode::Char('8'), || Message::KeyPressed(format_key(KeyCode::Char('8'))))
            .on_key(KeyCode::Char('9'), || Message::KeyPressed(format_key(KeyCode::Char('9'))))
            .on_key(KeyCode::Char(' '), || Message::KeyPressed(format_key(KeyCode::Char(' '))))
            .into(),
        // Instructions
        Label::new("Instructions:")
            .style(Style::default().fg(Color::Yellow))
            .into(),
        Label::new("  Press any key to see it captured and displayed above")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
        Label::new("  c: clear history | Esc/q: quit")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
        Label::new("")
            .into(),
        Label::new("Note: Try letter keys, numbers, arrows, function keys, etc.")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
    ]);

    Container::vertical(widgets)
        .gap(0)
        .padding(Padding::new(2, 2, 1, 1))
}

fn main() -> Result<()> {
    let app = App::new(KeyboardState {
        last_key: "None".to_string(),
        key_history: Vec::new(),
        char_count: 0,
        special_count: 0,
    });
    app.run(update, view)?;
    Ok(())
}
